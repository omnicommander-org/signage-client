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

    /// Loads `Config` from $HOME/.config/signage/signage.json
    pub async fn load(&mut self) -> Result<(), Box<dyn Error>> {
        load_json(
            self,
            &format!("{}/.config/signage", env::var("HOME")?),
            "signage.json",
        )
        .await
    }

    /// Writes `Config` to $HOME/.config/signage/signage.json
    pub async fn write(&self) -> Result<(), Box<dyn Error>> {
        let json_content = serde_json::to_string_pretty(self)?;
        println!("Writing to signage.json: {}", json_content);
        
        write_json(
            self,
            &format!("{}/.config/signage/signage.json", env::var("HOME")?),
        )
        .await
    }

    /// Updates the API key and writes the updated configuration to the file
    pub async fn update_key(&mut self, new_key: String) -> Result<(), Box<dyn Error>> {
        self.key = Some(new_key);
        self.write().await
    }
}
