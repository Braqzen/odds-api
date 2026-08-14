use eyre::{Result, eyre};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::{select, sync::mpsc::Sender};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct WsClient {
    url: String,
    api_key: String,
    sport_ids: Vec<u8>,
}

impl WsClient {
    pub fn new(url: &str, api_key: &str, sport_ids: &[u8]) -> Self {
        Self {
            url: url.to_string(),
            api_key: api_key.to_string(),
            sport_ids: sport_ids.to_vec(),
        }
    }

    pub async fn run(&self, tx: Sender<Value>, token: CancellationToken) -> Result<()> {
        let (mut socket, _) = connect_async(&self.url).await?;

        let login = json!({
            "type": "login",
            "apiKey": self.api_key,
            "receiveType": "json",
            "channels": ["fixtures", "odds", "bookmakers"],
            "sportIds": self.sport_ids
        });

        socket.send(Message::Text(login.to_string().into())).await?;

        loop {
            select! {
                biased;

                _ = token.cancelled() => socket.close(None).await?,

                message = socket.next() => {
                    match message {
                        Some(Ok(Message::Text(text))) => {
                            let value = serde_json::from_str::<Value>(text.as_ref())?;
                            tx.send(value).await?;
                        }
                        Some(Ok(Message::Close(_))) | None => return Ok(()),
                        Some(Ok(_)) => {}
                        Some(Err(error)) => {
                            return Err(eyre!("websocket receive failed: {error}"));
                        }
                    }
                }
            }
        }
    }
}
