use chrono::{DateTime, Utc};
use config::Config;
use data::Data;
use reqwest::{Client, StatusCode};
use screenshots::Screen;
use std::{boxed::Box, error::Error, path::Path, thread::sleep};
use tokio::process::{Child, Command};
use tokio::time::{self, Duration};
use tokio::io::AsyncWriteExt;
use util::{Apikey, Updated, Video};

mod config;
mod data;
mod util;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    capture_screenshot()?;

    let mut config = Config::new();
    let mut data = Data::new();
    let client = Client::new();

    // Load the configs
    println!("Loading configuration...");
    config.load().await?;
    println!("Loading data...");
    data.load().await?;

    let _ = wait_for_api(&client, &config).await;
    
    // Get our api key
    if config.key.is_none() {
        config.key = Some(get_new_key(&client, &config).await?.key);
    }
    config.write().await?;

    // Get the videos if we've never updated
    if data.last_update.is_none() {
        let updated = sync(&client, &config).await?;
        update_videos(&client, &mut config, &mut data, updated).await?;
        println!("Data Updated: {:?}", updated);    
    }

    let mut interval = time::interval(Duration::from_secs(30));
    let mut mpv = start_mpv().await?;
    loop {
        interval.tick().await;

        let updated = sync(&client, &config).await?;
        if let (Some(updated), Some(last_update)) = (updated, data.last_update) {
            println!("Updated: {:?}", updated);
            println!("Data last updated: {:?}", last_update);
            if updated > last_update {
                update_videos(&client, &mut config, &mut data, Some(updated)).await?;
                mpv.kill().await?;
            }
        } else if updated.is_some() {
            // Handle the case where `data.last_update` is None and `updated` is Some.
            println!("Updated: {:?}", updated);
            println!("Data last updated: None");
            update_videos(&client, &mut config, &mut data, updated).await?;
            mpv.kill().await?;
        } else {
            // Handle the case where both `updated` and `data.last_update` are None, if necessary.
            println!("No updates available.");
        }

        // Restart mpv if it exits
        match mpv.try_wait() {
            Ok(Some(_)) => mpv = start_mpv().await?,
            Ok(None) => (),
            Err(error) => eprintln!("Error waiting for mpv process: {error}"),
        }
    }
}

/// Loops until we get a response from the API to make sure our network is online
async fn wait_for_api(client: &Client, config: &Config) -> Result<bool, Box<dyn Error>> {
    let mut interval = time::interval(Duration::from_secs(1));
    loop {
        let res = client.get(format!("{}/health", config.url)).send().await;
        if let Ok(response) = res {
            match response.status() {
                StatusCode::OK => break,
                StatusCode::INTERNAL_SERVER_ERROR => {
                    println!("Server error. Retrying in 2 minutes...");
                    time::interval(Duration::from_secs(120)).tick().await;
                }
                _ => (),
            }
        }
        interval.tick().await;
    }
    Ok(true)
}

/// Starts the mpv player with the proper playlist and flags
async fn start_mpv() -> Result<Child, Box<dyn Error>> {
    let image_display_duration = 10;
    let child = Command::new("mpv")
        .arg("--loop-playlist=inf")
        .arg("--volume=-1")
        .arg("--no-terminal")
        .arg(format!("--playlist={}/.local/share/signage/playlist.txt", std::env::var("HOME")?))
        .spawn()?;

    Ok(child)
}

/// Makes the proper request to receive an API key
async fn get_new_key(client: &Client, config: &mut Config) -> Result<Apikey, Box<dyn Error>> {
    println!("Requesting a new API key...");
    let res: Apikey = client
        .get(format!("{}/get-new-key/{}", config.url, config.id))
        .basic_auth(&config.username, Some(&config.password))
        .send()
        .await?
        .json()
        .await?;

    println!("Received new API key: {}", res.key);
    config.key = Some(res.key.clone());
    config.write().await?;
    Ok(res)
}

/// Makes the proper request to receive the last time the connected playlist was updated
async fn sync(client: &Client, config: &Config) -> Result<Option<DateTime<Utc>>, Box<dyn Error>> {
    let res: Updated = client
        .get(format!("{}/sync/{}", config.url, config.id))
        .header("APIKEY", config.key.clone().unwrap_or_default())
        .send()
        .await?
        .json()
        .await?;
    println!("Last updated: {:?}", res);
    Ok(res.updated)
}

/// Makes the proper request to receive the list of videos
async fn receive_videos(client: &Client, config: &mut Config) -> Result<Vec<Video>, Box<dyn Error>> {
    let url = format!("{}/recieve-videos/{}", config.url, config.id);
    let standard_api_key = config.key.clone().unwrap_or_default();

    // Request a new authorization token
    let new_key = get_new_key(client, config).await?;
    let auth_token = new_key.key;
    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .header("Cache-Control", "no-cache")
        .header("Accept-Encoding", "gzip, deflate, br")
        .header("Connection", "keep-alive")
        .header("APIKEY", auth_token)
        .send()
        .await?;

    let status = response.status();
    let text = response.text().await?;

    if status.is_success() {
        let res: Vec<Video> = serde_json::from_str(&text)?;
        Ok(res)
    } else {
        Err(format!("Failed to receive videos: {}", text).into())
    }
}

/// Receives and downloads videos and writes to the playlist file
async fn update_videos(
    client: &Client,
    config: &mut Config,
    data: &mut Data,
    updated: Option<DateTime<Utc>>,
) -> Result<(), Box<dyn Error>> {
    data.videos = receive_videos(client, config).await?;
    data.last_update = updated;
    data.write().await?;
    
    let home = std::env::var("HOME")?;

    // Remove the playlist file
    if Path::new(&format!("{home}/.local/share/signage/playlist.txt")).try_exists()? {

        tokio::fs::remove_file(format!("{home}/.local/share/signage/playlist.txt")).await?;
    }

    // Open the playlist file
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(format!("{home}/.local/share/signage/playlist.txt"))
        .await?;

    for video in data.videos.clone() {
        if !video.in_whitelist() {
            continue;
        }
        // Download the video and get the file path
        let file_path = video.download(client).await?;
        // Write the path to the playlist file
        file.write_all(format!("{}/.local/share/signage/{}.mp4\n", home, video.title).as_bytes()).await?;
    }

    fn capture_screenshot() -> Result<(), Box<dyn std::error::Error>> {
        
        println!("Screenshot captured!");
    Ok(())
    }


}