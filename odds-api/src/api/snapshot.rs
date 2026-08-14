use crate::api::types::{Game, MatchPrice};
use eyre::{Result, eyre};
use reqwest::{Client, Response, StatusCode, header};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Clone)]
pub struct SnapshotClient {
    url: String,
    api_key: String,
    client: Client,
}

impl SnapshotClient {
    pub fn new(rest: String, api_key: String) -> Self {
        Self {
            url: rest,
            api_key,
            client: Client::new(),
        }
    }

    pub async fn fixtures(&self, sport_id: u8) -> Result<Vec<Game>> {
        let url = format!(
            "{}/fixtures?sportId={sport_id}&apiKey={}",
            self.url, self.api_key
        );

        let response = self.retry(&url).await?;
        Ok(response.json().await?)
    }

    pub async fn odds(&self, fixture_ids: &[String]) -> Result<Vec<MatchPrice>> {
        let ids = fixture_ids.join(",");
        let url = format!(
            "{}/fixtures/odds/main?fixtureIds={ids}&apiKey={}",
            self.url, self.api_key
        );
        let response = self.retry(&url).await?;
        Ok(response.json().await?)
    }

    async fn retry(&self, url: &str) -> Result<Response> {
        loop {
            let response = self.client.get(url).send().await?;
            match response.status() {
                StatusCode::OK => return Ok(response),
                StatusCode::TOO_MANY_REQUESTS => {
                    let header = match response.headers().get(header::RETRY_AFTER) {
                        Some(value) => value,
                        None => return Err(eyre!("429 missing Retry-After")),
                    };
                    let secs: u64 = header.to_str()?.parse()?;
                    sleep(Duration::from_secs(secs)).await;
                }
                status => {
                    let body = response.text().await?;
                    return Err(eyre!("snapshot request status {status}: {body}"));
                }
            }
        }
    }
}
