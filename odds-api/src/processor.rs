use crate::{
    api::types::{BookmakersPayload, Game, OddsPayload},
    risk::{BookmakerInput, Decision, QuoteInput, Validator},
    state::State,
};
use eyre::{Result, eyre};
use serde_json::Value;
use tokio::{
    select,
    sync::mpsc::{Receiver, Sender, error::TrySendError},
};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

pub struct Processor {
    state: Option<State>,
    validator: Validator,
    freeze: bool,
    snapshot_tx: Sender<()>,
}

impl Processor {
    pub fn new(validator: Validator, snapshot_tx: Sender<()>) -> Self {
        Self {
            state: None,
            validator,
            freeze: true,
            snapshot_tx,
        }
    }

    pub async fn run(
        mut self,
        mut updates: Receiver<Value>,
        mut snapshots: Receiver<State>,
        token: CancellationToken,
    ) {
        loop {
            select! {
                biased;

                _ = token.cancelled() => break,

                snapshot = snapshots.recv() => {
                    let Some(snapshot) = snapshot else {
                        break;
                    };

                    info!(
                        matches = snapshot.matches.len(),
                        prices = snapshot.prices.len(),
                        "Unfreezing"
                    );
                    self.state = Some(snapshot);
                    self.freeze = false;
                }

                message = updates.recv() => {
                    let Some(message) = message else {
                        break;
                    };

                    if let Err(error) = self.process(message).await {
                        error!(%error, "processing websocket message failed");
                    }
                }
            }
        }
    }

    fn request_snapshot(&mut self) -> Result<()> {
        info!("Freeze request");
        self.freeze = true;

        match self.snapshot_tx.try_send(()) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => Ok(()),
            Err(TrySendError::Closed(_)) => {
                Err(eyre!("bootstrap snapshot request failed: channel closed"))
            }
        }
    }

    async fn process(&mut self, message: Value) -> Result<()> {
        let mut object = match message {
            Value::Object(object) => object,
            _ => return Err(eyre!("websocket message is not an object")),
        };

        let message_type = match object.remove("type") {
            Some(Value::String(value)) => value,
            _ => return Err(eyre!("websocket message missing type")),
        };

        if message_type == "snapshot_required" {
            return self.request_snapshot();
        }

        if message_type != "UPDATE" {
            info!(message_type, "websocket control message");
            return Ok(());
        }

        if self.freeze {
            return Ok(());
        }

        let channel = match object.remove("channel") {
            Some(Value::String(value)) => value,
            _ => return Err(eyre!("websocket update missing channel")),
        };

        let payload = match object.remove("payload") {
            Some(value) => value,
            None => return Err(eyre!("websocket update missing payload")),
        };

        match channel.as_str() {
            "fixtures" => self.process_fixture(payload).await,
            "odds" => self.process_odds(payload).await,
            "bookmakers" => self.process_bookmakers(payload).await,
            _ => Err(eyre!("unsupported websocket channel: {channel}")),
        }
    }

    async fn process_fixture(&mut self, payload: Value) -> Result<()> {
        let Some(state) = self.state.as_mut() else {
            return Ok(());
        };

        let game = serde_json::from_value::<Game>(payload)?;

        let fixture_id = game.fixture_id.clone();
        let status_id = game.status.status_id;

        match self.validator.game(game).await? {
            Decision::Enable(game) => {
                info!(
                    fixture_id,
                    ?status_id,
                    matches = state.matches.len(),
                    "Updating game"
                );

                state.matches.insert(fixture_id, game);
            }
            Decision::Disable(reason) => {
                let removed_prices = state.remove_fixture(&fixture_id);
                info!(
                    fixture_id,
                    ?status_id,
                    reason,
                    removed_prices,
                    matches = state.matches.len(),
                    "Game update: processed"
                );
            }
            Decision::Ignore(reason) => {
                warn!(fixture_id, reason, "Game update: ignored");
            }
        }

        Ok(())
    }

    async fn process_odds(&mut self, payload: Value) -> Result<()> {
        let Some(state) = self.state.as_mut() else {
            return Ok(());
        };

        let payload = match serde_json::from_value::<OddsPayload>(payload) {
            Ok(payload) => payload,
            Err(error) => return Err(eyre!("odds payload decode failed: {error}")),
        };

        let fixture_id = payload.fixture_id;
        let bookmakers = payload.odds.len();
        let mut quotes_received = 0usize;
        let mut quotes_applied = 0usize;

        for bookmaker_odds in payload.odds.into_values() {
            for (odds_id, quote) in bookmaker_odds {
                quotes_received += 1;
                let fixture_exists = state.matches.contains_key(&fixture_id);
                let bookmaker_available = match state.matches.get(&fixture_id) {
                    Some(fixture) => fixture.bookmakers.contains_key(&quote.bookmaker),
                    None => false,
                };
                let current_changed_at =
                    state.prices.get(&odds_id).map(|current| current.changed_at);

                let input = QuoteInput::new(
                    fixture_id.clone(),
                    odds_id.clone(),
                    fixture_exists,
                    bookmaker_available,
                    current_changed_at,
                    quote,
                );

                match self.validator.quote(input).await? {
                    Decision::Enable(input) => {
                        state
                            .odds_by_fixture
                            .entry(input.fixture_id)
                            .or_default()
                            .insert(input.odds_id.clone());
                        state.prices.insert(input.odds_id, input.quote);
                        quotes_applied += 1;
                    }
                    Decision::Disable(reason) => {
                        state.remove_price(&fixture_id, &odds_id);
                        info!(fixture_id, odds_id, reason, "Quote update: processed");
                    }
                    Decision::Ignore(reason) => {
                        warn!(fixture_id, odds_id, reason, "Quote update: ignored");
                    }
                }
            }
        }

        info!(
            fixture_id,
            bookmakers,
            quotes_received,
            quotes_applied,
            prices = state.prices.len(),
            "Odds update: processed"
        );

        Ok(())
    }

    async fn process_bookmakers(&mut self, payload: Value) -> Result<()> {
        let Some(state) = self.state.as_mut() else {
            return Ok(());
        };

        let payload = match serde_json::from_value::<BookmakersPayload>(payload) {
            Ok(payload) => payload,
            Err(error) => {
                return Err(eyre!("bookmakers payload decode failed: {error}"));
            }
        };

        let fixture_id = payload.fixture_id;
        let bookmakers = payload.bookmakers.len();
        let mut kept = 0usize;

        for (bookmaker_id, meta) in payload.bookmakers {
            let input = BookmakerInput::new(
                fixture_id.clone(),
                bookmaker_id.clone(),
                state.matches.contains_key(&fixture_id),
                meta,
            );

            match self.validator.bookmaker(input).await? {
                Decision::Enable(input) => {
                    if let Some(fixture) = state.matches.get_mut(&input.fixture_id) {
                        fixture.bookmakers.insert(input.bookmaker_id, input.meta);
                    }
                    kept += 1;
                }
                Decision::Disable(reason) => {
                    if let Some(fixture) = state.matches.get_mut(&fixture_id) {
                        fixture.bookmakers.remove(&bookmaker_id);
                    }
                    state.remove_bookmaker_prices(&fixture_id, &bookmaker_id);
                    info!(
                        fixture_id,
                        bookmaker = bookmaker_id,
                        reason,
                        "Bookmaker update: processed"
                    );
                }
                Decision::Ignore(reason) => {
                    warn!(
                        fixture_id,
                        bookmaker = bookmaker_id,
                        reason,
                        "Bookmaker update: ignored"
                    );
                }
            }
        }

        info!(fixture_id, bookmakers, kept, "Processed bookmaker updates");

        Ok(())
    }
}
