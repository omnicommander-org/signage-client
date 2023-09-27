use chrono::{DateTime, Utc};
use config::Config;
use data::Data;
use reqwest::Client;
use std::{boxed::Box, error::Error};
use tokio::process::{Child, Command};
use tokio::time::{self, Duration};
use tokio::io::AsyncWriteExt;
use util::{Apikey, Updated, Video};

mod config;
mod data;
mod util;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut config = Config::new();
    let mut data = Data::new();
    let client = Client::new();

    // Load the configs
    config.load().await?;
    data.load().await?;
    
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
            Err(error) => eprintln!("{}", error),
        }
    }
}

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

async fn recieve_videos(client: &Client, config: &Config) -> Result<Vec<Video>, Box<dyn Error>> {
    let res: Vec<Video> = client
        .get(format!("{}/recieve-videos/{}", config.url, config.id))
        .header("APIKEY", config.key.clone().unwrap())
        .send()
        .await?
        .json()
        .await?;

    println!("{:?}", res);
    Ok(res)
}

async fn update_videos(
    client: &Client,
    config: &Config,
    data: &mut Data,
    updated: Option<DateTime<Utc>>,
) -> Result<(), Box<dyn Error>> {
    data.videos = recieve_videos(&client, &config).await?;
    data.last_update = updated;
    data.write().await?;
    
    let home = std::env::var("HOME")?;

    // Remove the playlist file
    tokio::fs::remove_file(format!("{}/.local/share/signage/playlist.txt", home)).await?;

    // Open the playlist file
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(format!("{}/.local/share/signage/playlist.txt", home))
        .await?;

    for video in data.videos.clone() {
        // Download the video
        video.download(&client).await?;

        // Write the path to the playlist file
        file.write(format!("{}/.local/share/signage/{}.mp4\n", home, video.title).as_bytes()).await?;
    }

    Ok(())
}
