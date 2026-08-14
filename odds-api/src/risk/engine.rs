use crate::{
    api::types::{Game, Quote},
    risk::validator::{BookmakerInput, Decision, QuoteInput, ValidationRequest},
};
use tokio::{select, sync::mpsc::Receiver};
use tokio_util::sync::CancellationToken;
use tracing::warn;

pub struct Engine {
    sport_ids: Vec<u8>,
}

impl Engine {
    pub fn new(sport_ids: Vec<u8>) -> Self {
        Self { sport_ids }
    }

    pub async fn run(self, mut receiver: Receiver<ValidationRequest>, token: CancellationToken) {
        loop {
            select! {
                biased;

                _ = token.cancelled() => break,

                request = receiver.recv() => {
                    let Some(request) = request else {
                        break;
                    };

                    match request {
                        ValidationRequest::Game { input, respond_to } => {
                            if respond_to.send(self.validate_game(input)).is_err() {
                                warn!("fixture validation response receiver dropped");
                            }
                        }
                        ValidationRequest::Bookmaker { input, respond_to } => {
                            if respond_to.send(self.validate_bookmaker(input)).is_err() {
                                warn!("bookmaker validation response receiver dropped");
                            }
                        }
                        ValidationRequest::Quote { input, respond_to } => {
                            if respond_to.send(self.validate_quote(input)).is_err() {
                                warn!("quote validation response receiver dropped");
                            }
                        }
                    }
                }
            }
        }
    }

    fn validate_game(&self, game: Game) -> Decision<Game> {
        // The only allowed sports are the ones configured on boot
        if !self
            .sport_ids
            .iter()
            .any(|id| i32::from(*id) == game.sport.sport_id)
        {
            return Decision::Ignore("sportId not allowed".to_string());
        }

        // Games are limited to pregame and live, cannot trade finished games
        match game.status.status_id {
            Some(0) | Some(1) => Decision::Enable(game),
            _ => Decision::Disable("Game status ID is not tradable".to_string()),
        }
    }

    fn validate_bookmaker(&self, bookmaker: BookmakerInput) -> Decision<BookmakerInput> {
        if !bookmaker.fixture_exists {
            return Decision::Ignore("fixtureId missing from matches".to_string());
        }
        if !bookmaker.meta.has_odds {
            return Decision::Disable("hasOdds false".to_string());
        }
        if bookmaker.meta.stale_odds {
            return Decision::Disable("staleOdds true".to_string());
        }
        if bookmaker.meta.suspended {
            return Decision::Disable("suspended true".to_string());
        }
        if bookmaker.meta.participants_rotated {
            return Decision::Disable("participantsRotated unhandled".to_string());
        }

        Decision::Enable(bookmaker)
    }

    fn validate_quote(&self, quote: QuoteInput) -> Decision<QuoteInput> {
        if !quote.fixture_exists {
            return Decision::Ignore("fixtureId missing from matches".to_string());
        }

        if !quote.bookmaker_available {
            return Decision::Ignore("bookmaker meta missing".to_string());
        }

        if quote
            .current_changed_at
            .is_some_and(|changed_at| quote.quote.changed_at < changed_at)
        {
            return Decision::Ignore("changedAt older than stored".to_string());
        }

        if let Some(reason) =
            invalid_odds_id_reason(&quote.fixture_id, &quote.odds_id, &quote.quote)
        {
            return Decision::Ignore(reason);
        }

        if !quote.quote.price.is_finite() || quote.quote.price == 0.0 {
            return Decision::Ignore("price invalid".to_string());
        }

        if !quote.quote.active {
            return Decision::Disable("active false".to_string());
        }

        match quote.quote.market_active {
            Some(true) => Decision::Enable(quote),
            Some(false) => Decision::Disable("marketActive false".to_string()),
            None => Decision::Disable("marketActive missing".to_string()),
        }
    }
}

fn invalid_odds_id_reason(fixture_id: &str, odds_id: &str, quote: &Quote) -> Option<String> {
    let mut parts = Vec::new();
    for part in odds_id.split(':') {
        parts.push(part);
    }

    if parts.len() != 4 {
        return Some("oddsId format".to_string());
    }

    if parts[0] != fixture_id {
        return Some("oddsId fixtureId mismatch".to_string());
    }

    if parts[1] != quote.bookmaker {
        return Some("oddsId bookmaker mismatch".to_string());
    }

    let outcome_id = match parts[2].parse::<i64>() {
        Ok(value) => value,
        Err(_) => return Some("oddsId outcomeId invalid".to_string()),
    };

    let player_id = match parts[3].parse::<i64>() {
        Ok(value) => value,
        Err(_) => return Some("oddsId playerId invalid".to_string()),
    };

    if outcome_id != quote.outcome_id {
        return Some("oddsId outcomeId mismatch".to_string());
    }

    if player_id != quote.player_id {
        return Some("oddsId playerId mismatch".to_string());
    }

    None
}
