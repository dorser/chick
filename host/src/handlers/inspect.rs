use axum::{http::StatusCode, Json};
use std::sync::Arc;

use crate::services::inspect::pull_and_inspect_image;
use crate::models::requests::{ErrorResponse, InspectRequest, InspectResponse};
use crate::models::state::AppState;

pub async fn inspect_handler(
    Json(payload): Json<InspectRequest>,
    state: Arc<AppState>,
) -> Result<Json<InspectResponse>, (StatusCode, Json<ErrorResponse>)> {
    let oci_client = &state.oci_client;

    match pull_and_inspect_image(oci_client, &payload.image).await {
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
