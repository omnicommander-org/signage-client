use daemonize::Daemonize;
use std::fs::File;
use std::{thread, time};
use reqwest::blocking::{get, Client};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Apikey {
    key: String
}

#[derive(Serialize, Deserialize, Debug)]
struct Video {
    title: String,
    url: String,
}

fn main() -> std::io::Result<()> {
    let stdout = File::create("/tmp/signage.out")?;
    let stderr = File::create("/tmp/signage.err")?;


    let daemonize = Daemonize::new()
        .pid_file("/tmp/signage.pid")
        .chown_pid_file(true)
        .working_directory("/tmp")
        .stdout(stdout)
        .stderr(stderr);

    match daemonize.start() {
        Ok(_) => println!("Success, daemonized"),
        Err(e) => eprintln!("Error: {}", e)
    }

    let sleep_duration = time::Duration::new(30, 0);
    let mut mpv_handle: Option<std::process::Child> = None;

    loop {
        get_videos(authenticate());

        if mpv_handle.is_some() {
            mpv_handle.unwrap().kill()?;
        }

        mpv_handle = Some(std::process::Command::new("mpv")
            .arg("--loop")
            .arg("/home/noah/Downloads/icelandwaterfall.mp4")
            .spawn()?);

        thread::sleep(sleep_duration);
    }
}

fn authenticate() -> Apikey {
    let apikey: Apikey = get("http://localhost:8080/get-new-key/fd7b5f34-0b07-4ea1-8840-a577c8a5a4ed").unwrap()
        .json().unwrap();

    println!("{}", apikey.key);
    apikey
}

fn get_videos(apikey: Apikey) {
    let videos: Vec<Video> = Client::new().get("http://localhost:8080/recieve-videos/fd7b5f34-0b07-4ea1-8840-a577c8a5a4ed").header("APIKEY", apikey.key).send().unwrap().json().unwrap();

    println!("{:?}", videos);
}
