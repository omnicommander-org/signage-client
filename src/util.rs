use chrono::{DateTime, Utc};
use futures_util::StreamExt;
use image::{ImageBuffer, RgbaImage};
use reqwest::Client;
use screenshots::Screen;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{boxed::Box, error::Error, fs, path::Path};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};

#[derive(Serialize, Deserialize, Clone)]
pub struct Apikey {
    pub key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Video {
    pub title: String,
    pub url: String,
}

#[derive(Serialize, Deserialize)]
pub struct Updated {
    pub updated: Option<DateTime<Utc>>,
}

impl Video {
    /// Downloads videos to `$HOME/.local/share/signage`
    pub async fn download(&self, client: &Client) -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = client.get(self.url.clone()).send().await?.bytes_stream();
        let mut file = tokio::fs::File::create(format!(
            "{}/.local/share/signage/{}.mp4",
            std::env::var("HOME")?,
            self.title
        ))
        .await?;

        while let Some(content) = stream.next().await {
            tokio::io::copy(&mut content?.as_ref(), &mut file).await?;
        }

        Ok(())
    }

    pub fn in_whitelist(&self) -> bool {
        let whitelist = ["player.vimeo.com"];

        for url in whitelist {
            if self.url.contains(url) {
                return true;
            }
        }

        false
    }
}

/// Loads json from `dir/filename` into `T`
pub async fn load_json<T: Serialize + DeserializeOwned>(
    json: &mut T,
    dir: &str,
    filename: &str,
) -> Result<(), Box<dyn Error>> {
    if Path::new(&format!("{dir}/{filename}")).try_exists()? {
        let mut file = File::open(format!("{dir}/{filename}")).await?;
        let mut contents = vec![];
        file.read_to_end(&mut contents).await?;
        *json = serde_json::from_slice(&contents)?;
    } else {
        fs::create_dir_all(dir)?;
        write_json(json, &format!("{dir}/{filename}")).await?;
    }

    Ok(())
}

/// Writes json from `T` into `path`
pub async fn write_json<T: Serialize>(json: &T, path: &str) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(path).await?;
    file.write_all(&serde_json::to_vec_pretty(&json)?).await?;

    Ok(())
}

pub fn capture_screenshot() -> Result<(), Box<dyn std::error::Error>> {
    let screens = Screen::all().unwrap();
    let screen = &screens[0]; // Assuming you want to capture the first screen

    // Capture the screen
    let image = screen.capture().unwrap();
    let width = image.width();
    let height = image.height();
    let buffer = image.to_vec(); // Get the raw pixel data as Vec<u8>

    // Convert the buffer to an image
    let img_buffer: RgbaImage =
        ImageBuffer::from_raw(width as u32, height as u32, buffer).unwrap();

    // Save the image
    img_buffer.save(Path::new(&format!("{}/.local/share/signage/screenshot.png", std::env::var("HOME")?)))?;

    println!("Screenshot saved successfully.");

    Ok(())
}

