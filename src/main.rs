use clap::Parser;
use reqwest::StatusCode;
use serde_json::Value;
use std::{collections::HashMap, str::FromStr, time::Duration};
use tokio::time::sleep;

#[derive(Parser)]
struct Cli {
    #[arg(long)]
    domain_id: i64,
    #[arg(long)]
    record_id: i64,
    #[arg(long)]
    delay: u16,
    #[arg(long)]
    token: String,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let client: reqwest::Client = reqwest::Client::new();

    let record_url = format!(
        "https://api.linode.com/v4/domains/{}/records/{}",
        cli.domain_id, cli.record_id
    );

    let record_json = client
        .get(&record_url)
        .bearer_auth(&cli.token)
        .send()
        .await
        .unwrap()
        .json::<HashMap<String, Value>>()
        .await
        .unwrap();

    let mut current_ip = (if let Value::String(target) = record_json.get("target").unwrap() {
        Some(target)
    } else {
        None
    })
    .unwrap()
    .clone();

    loop {
        let new_ip = client
            .get("https://api6.ipify.org")
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        if new_ip != current_ip {
            println!(
                "IPv6 address changed from {} to {}, updating AAAA record...",
                current_ip, new_ip
            );

            let mut target_json = HashMap::new();

            target_json.insert(String::from_str("target").unwrap(), &new_ip);

            let result = client
                .put(&record_url)
                .bearer_auth(&cli.token)
                .json(&target_json)
                .send()
                .await
                .unwrap()
                .status();

            if result == StatusCode::OK {
                println!("AAAA record updated");
            } else {
                println!("Failed to update AAAA record");
            }

            current_ip = new_ip;
        } else {
            println!("IPv6 unchaged from {}", current_ip);
        }

        sleep(Duration::from_secs(cli.delay as u64)).await;
    }
}
