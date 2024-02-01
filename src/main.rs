use clap::Parser;
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

    loop {
        let ipv6_address = client
            .get("https://api6.ipify.org")
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        println!("IPv6 address: {}", ipv6_address);

        let put_url = format!(
            "https://api.linode.com/v4/domains/{}/records/{}",
            cli.domain_id, cli.record_id
        );

        let mut target_json = HashMap::new();

        target_json.insert(String::from_str("target").unwrap(), ipv6_address);

        let result = client
            .put(put_url)
            .bearer_auth(&cli.token)
            .json(&target_json)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        println!("Result: {}", result);

        sleep(Duration::from_secs(cli.delay as u64)).await;
    }
}
