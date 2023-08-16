use daemonize::Daemonize;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{thread, time, fs::File, rc::Rc};
use anyhow::Result;

#[derive(Serialize, Deserialize)]
struct Apikey {
    key: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Video {
    title: String,
    url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let url = "http://localhost:8080";
    let id = "fd7b5f34-0b07-4ea1-8840-a577c8a5a4ed";

    // Wrap the client in an Rc so we're not using unnecessary memory
    let client = Rc::new(Client::new());

    let stdout = File::create("/tmp/signage.out")?;
    let stderr = File::create("/tmp/signage.err")?;

    // Start the daemon
    Daemonize::new()
        .pid_file("/tmp/signage.pid")
        .chown_pid_file(true)
        .working_directory("/tmp")
        .stdout(stdout)
        .stderr(stderr)
        .start()?;

    let mut mpv: Option<std::process::Child> = None;

    loop {
        get_videos(client.clone(), url, id, authenticate(client.clone(), url, id).await?).await?;

        if mpv.is_some() {
            mpv.unwrap().kill()?;
        }

        mpv = Some(
            std::process::Command::new("mpv")
                .arg("--loop")
                .arg("/home/noah/Downloads/icelandwaterfall.mp4")
                .spawn()?,
        );

        thread::sleep(time::Duration::new(30, 0));
    }
}

async fn authenticate(client: Rc<Client>, url: &str, id: &str) -> Result<Apikey> {
    let apikey: Apikey = client.get(String::from(url) + "/get-new-key/" + id).send().await?.json().await?;

    println!("{}", apikey.key);
    Ok(apikey)
}

async fn get_videos(client: Rc<Client>, url: &str, id: &str, apikey: Apikey) -> Result<()> {
    let videos: Vec<Video> = client 
        .get(String::from(url) + "/recieve-videos/" + id)
        .header("APIKEY", apikey.key)
        .send()
        .await?
        .json()
        .await?;

    println!("{:?}", videos);
    Ok(())
}
