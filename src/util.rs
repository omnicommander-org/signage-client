use chrono::{DateTime, Utc};
use futures_util::StreamExt;
use reqwest::Client;
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
    pub id: String,
    pub asset_url: String,
}

#[derive(Serialize, Deserialize)]
pub struct Updated {
    pub updated: Option<DateTime<Utc>>,
}

impl Video {
    /// Downloads videos to `$HOME/.local/share/signage`
    pub async fn download(&self, client: &Client) -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = client.get(self.asset_url.clone()).send().await?.bytes_stream();
        let mut file = tokio::fs::File::create(format!(
            "{}/.local/share/signage/{}.mp4",
            std::env::var("HOME")?,
            self.id
        ))
        .await?;

        while let Some(content) = stream.next().await {
            tokio::io::copy(&mut content?.as_ref(), &mut file).await?;
        }

        Ok(())
    }

    pub fn in_whitelist(&self) -> bool {
        let whitelist = [
            "player.vimeo.com",
        ];

        for url in whitelist {
            if self.asset_url.contains(url) {
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
