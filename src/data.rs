use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{boxed::Box, env, error::Error};

use crate::util::{load_json, write_json, Video};

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Data {
    pub videos: Vec<Video>,
    pub last_update: Option<DateTime<Utc>>,
}

impl Data {
    pub fn new() -> Self {
        Default::default()
    }

    pub async fn load(self: &mut Self) -> Result<(), Box<dyn Error>> {
        load_json(
            self,
            &format!("{}/.local/share/signage", env::var("HOME")?),
            "data.json",
        )
        .await
    }

    pub async fn write(self: &Self) -> Result<(), Box<dyn Error>> {
        write_json(
            self,
            &format!("{}/.local/share/signage/data.json", env::var("HOME")?),
        )
        .await
    }
}