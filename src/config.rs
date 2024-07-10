use crate::util::{load_json, write_json};
use serde::{Deserialize, Serialize};
use std::{boxed::Box, env, error::Error};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub url: String,
    pub id: String,
    pub username: String,
    pub password: String,
    pub key: Option<String>,
}

impl Config {
    pub fn new() -> Self {
        Config::default()
    }

    /// Writes `Config` to $HOME/config/signage/signage.json
    pub async fn write(&self) -> Result<(), Box<dyn Error>> {
        let json_content = serde_json::to_string_pretty(self)?;
        println!("Writing to signage.json: {}", json_content);
        
        write_json(
            self,
            &format!("{}/.config/signage/signage.json", env::var("HOME")?),
        )
        .await
    }

    /// Writes `Config` to $HOME/config/signage/signage.json
    pub async fn write(&self) -> Result<(), Box<dyn Error>> {
        write_json(
            self,
            &format!("{}/.config/signage/signage.json", env::var("HOME")?),
        )
        .await
    }
}
