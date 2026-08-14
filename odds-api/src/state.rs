use crate::api::types::{Game, Quote};
use std::collections::{HashMap, HashSet};

pub struct State {
    pub matches: HashMap<String, Game>,
    pub prices: HashMap<String, Quote>,
    pub odds_by_fixture: HashMap<String, HashSet<String>>,
}

impl State {
    pub fn new(
        matches: HashMap<String, Game>,
        prices: HashMap<String, Quote>,
        odds_by_fixture: HashMap<String, HashSet<String>>,
    ) -> Self {
        Self {
            matches,
            prices,
            odds_by_fixture,
        }
    }

    pub fn remove_fixture(&mut self, fixture_id: &str) -> usize {
        self.matches.remove(fixture_id);
        let Some(odds_ids) = self.odds_by_fixture.remove(fixture_id) else {
            return 0;
        };
        let removed_prices = odds_ids.len();
        for odds_id in odds_ids {
            self.prices.remove(&odds_id);
        }
        removed_prices
    }

    pub fn remove_price(&mut self, fixture_id: &str, odds_id: &str) {
        self.prices.remove(odds_id);
        let empty = match self.odds_by_fixture.get_mut(fixture_id) {
            Some(odds_ids) => {
                odds_ids.remove(odds_id);
                odds_ids.is_empty()
            }
            None => false,
        };
        if empty {
            self.odds_by_fixture.remove(fixture_id);
        }
    }

    pub fn remove_bookmaker_prices(&mut self, fixture_id: &str, bookmaker: &str) {
        let prefix = format!("{fixture_id}:{bookmaker}:");
        let mut to_remove = Vec::new();
        match self.odds_by_fixture.get(fixture_id) {
            Some(odds_ids) => {
                for odds_id in odds_ids {
                    if odds_id.starts_with(&prefix) {
                        to_remove.push(odds_id.clone());
                    }
                }
            }
            None => return,
        }

        for odds_id in to_remove {
            self.prices.remove(&odds_id);
            if let Some(odds_ids) = self.odds_by_fixture.get_mut(fixture_id) {
                odds_ids.remove(&odds_id);
            }
        }

        if self
            .odds_by_fixture
            .get(fixture_id)
            .is_some_and(HashSet::is_empty)
        {
            self.odds_by_fixture.remove(fixture_id);
        }
    }
}
