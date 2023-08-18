use chrono::{DateTime, Utc};
use config::Config;
use data::Data;
use reqwest::Client;
use std::{boxed::Box, error::Error};
use tokio::process::{Child, Command};
use tokio::time::{self, Duration};
use util::{Apikey, Updated, Video};

mod config;
mod data;
mod util;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut config = Config::new();
    let mut data = Data::new();
    let client = Client::new();

    config.load().await?;
    data.load().await?;

    if data.last_update.is_none() {
        let updated = sync(&client, &config).await?;
        update_videos(&client, &config, &mut data, updated).await?;
    }

    let mut mpv = start_mpv().await?;

    if config.key.is_none() {
        config.key = Some(get_new_key(&client, &config).await?.key);
    }
    config.write().await?;

    let mut interval = time::interval(Duration::from_secs(5));

    loop {
        interval.tick().await;

        let updated = sync(&client, &config).await?;

        if updated > data.last_update {
            update_videos(&client, &config, &mut data, updated).await?;
            mpv.kill().await?;
        }

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
        .arg("/home/noah/Downloads/icelandwaterfall.mp4")
        .spawn()?;

    Ok(child)
}

async fn get_new_key(client: &Client, config: &Config) -> Result<Apikey, Box<dyn Error>> {
    let res: Apikey = client
        .get(format!("{}/get-new-key/{}", config.url, config.id))
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

    // for video in data.videos.clone() {
    //     video.download(&client).await?;
    // }

    Ok(())
}
