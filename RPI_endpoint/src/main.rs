use std::fs::File;
use std::io::Write;
use std::process::Command;
use serde::Serialize;
use std::thread;
use std::time::Duration;
use reqwest::blocking::Client;
use reqwest::header::{CONTENT_TYPE, HeaderValue, HeaderMap};
use uuid::Uuid;

fn run_command(command: &str, args: &[&str]) -> String {
    let output = Command::new(command)
        .args(args)
        .output()
        .expect("Failed to execute command");

    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn temp() -> String {
    run_command("vcgencmd", &["measure_temp"])
}

fn cpuusage() -> String {
    run_command("sh", &["-c", "top -bn1 | grep 'Cpu(s)'"])
}

fn memory() -> String {
    run_command("free", &["-h"])
}

fn diskusage() -> String {
    run_command("df", &["-h"])
}

fn swapusage() -> String {
    run_command("swapon", &["--summary"])
}

fn uptime() -> String {
    run_command("uptime", &[])
}

fn mpvstatus() -> String {
    let output = run_command("sh", &["-c", "ps aux | grep -v grep | grep mpv"]);
    if output.is_empty() {
        "MPV is not running".to_string()
    } else {
        "MPV is running".to_string()
    }
}

#[derive(Serialize)]
struct Metrics {
    client_id: String,
    temp: String,
    cpuusage: String,
    memory: String,
    diskusage: String,
    swapusage: String,
    uptime: String,
    mpvstatus: String,
}

fn collect_and_write_metrics(client_id: &str) -> Metrics {
    let metrics = Metrics {
        client_id: client_id.to_string(),
        temp: temp(),
        cpuusage: cpuusage(),
        memory: memory(),
        diskusage: diskusage(),
        swapusage: swapusage(),
        uptime: uptime(),
        mpvstatus: mpvstatus(),
    };

    // Serialize metrics to JSON
    let json = serde_json::to_string_pretty(&metrics).expect("Failed to serialize metrics");

    // Write JSON to a file
    let mut file = File::create("metrics.json").expect("Failed to create file");
    file.write_all(json.as_bytes()).expect("Failed to write to file");

    // Print to console for verification
    println!("{}", json);

    metrics
}

fn send_metrics(client_id: &str, metrics: &Metrics, api_key: &str) {
    // Check if the client_id is a valid UUID
    if let Err(_) = Uuid::parse_str(client_id) {
        println!("Invalid client ID format: {}", client_id);
        return;
    }

    let client = Client::new();
    let url = format!("https://ds-dev-api.omnicommando.com/client_vitals/{}", client_id);
    
    // Print the URL for debugging
    println!("Sending metrics to URL: {}", url);

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert("Apikey", HeaderValue::from_str(api_key).expect("Invalid API key"));

    let res = client.post(&url)
        .headers(headers)
        .json(metrics)
        .send()
        .expect("Failed to send metrics");

    let status = res.status();
    if status.is_success() {
        println!("Successfully sent metrics");
    } else {
        let error_text = res.text().unwrap_or_else(|_| "Failed to read error text".to_string());
        println!("Failed to send metrics: {:?}\nError: {}", status, error_text);
    }
}

fn main() {
    let client_id = "24f978c6-5479-407d-8cd9-3b0a7fd4d5e2";
    let api_key = "V1d0zkcqbGLyOyHVPMP7qvJ7xM/1WMK/i+fstrL0zv8="; 

    loop {
        let metrics = collect_and_write_metrics(client_id);
        send_metrics(client_id, &metrics, api_key);
        // Sleep for 15 minutes
        thread::sleep(Duration::from_secs(900));
    }
}
