use anyhow;
use oci_client::{manifest::OciDescriptor, Client, Reference};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub struct LayerCache {
    cache_dir: PathBuf,
}

impl LayerCache {
    pub fn new(base_path: &Path) -> Self {
        let cache_dir = base_path.join("layer_cache");
        fs::create_dir_all(&cache_dir).expect("Failed to create layer cache directory");

        Self { cache_dir }
    }

    fn get_layer_path(&self, layer_digest: &str) -> PathBuf {
        self.cache_dir.join(layer_digest.replace(':', "_"))
    }

    pub async fn pull_or_get_cached_blob(
        &self,
        oci_client: &Client,
        reference: &Reference,
        layer: &OciDescriptor,
        buffer: &mut Vec<u8>,
    ) -> Result<bool, anyhow::Error> {
        let layer_path = self.get_layer_path(&layer.digest);

        if layer_path.exists() {
            let mut cached_file = File::open(&layer_path)?;
            buffer.clear();
            cached_file.read_to_end(buffer)?;
            return Ok(true);
        }

        // Pull blob if not cached
        buffer.clear();
        match oci_client.pull_blob(reference, layer, &mut *buffer).await {
            Ok(_) => {
                let mut cached_file = File::create(&layer_path)?;
                cached_file.write_all(buffer.as_slice())?;
            }
            Err(e) => {
                return Err(anyhow::anyhow!(e));
            }
        }

        Ok(false)
    }
}
