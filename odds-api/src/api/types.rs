use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub fixture_id: String,
    pub status: Status,
    pub sport: SportRef,
    pub tournament: TournamentRef,
    pub season: SeasonRef,
    pub venue: VenueRef,
    pub start_time: i64,
    pub true_start_time: Option<String>,
    pub true_end_time: Option<String>,
    pub participants: ParticipantsRef,
    pub scores: HashMap<String, ScorePeriod>,
    pub clock: Option<Clock>,
    pub expected_periods: Option<i32>,
    pub period_length: Option<i32>,
    pub external_providers: HashMap<String, Value>,
    #[serde(default)]
    pub bookmakers: HashMap<String, BookmakerFixtureMeta>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub live: bool,
    pub status_id: Option<i32>,
    pub status_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SportRef {
    pub sport_id: i32,
    pub sport_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TournamentRef {
    pub tournament_id: i32,
    pub tournament_name: String,
    pub category_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeasonRef {
    pub season_id: Option<i64>,
    pub season_name: Option<String>,
    pub season_round: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VenueRef {
    pub venue_id: Option<i64>,
    pub venue_name: Option<String>,
    pub venue_location: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticipantsRef {
    pub participant1_id: i64,
    pub participant1_rot_nr: Option<i64>,
    pub participant1_name: Option<String>,
    pub participant1_short_name: Option<String>,
    pub participant1_abbr: Option<String>,
    pub participant2_id: i64,
    pub participant2_rot_nr: Option<i64>,
    pub participant2_name: Option<String>,
    pub participant2_short_name: Option<String>,
    pub participant2_abbr: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScorePeriod {
    pub period: String,
    pub participant1_score: i32,
    pub participant2_score: i32,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Clock {
    pub current_period: Option<String>,
    pub current_time: Option<String>,
    pub remaining_time: Option<String>,
    pub remaining_time_in_period: Option<String>,
    pub stopped: Option<bool>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookmakerFixtureMeta {
    pub bookmaker: String,
    pub bookmaker_fixture_id: Option<String>,
    pub fixture_path: Option<String>,
    pub has_odds: bool,
    pub stale_odds: bool,
    pub stale_odds_response_code: Option<i32>,
    pub suspended: bool,
    pub participants_rotated: bool,
    pub meta: Option<Value>,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchPrice {
    pub fixture_id: String,
    pub status: Status,
    pub sport: SportRef,
    pub tournament: TournamentRef,
    pub season: SeasonRef,
    pub venue: VenueRef,
    pub start_time: i64,
    pub true_start_time: Option<String>,
    pub true_end_time: Option<String>,
    pub participants: ParticipantsRef,
    pub scores: HashMap<String, ScorePeriod>,
    pub clock: Option<Clock>,
    pub expected_periods: Option<i32>,
    pub period_length: Option<i32>,
    pub external_providers: HashMap<String, Value>,
    #[serde(default)]
    pub bookmakers: HashMap<String, BookmakerFixtureMeta>,
    #[serde(default)]
    pub odds: HashMap<String, HashMap<String, Quote>>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Quote {
    pub bookmaker: String,
    pub outcome_id: i64,
    pub player_id: i64,
    pub price: f64,
    pub active: bool,
    pub market_active: Option<bool>,
    pub main_line: Option<bool>,
    pub market_id: Option<i64>,
    pub bookmaker_market_id: Option<String>,
    pub bookmaker_outcome_id: Option<String>,
    pub bookmaker_changed_at: Option<i64>,
    pub price_fractional: Option<String>,
    pub price_american: Option<i64>,
    pub meta: Option<Value>,
    pub limit: Option<f64>,
    pub betslip: Option<String>,
    pub changed_at: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OddsPayload {
    pub fixture_id: String,
    pub odds: HashMap<String, HashMap<String, Quote>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookmakersPayload {
    pub fixture_id: String,
    pub bookmakers: HashMap<String, BookmakerFixtureMeta>,
}
