use clap::{Parser, ValueEnum};
use reqwest::{Client, StatusCode};
use serde_json::Value;
use std::{collections::HashMap, env, error::Error, future::Future, str::FromStr, time::Duration};
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

const INDEFINITITE_RETRY_DELAY: Duration = Duration::from_secs(60);

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let token = env::var("TOKEN").unwrap();

    let (ip_url, ip_description, record_description) = match cli.ip {
        Some(CliIpVersion::IPv6) => IPV6,
        Some(CliIpVersion::IPv4) => IPV4,
        None => IPV6,
    };

    let client: Client = Client::new();

    let record_url = format!(
        "https://api.linode.com/v4/domains/{}/records/{}",
        cli.domain_id, cli.record_id
    );

    let mut current_ip = retry_indefinitely(|| async {
        let record_json = client
            .get(&record_url)
            .bearer_auth(&token)
            .send()
            .await?
            .json::<HashMap<String, Value>>()
            .await?;

        if let Some(Value::String(target)) = record_json.get("target") {
            Ok::<String, _>(target.clone())
        } else {
            Err::<_, Box<dyn Error>>(
                format!(
                    "Error extracting current {} address from response",
                    ip_description
                )
                .into(),
            )
        }
    })
    .await;

    loop {
        let new_ip = retry_indefinitely(|| async {
            Ok::<String, Box<dyn Error>>(client.get(ip_url).send().await?.text().await?)
        })
        .await;

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

async fn retry_indefinitely<T, F, R, E>(call: T) -> R
where
    T: Fn() -> F,
    F: Future<Output = Result<R, E>>,
{
    loop {
        let result = call().await;
        if let Ok(value) = result {
            return value;
        }
        sleep(INDEFINITITE_RETRY_DELAY).await
    }
}
