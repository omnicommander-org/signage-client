use serde::{Deserialize, Serialize};
use std::{boxed::Box, env, error::Error};
use tokio::fs::{self, File};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub url: String,
    pub id: String,
    pub username: String,
    pub password: String,
    pub key: Option<String>,
}

impl Config {
    pub fn new() -> Self {
        Config {
            url: String::new(),
            id: String::new(),
            username: String::new(),
            password: String::new(),
            key: None,
        }
    }

    /// Loads `Config` from $HOME/.local/share/signage/signage.json
    pub async fn load(&mut self) -> Result<(), Box<dyn Error>> {
        let path = format!("{}/.local/share/signage/signage.json", env::var("HOME")?);
        if Path::new(&path).exists() {
            let mut file = File::open(&path).await?;
            let mut contents = vec![];
            file.read_to_end(&mut contents).await?;
            *self = serde_json::from_slice(&contents)?;
        }

        Ok(())
    }

    /// Writes `Config` to $HOME/.local/share/signage/signage.json
    pub async fn write(&self) -> Result<(), Box<dyn Error>> {
        let path = format!("{}/.local/share/signage/signage.json", env::var("HOME")?);
        let mut file = File::create(&path).await?;
        file.write_all(&serde_json::to_vec_pretty(&self)?).await?;

        Ok(())
    }
}
