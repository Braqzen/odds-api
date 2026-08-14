use crate::{
    api::stream::WsClient,
    bootstrap::Bootstrap,
    processor::Processor,
    risk::{Engine, Validator},
};
use eyre::Result;
use tokio::signal::unix::{SignalKind, signal};
use tokio::{select, sync::mpsc};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

pub struct Worker {
    ws_client: WsClient,
    bootstrap: Bootstrap,
    sport_ids: Vec<u8>,
}

impl Worker {
    pub fn new(rest: String, ws: String, api_key: String, sport_ids: Vec<u8>) -> Self {
        let ws_client = WsClient::new(&ws, &api_key, &sport_ids);
        let bootstrap = Bootstrap::new(&rest, &api_key, &sport_ids);

        Self {
            ws_client,
            bootstrap,
            sport_ids,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        // Handle running locally and interrupting the process with ctrl+c.
        let mut sigint = signal(SignalKind::interrupt())?;
        // Handle running in a container and terminating the process with docker stop.
        let mut sigterm = signal(SignalKind::terminate())?;

        // Token used to stop background tasks
        let token = CancellationToken::new();

        let engine_token = token.child_token();
        let bootstrap_token = token.child_token();
        let ws_token = token.child_token();
        let processor_token = token.child_token();

        let engine_shutdown = token.clone();
        let bootstrap_shutdown = token.clone();
        let ws_shutdown = token.clone();
        let processor_shutdown = token.clone();

        let (validation_sender, validation_receiver) = mpsc::channel(1024);
        let (snapshot_sender, snapshot_receiver) = mpsc::channel(1);
        let (state_sender, state_receiver) = mpsc::channel(1);
        let (ws_sender, ws_receiver) = mpsc::channel(1024);

        let engine = Engine::new(self.sport_ids.clone());
        let validator = Validator::new(validation_sender);
        let processor = Processor::new(validator.clone(), snapshot_sender);

        let bootstrap = self.bootstrap.clone();
        let ws_client = self.ws_client.clone();

        let engine_task = tokio::spawn(async move {
            let _shutdown_guard = engine_shutdown.drop_guard();
            engine.run(validation_receiver, engine_token).await;
        });

        let bootstrap_task = tokio::spawn(async move {
            let _shutdown_guard = bootstrap_shutdown.drop_guard();
            bootstrap
                .run(snapshot_receiver, state_sender, validator, bootstrap_token)
                .await;
        });

        let processor_task = tokio::spawn(async move {
            let _shutdown_guard = processor_shutdown.drop_guard();
            processor
                .run(ws_receiver, state_receiver, processor_token)
                .await;
        });

        let ws_task = tokio::spawn(async move {
            let _shutdown_guard = ws_shutdown.drop_guard();
            ws_client.run(ws_sender, ws_token).await
        });

        select! {
            _ = sigint.recv() => info!("Received interrupt signal"),
            _ = sigterm.recv() => info!("Received terminate signal"),
            _ = token.cancelled() => info!("Background task stopped"),
        }

        token.cancel();

        match ws_task.await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => error!(%error, "websocket task failed"),
            Err(error) => error!(%error, "websocket task join failed"),
        }
        match processor_task.await {
            Ok(()) => {}
            Err(error) => error!(%error, "processor task join failed"),
        }
        match bootstrap_task.await {
            Ok(()) => {}
            Err(error) => error!(%error, "bootstrap task join failed"),
        }
        match engine_task.await {
            Ok(()) => {}
            Err(error) => error!(%error, "validation engine task join failed"),
        }

        Ok(())
    }
}
