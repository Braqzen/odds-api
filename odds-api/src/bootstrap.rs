use crate::{
    api::{
        snapshot::SnapshotClient,
        types::{Game, MatchPrice, Quote},
    },
    risk::{BookmakerInput, Decision, QuoteInput, Validator},
    state::State,
};
use eyre::Result;
use std::collections::{HashMap, HashSet};
use tokio::{
    select,
    sync::mpsc::{Receiver, Sender},
};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

#[derive(Clone)]
pub struct Bootstrap {
    client: SnapshotClient,
    sports_ids: Vec<u8>,
}

impl Bootstrap {
    pub fn new(rest: &str, api_key: &str, sport_ids: &[u8]) -> Self {
        let client = SnapshotClient::new(rest.to_string(), api_key.to_string());

        Self {
            client,
            sports_ids: sport_ids.to_vec(),
        }
    }

    pub async fn run(
        self,
        mut requests: Receiver<()>,
        states: Sender<State>,
        validator: Validator,
        token: CancellationToken,
    ) {
        self.publish_state(&validator, &states).await;

        loop {
            select! {
                biased;

                _ = token.cancelled() => break,

                Some(()) = requests.recv() => {
                    self.publish_state(&validator, &states).await;
                }
            }
        }
    }

    async fn publish_state(&self, validator: &Validator, states: &Sender<State>) {
        match self.state_snapshot(validator).await {
            Ok(state) => {
                if let Err(error) = states.send(state).await {
                    error!(%error, "Processor state receiver dropped");
                }
            }
            Err(error) => {
                error!(%error, "Bootstrap failed");
            }
        }
    }

    pub async fn state_snapshot(&self, validator: &Validator) -> Result<State> {
        let active_matches = self.active_games(validator).await?;
        let match_odds = self.game_odds(&active_matches).await;

        let (matches, odds, odds_by_fixture) =
            filter_odds(active_matches, match_odds, validator).await?;

        info!(matches = matches.len(), odds = odds.len(), "State size");

        Ok(State::new(matches, odds, odds_by_fixture))
    }

    async fn active_games(&self, validator: &Validator) -> Result<Vec<Game>> {
        let snapshot_matches = self.query_snapshot().await;

        let mut active_matches: Vec<Game> = Vec::new();

        // Filter to only contain: pregame & live games for trading
        for game in snapshot_matches {
            let fixture_id = game.fixture_id.clone();
            match validator.game(game).await? {
                Decision::Enable(game) => {
                    active_matches.push(game);
                }
                Decision::Disable(reason) | Decision::Ignore(reason) => {
                    warn!(reason, fixture_id, "Snapshot game filtered");
                }
            }
        }

        info!(
            matches = active_matches.len(),
            sports = self.sports_ids.len(),
            "Snapshot active games"
        );

        Ok(active_matches)
    }

    async fn game_odds(&self, active_matches: &[Game]) -> Vec<MatchPrice> {
        let fixture_ids: Vec<String> = active_matches
            .iter()
            .map(|fixture| fixture.fixture_id.clone())
            .collect();

        let match_odds = self.query_odds(&fixture_ids).await;

        info!(
            received = match_odds.len(),
            matches = fixture_ids.len(),
            "Snapshot active game odds"
        );

        match_odds
    }

    async fn query_odds(&self, fixture_ids: &[String]) -> Vec<MatchPrice> {
        let mut match_odds: Vec<MatchPrice> = Vec::new();
        let chunk_size = 200;
        let mut start = 0;

        while start < fixture_ids.len() {
            let end = (start + chunk_size).min(fixture_ids.len());
            let chunk = &fixture_ids[start..end];
            match self.client.odds(chunk).await {
                Ok(data) => {
                    match_odds.extend(data);
                }
                Err(error) => {
                    error!(start, end, %error, "Querying snapshot odds failed");
                }
            }
            start = end;
        }

        match_odds
    }

    async fn query_snapshot(&self) -> Vec<Game> {
        let mut matches: Vec<Game> = Vec::new();

        for id in &self.sports_ids {
            match self.client.fixtures(*id).await {
                Ok(fixtures) => {
                    matches.extend(fixtures);
                }
                Err(error) => {
                    error!(id, %error, "Querying snapshot sport failed");
                }
            };
        }

        matches
    }
}

async fn filter_odds(
    active_matches: Vec<Game>,
    match_odds: Vec<MatchPrice>,
    validator: &Validator,
) -> Result<(
    HashMap<String, Game>,
    HashMap<String, Quote>,
    HashMap<String, HashSet<String>>,
)> {
    let mut matches = HashMap::new();
    for game in active_matches {
        matches.insert(game.fixture_id.clone(), game);
    }

    let mut odds = HashMap::new();
    let mut odds_by_fixture: HashMap<String, HashSet<String>> = HashMap::new();

    for game in match_odds {
        let exists = matches.contains_key(&game.fixture_id);

        // Since we do not provide a bookmaker arg on boot (similar to sports id)
        // Add every valid bookmaker for each game
        for (bookmaker_id, meta) in game.bookmakers {
            let input =
                BookmakerInput::new(game.fixture_id.clone(), bookmaker_id.clone(), exists, meta);

            match validator.bookmaker(input).await? {
                Decision::Enable(input) => {
                    if let Some(game) = matches.get_mut(&input.fixture_id) {
                        game.bookmakers.insert(input.bookmaker_id, input.meta);
                    }
                }
                Decision::Disable(reason) | Decision::Ignore(reason) => {
                    warn!(
                        reason,
                        fixture_id = game.fixture_id,
                        bookmaker = bookmaker_id,
                        "Snapshot bookmaker filtered"
                    );
                }
            }
        }

        // Now that we have enabled bookmakers we must check the odds and only keep the ones associated
        // with enabled bookmakers
        for bookmaker_odds in game.odds.into_values() {
            for (odds_id, quote) in bookmaker_odds {
                // Filter out quotes by bookmakers that have been disabled in the previous step
                let bookmaker_available = match matches.get(&game.fixture_id) {
                    Some(game) => game.bookmakers.contains_key(&quote.bookmaker),
                    None => false,
                };

                // Time is used to remove older quotes
                let current_changed_at =
                    odds.get(&odds_id).map(|current: &Quote| current.changed_at);

                let input = QuoteInput::new(
                    game.fixture_id.clone(),
                    odds_id.clone(),
                    exists,
                    bookmaker_available,
                    current_changed_at,
                    quote,
                );

                match validator.quote(input).await? {
                    Decision::Enable(input) => {
                        odds_by_fixture
                            .entry(input.fixture_id)
                            .or_default()
                            .insert(input.odds_id.clone());
                        odds.insert(input.odds_id, input.quote);
                    }
                    Decision::Disable(reason) | Decision::Ignore(reason) => {
                        warn!(
                            reason,
                            fixture_id = game.fixture_id,
                            odds_id,
                            "Snapshot quote filtered"
                        );
                    }
                }
            }
        }
    }

    Ok((matches, odds, odds_by_fixture))
}
