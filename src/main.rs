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
    config.load().await?;
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
        update_videos(&client, &config, &mut data, updated).await?;
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
            Err(error) => eprintln!("{error}"),
        }
    }
}

/// Loops until we get a response from the api to make sure our network is online
async fn wait_for_api(client: &Client, config: &Config) -> Result<bool, Box<dyn Error>> {
    let mut interval = time::interval(Duration::from_secs(1));
    loop {
        let res = client.get(format!("{}/health", config.url)).send().await;
        if res.is_ok() {
            match res.unwrap().status() {
                StatusCode::OK => break,
                StatusCode::INTERNAL_SERVER_ERROR => {
                    time::interval(Duration::from_secs(120)).tick().await;
                },
                _ => (),
            }
        }
        interval.tick().await;
    }

    Ok(true)
}

/// Starts the mpv player with the proper playlist and flags
async fn start_mpv() -> Result<Child, Box<dyn Error>> {
    let child = Command::new("mpv")
        // .arg("-fs")
        .arg("--loop-playlist=inf")
        .arg("--volume=-1")
        .arg("--no-terminal")
        .arg(format!("--playlist={}/.local/share/signage/playlist.txt", std::env::var("HOME")?))
        .spawn()?;

    Ok(child)
}

/// Makes the proper request to recieve an apikey
async fn get_new_key(client: &Client, config: &Config) -> Result<Apikey, Box<dyn Error>> {
    let res: Apikey = client
        .get(format!("{}/get-new-key/{}", config.url, config.id))
        .basic_auth(&config.username, Some(&config.password))
        .send()
        .await?
        .json()
        .await?;

    println!("{}", res.key);
    Ok(res)
}

/// Makes the proper request to recieve the last time the connected group was updated
async fn sync(client: &Client, config: &Config) -> Result<Option<DateTime<Utc>>, Box<dyn Error>> {
    let res: Updated = client
        .get(format!("{}/sync/{}", config.url, config.id))
        .header("APIKEY", config.key.clone().unwrap())
        .send()
        .await?
        .json()
        .await?;

    println!("{:?}", res.updated);

    Ok(res.updated)
}

/// Makes the proper request to recieve the list of videos
async fn recieve_videos(client: &Client, config: &Config) -> Result<Vec<Video>, Box<dyn Error>> {
    let res: Vec<Video> = client
        .get(format!("{}/recieve-videos/{}", config.url, config.id))
        .header("APIKEY", config.key.clone().unwrap())
        .send()
        .await?
        .json()
        .await?;

    println!("{res:?}");
    Ok(res)
}

/// Recieves and downloads videos and writes to the playlist file
async fn update_videos(
    client: &Client,
    config: &Config,
    data: &mut Data,
    updated: Option<DateTime<Utc>>,
) -> Result<(), Box<dyn Error>> {
    data.videos = recieve_videos(client, config).await?;
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

        // Download the video
        video.download(client).await?;

        // Write the path to the playlist file
        file.write_all(format!("{}/.local/share/signage/{}.mp4\n", home, video.id).as_bytes()).await?;
    }

    Ok(())
}
