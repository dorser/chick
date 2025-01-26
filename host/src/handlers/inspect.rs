use axum::{extract::State, http::StatusCode, Json};
use std::sync::Arc;

use crate::models::requests::{ErrorResponse, InspectRequest, InspectResponse};
use crate::models::state::AppState;
use crate::services::inspect::pull_and_inspect_image;

pub async fn inspect_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<InspectRequest>,
) -> Result<Json<InspectResponse>, (StatusCode, Json<ErrorResponse>)> {
    let layer_cache = state.layer_cache.lock().await;
    match pull_and_inspect_image(&state.oci_client, &layer_cache, &payload.image).await {
        Ok(layers) => Ok(Json(InspectResponse {
            image: payload.image,
            layers,
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )),
    }
}
