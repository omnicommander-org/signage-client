use crate::util::{load_json, write_json};
use serde::{Deserialize, Serialize};
use std::{boxed::Box, env, error::Error};

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub url: String,
    pub id: String,
    pub key: Option<String>,
}

impl Config {
    pub fn new() -> Self {
        Default::default()
    }

    pub async fn load(self: &mut Self) -> Result<(), Box<dyn Error>> {
        load_json(
            self,
            &format!("{}/.config/signage", env::var("HOME")?),
            "signage.json",
        )
        .await
    }

    pub async fn write(self: &Self) -> Result<(), Box<dyn Error>> {
        write_json(
            self,
            &format!("{}/.config/signage/signage.json", env::var("HOME")?),
        )
        .await
    }
}