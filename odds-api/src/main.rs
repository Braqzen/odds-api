mod api;
mod bootstrap;
mod processor;
mod risk;
mod state;
mod worker;

use eyre::{Result, eyre};
use maiya::logs::Logger;
use opentelemetry_sdk::Resource;
use worker::Worker;

#[tokio::main]
async fn main() -> Result<()> {
    let resource = Resource::builder().with_service_name("client").build();
    let logger = Logger::new(&resource, "client").map_err(|error| eyre!("{error}"))?;

    let rest = std::env::var("ODDS_REST_URL")?;
    let ws = std::env::var("ODDS_WS_URL")?;
    let api_key = std::env::var("ODDS_API_KEY")?;
    let sport_ids_raw = std::env::var("ODDS_SPORT_IDS")?;

    let sport_ids: Vec<u8> = sport_ids_raw
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::parse)
        .collect::<Result<_, _>>()
        .map_err(|error| eyre!("invalid ODDS_SPORT_IDS: {error}"))?;

    let mut worker = Worker::new(rest, ws, api_key, sport_ids);
    let result = worker.run().await;

    logger.shutdown().map_err(|error| eyre!("{error}"))?;

    result
}
