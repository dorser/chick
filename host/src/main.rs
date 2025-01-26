use std::sync::Arc;
use axum::{Router, routing::post};
use hyperlight::warm_up_pool;
mod handlers;
mod models;
mod services;
mod utils;
mod hyperlight;

use crate::handlers::inspect::inspect_handler;
use crate::models::state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    warm_up_pool().await;
    let config = oci_client::client::ClientConfig::default();
    let oci_client = Arc::new(oci_client::Client::new(config));
    let app_state = Arc::new(AppState { oci_client });

    let app = Router::new()
        .route("/inspect", post({
            let app_state = Arc::clone(&app_state);
            move |payload| inspect_handler(payload, app_state)
        }))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}