use crate::hyperlight::{acquire_sandbox, release_sandbox};
use crate::layer_cache::LayerCache;
use crate::models::package::DpkgRecord;
use crate::models::requests::Layer;
use crate::utils::errors::{HyperlightGuestError, ImagePullError};
use crate::utils::line_reader::LineReader;
use anyhow;
use flate2::read::GzDecoder;
use hyperlight_common::flatbuffer_wrappers::function_types::{
    ParameterValue, ReturnType, ReturnValue,
};
use oci_client::{secrets::RegistryAuth, Client, Reference};
use std::io::Read;
use tar::Archive;
use timed::timed;

#[timed]
pub async fn pull_and_inspect_image(
    oci_client: &Client,
    layer_cache: &LayerCache,
    image_reference: &str,
    hyperlighted: bool,
) -> Result<Vec<Layer>, anyhow::Error> {
    let mut layers = Vec::<Layer>::new();
    let reference = Reference::try_from(image_reference).map_err(|e| ImagePullError {
        message: format!("Invalid image reference: {}", e),
    })?;

    let (manifest, _digest) = oci_client
        .pull_image_manifest(&reference, &RegistryAuth::Anonymous)
        .await
        .map_err(|e| ImagePullError {
            message: format!("Failed to pull image manifest: {}", e),
        })?;

    for layer in manifest.layers {
        let mut layer_buffer: Vec<u8> = Vec::new();

        match layer_cache
            .pull_or_get_cached_blob(&oci_client, &reference, &layer, &mut layer_buffer)
            .await
        {
            Ok(_) => {
                // println!(
                //     "Layer of media type: {}, pulled successfully: {}",
                //     layer.media_type, layer.digest
                // );

                let sandbox = acquire_sandbox().await;
                let packages: Vec<DpkgRecord>;
                {
                    let mut sandbox_guard = sandbox.lock().await;
                    packages = inspect_layer(
                        &layer_buffer,
                        "var/lib/dpkg/status",
                        hyperlighted,
                        &mut sandbox_guard,
                    )
                    .await?;
                }
                release_sandbox(sandbox).await;
                let layer = Layer {
                    layer: layer.digest.to_string(),
                    packages,
                };
                layers.push(layer);
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to pull image manifest: {}", e));
            }
        }
    }
    // println!(
    //     "Image pulled successfully: {}, digest: {}",
    //     image_reference, digest
    // );
    Ok(layers)
}

#[timed]
async fn inspect_layer(
    layer_data: &[u8],
    target_path: &str,
    hyperlighted: bool,
    sandbox_guard: &mut tokio::sync::MutexGuard<'_, hyperlight_host::MultiUseSandbox>,
) -> Result<Vec<DpkgRecord>, anyhow::Error> {
    let gz = GzDecoder::new(layer_data);
    let mut archive = Archive::new(gz);
    let mut packages: Vec<DpkgRecord> = vec![];

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;

        if path.to_string_lossy() == target_path {
            let mut buffer = Vec::new();
            entry.read_to_end(&mut buffer)?;

            match hyperlighted {
                true => {
                    let result = sandbox_guard
                        .call_guest_function_by_name(
                            "Inspect",
                            ReturnType::VecBytes,
                            Some(vec![ParameterValue::VecBytes(buffer)]),
                        )
                        .map_err(|e| {
                            Box::new(HyperlightGuestError {
                                message: format!("Failed to call inspect: {}", e),
                            })
                        });

                    match result {
                        Ok(ReturnValue::String(result)) => {
                            println!("hyperlighted success");
                            packages = serde_json::from_str(&result).unwrap();
                        }
                        Ok(_) => {
                            return Err(anyhow::anyhow!("Invalid return type from inspect"));
                        }
                        Err(e) => return Err(anyhow::anyhow!(e)),
                    }
                }
                false => {
                    let mut reader = LineReader::new(buffer.as_slice());
                    while let Some(package) = read_package(&mut reader)? {
                        packages.push(package);
                    }
                    println!("non-hyperlighted success");
                }
            }
        }
    }

    Ok(packages)
}

fn read_package(reader: &mut LineReader) -> Result<Option<DpkgRecord>, anyhow::Error> {
    let mut package = DpkgRecord::default();

    let mut line = reader.next();
    if line.is_none() {
        return Ok(None);
    }

    loop {
        if let Some(line_str) = line {
            if line_str.is_empty() {
                break;
            }
            let (key, value) = line_str.split_once(':').unwrap_or(("", ""));
            match key {
                "Package" => package.package = value.trim().to_string(),
                "Status" => package.status = value.trim().to_string(),
                "Version" => package.version = value.trim().to_string(),
                _ => {}
            }
        }

        line = reader.next();
        if line.is_none() {
            break;
        }
    }

    Ok(Some(package))
}
