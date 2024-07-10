use chrono::{DateTime, Utc};
use config::Config;
use data::Data;
use reqwest::{Client, StatusCode};
use std::{boxed::Box, error::Error, path::Path};
use tokio::process::{Child, Command};
use tokio::time::{self, Duration};
use tokio::io::AsyncWriteExt;
use util::{capture_screenshot, Apikey, Updated, Video};

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
        println!("API key is not set. Requesting a new API key...");
        config.key = Some(get_new_key(&client, &config).await?.key);
        config.write().await?;
    }
    
    // Print the API key
    if let Some(api_key) = &config.key {
        println!("API Key: {}", api_key);
    }

    config.write().await?;

    // Get the videos if we've never updated
    if data.last_update.is_none() {
        let updated = sync(&client, &config).await?;
        update_videos(&client, &config, &mut data, updated).await?;
        println!("Data Updated: {:?}", updated);    
    }


    let mut interval = time::interval(Duration::from_secs(30));
    let mut mpv = start_mpv().await?;

    loop {
        interval.tick().await;

        let updated = sync(&client, &config).await?;

        // Update videos if the group was updated
        if updated > data.last_update {
            update_videos(&client, &config, &mut data, updated).await?;
            mpv.kill().await?;
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
    println!("Waiting for API to become available...");
    let mut interval = time::interval(Duration::from_secs(1));
    loop {
        let res = client.get(format!("{}/health", config.url)).send().await;
        if let Ok(response) = res {
            match response.status() {
                StatusCode::OK => break,
                StatusCode::INTERNAL_SERVER_ERROR => {
                    println!("Server error. Retrying in 2 minutes...");
                    time::interval(Duration::from_secs(120)).tick().await;
                },
                _ => (),
            }
        }
        interval.tick().await;
    }

    println!("API is available.");
    Ok(true)
}

/// Starts the mpv player with the proper playlist and flags
async fn start_mpv() -> Result<Child, Box<dyn Error>> {
    println!("Starting mpv player...");
    let image_display_duration = 10;
    let child = Command::new("mpv")
        .arg("--loop-playlist=inf")
        .arg("--volume=-1")
        .arg("--no-terminal")
        .arg("--fullscreen")
        .arg(format!("--image-display-duration={}", image_display_duration))
        .arg(format!("--playlist={}/.local/share/signage/playlist.txt", std::env::var("HOME")?))
        .spawn()?;
    println!("mpv player started.");
    Ok(child)
}

/// Makes the proper request to receive an API key
async fn get_new_key(client: &Client, config: &Config) -> Result<Apikey, Box<dyn Error>> {
    println!("Requesting a new API key...");
    let res: Apikey = client
        .get(format!("{}/get-new-key/{}", config.url, config.id))
        .basic_auth(&config.username, Some(&config.password))
        .send()
        .await?
        .json()
        .await?;

    println!("Received new API key: {}", res.key);
    Ok(res)
}

/// Makes the proper request to receive the last time the connected playlist was updated
async fn sync(client: &Client, config: &Config) -> Result<Option<DateTime<Utc>>, Box<dyn Error>> {
    println!("Syncing with the server...");
    println!("Current Config: {:?}", config); // Print the entire config
    let res: Updated = client
        .get(format!("{}/sync/{}", config.url, config.id))
        .header("APIKEY", config.key.clone().unwrap_or_default())
        .send()
        .await?
        .json()
        .await?;


    Ok(res.updated)
}

/// Makes the proper request to receive the list of videos
async fn receive_videos(client: &Client, config: &Config) -> Result<Vec<Video>, Box<dyn Error>> {
    println!("Receiving videos...");

    let url = format!("{}/recieve-videos/{}", config.url, config.id);
    let standard_api_key = config.key.clone().unwrap_or_default();

    // Request a new authorization token
    let new_key = get_new_key(client, config).await?;
    let auth_token = new_key.key;

    // Print the values for debugging
    println!("Request URL: {}", url);
    println!("Standard API Key: {}", standard_api_key);
    println!("Authorization Token: {}", auth_token);

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

    println!("Response status: {}", status);
    println!("Response body: {}", text);

    if status.is_success() {
        let res: Vec<Video> = serde_json::from_str(&text)?;
        println!("Received videos: {:?}", res);
        Ok(res)
    } else {
        Err(format!("Failed to receive videos: {}", text).into())
    }
}

/// Receives and downloads videos and writes to the playlist file
async fn update_videos(
    client: &Client,
    config: &Config,
    data: &mut Data,
    updated: Option<DateTime<Utc>>,
) -> Result<(), Box<dyn Error>> {
    data.videos = receive_videos(client, config).await?;
    data.last_update = updated;
    data.write().await?;
    println!("Last Updated: {:?}", updated);
    let home = std::env::var("HOME")?;

    // Remove the playlist file
    if Path::new(&format!("{home}/.local/share/signage/playlist.txt")).try_exists()? {
        println!("Removing old playlist file...");
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
        println!("Downloading video: {}", video.id);
        let file_path = video.download(client).await?;
        // Write the path to the playlist file
        file.write_all(format!("{}\n", file_path).as_bytes()).await?;
    }
    println!("Updated playlist file.");
    Ok(())
}


