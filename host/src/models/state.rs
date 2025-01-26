use crate::layer_cache::LayerCache;
use oci_client::Client;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AppState {
    pub oci_client: Arc<Client>,
    pub layer_cache: Arc<Mutex<LayerCache>>,
}
