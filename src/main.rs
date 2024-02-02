use clap::{Parser, ValueEnum};
use reqwest::StatusCode;
use serde_json::Value;
use std::{collections::HashMap, env, str::FromStr, time::Duration};
use tokio::time::sleep;

#[derive(Clone, ValueEnum)]
enum CliIpVersion {
    #[value(name = "v6")]
    IPv6,
    #[value(name = "v4")]
    IPv4,
}

#[derive(Parser)]
struct Cli {
    #[arg(long)]
    domain_id: i64,
    #[arg(long)]
    record_id: i64,
    #[arg(long)]
    delay: u16,
    #[arg(long, value_enum)]
    ip: Option<CliIpVersion>,
}

const IPV6: (&str, &str, &str) = ("https://api6.ipify.org", "IPv6", "AAAA");
const IPV4: (&str, &str, &str) = ("https://api.ipify.org", "IPv4", "A");

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let token = env::var("TOKEN").unwrap();

    let (ip_url, ip_description, record_description) = match cli.ip {
        Some(CliIpVersion::IPv6) => IPV6,
        Some(CliIpVersion::IPv4) => IPV4,
        None => IPV6,
    };

    let client: reqwest::Client = reqwest::Client::new();

    let record_url = format!(
        "https://api.linode.com/v4/domains/{}/records/{}",
        cli.domain_id, cli.record_id
    );

    let record_json = client
        .get(&record_url)
        .bearer_auth(&token)
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
            .get(ip_url)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        if new_ip != current_ip {
            println!(
                "{} address changed from {} to {}, updating {} record...",
                ip_description, current_ip, new_ip, record_description
            );

            let mut target_json = HashMap::new();

            target_json.insert(String::from_str("target").unwrap(), &new_ip);

            let result = client
                .put(&record_url)
                .bearer_auth(&token)
                .json(&target_json)
                .send()
                .await
                .unwrap()
                .status();

            if result == StatusCode::OK {
                println!("{} record updated", record_description);
            } else {
                println!("Failed to update {} record", record_description);
            }

            current_ip = new_ip;
        } else {
            println!("{} unchaged from {}", ip_description, current_ip);
        }

        sleep(Duration::from_secs(cli.delay as u64)).await;
    }
}
