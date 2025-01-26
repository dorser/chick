use axum::{routing::post, Router};
use hyperlight::warm_up_pool;
use std::sync::Arc;
mod handlers;
mod hyperlight;
mod layer_cache;
mod models;
mod services;
mod utils;

use crate::handlers::inspect::inspect_handler;
use crate::layer_cache::LayerCache;
use crate::models::state::AppState;
use anyhow;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    warm_up_pool().await;
    let config = oci_client::client::ClientConfig::default();
    let oci_client = Arc::new(oci_client::Client::new(config));
    let layer_cache = Arc::new(Mutex::new(LayerCache::new(std::path::Path::new(
        "/tmp/oci_cache",
    ))));
    let app_state = Arc::new(AppState {
        oci_client,
        layer_cache,
    });

    let app = Router::new()
        .route("/inspect", post(inspect_handler))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}
