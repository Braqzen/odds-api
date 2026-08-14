use crate::api::types::{BookmakerFixtureMeta, Game, Quote};
use eyre::{Result, eyre};
use tokio::sync::{mpsc, oneshot};

#[derive(Clone)]
pub struct Validator {
    sender: mpsc::Sender<ValidationRequest>,
}

impl Validator {
    pub fn new(sender: mpsc::Sender<ValidationRequest>) -> Self {
        Self { sender }
    }

    pub async fn game(&self, input: Game) -> Result<Decision<Game>> {
        let (respond_to, response) = oneshot::channel();
        self.sender
            .send(ValidationRequest::new_game(input, respond_to))
            .await?;

        response
            .await
            .map_err(|_| eyre!("validation engine dropped response"))
    }

    pub async fn bookmaker(&self, input: BookmakerInput) -> Result<Decision<BookmakerInput>> {
        let (respond_to, response) = oneshot::channel();
        self.sender
            .send(ValidationRequest::new_bookmaker(input, respond_to))
            .await?;

        response
            .await
            .map_err(|_| eyre!("validation engine dropped response"))
    }

    pub async fn quote(&self, input: QuoteInput) -> Result<Decision<QuoteInput>> {
        let (respond_to, response) = oneshot::channel();
        self.sender
            .send(ValidationRequest::new_quote(input, respond_to))
            .await?;

        response
            .await
            .map_err(|_| eyre!("validation engine dropped response"))
    }
}

pub enum ValidationRequest {
    Game {
        input: Game,
        respond_to: oneshot::Sender<Decision<Game>>,
    },
    Bookmaker {
        input: BookmakerInput,
        respond_to: oneshot::Sender<Decision<BookmakerInput>>,
    },
    Quote {
        input: QuoteInput,
        respond_to: oneshot::Sender<Decision<QuoteInput>>,
    },
}

impl ValidationRequest {
    fn new_game(input: Game, respond_to: oneshot::Sender<Decision<Game>>) -> ValidationRequest {
        Self::Game { input, respond_to }
    }

    fn new_bookmaker(
        input: BookmakerInput,
        respond_to: oneshot::Sender<Decision<BookmakerInput>>,
    ) -> ValidationRequest {
        Self::Bookmaker { input, respond_to }
    }

    fn new_quote(
        input: QuoteInput,
        respond_to: oneshot::Sender<Decision<QuoteInput>>,
    ) -> ValidationRequest {
        Self::Quote { input, respond_to }
    }
}

pub enum Decision<T> {
    Enable(T),
    Disable(String),
    Ignore(String),
}

pub struct BookmakerInput {
    pub fixture_id: String,
    pub bookmaker_id: String,
    pub fixture_exists: bool,
    pub meta: BookmakerFixtureMeta,
}

impl BookmakerInput {
    pub fn new(
        fixture_id: String,
        bookmaker_id: String,
        fixture_exists: bool,
        meta: BookmakerFixtureMeta,
    ) -> Self {
        Self {
            fixture_id,
            bookmaker_id,
            fixture_exists,
            meta,
        }
    }
}

pub struct QuoteInput {
    pub fixture_id: String,
    pub odds_id: String,
    pub fixture_exists: bool,
    pub bookmaker_available: bool,
    pub current_changed_at: Option<i64>,
    pub quote: Quote,
}

impl QuoteInput {
    pub fn new(
        fixture_id: String,
        odds_id: String,
        fixture_exists: bool,
        bookmaker_available: bool,
        current_changed_at: Option<i64>,
        quote: Quote,
    ) -> Self {
        Self {
            fixture_id,
            odds_id,
            fixture_exists,
            bookmaker_available,
            current_changed_at,
            quote,
        }
    }
}
