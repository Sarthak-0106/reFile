use std::env;
use std::io::Read;
use reqwest::blocking::{Client, multipart::Form};
use serde_json;
use backoff::{ExponentialBackoff, Error as BackoffError, retry};
use rayon::prelude::*;
use std::sync::Arc;

pub fn upload_chunk_to_cloud_with_retries(
    chunk_path: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let retry_operation = || -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let client = Client::new();
        let mut file = std::fs::File::open(chunk_path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        let cloud_name = env::var("CLOUDINARY_CLOUD_NAME")?;
        let api_key = env::var("CLOUDINARY_API_KEY")?;
        let api_secret = env::var("CLOUDINARY_API_SECRET")?;
        let upload_preset = env::var("CLOUDINARY_UPLOAD_PRESET")?;
        let url = format!("https://api.cloudinary.com/v1_1/{}/upload", cloud_name);

        let form = Form::new()
            .text("upload_preset", upload_preset)
            .text("api_key", api_key)
            .text("api_secret", api_secret)
            .text("resource_type", "raw")
            .file("file", chunk_path)?;

        let res = client.post(&url).multipart(form).send()?;

        if res.status().is_success() {
            let json: serde_json::Value = res.json()?;
            let url = json["secure_url"].as_str().unwrap_or("").to_string();
            Ok(url)
        } else {
            let status = res.status();
            let body = res.text()?;
            Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Error uploading file: {} - {}", status, body))))
        }
    };

    let backoff = ExponentialBackoff::default();
    retry(backoff, || Ok(retry_operation()))
        .map_err(|e| match e {
            BackoffError::Transient { err, .. } => {
                Arc::try_unwrap(err).unwrap_or_else(|e: Arc<Box<(dyn std::error::Error + Send + Sync)>>| (*e).to_string().into())
            }
            BackoffError::Permanent(err) => {
                Arc::try_unwrap(err).unwrap_or_else(|e: Arc<Box<(dyn std::error::Error + Send + Sync)>>| (*e).to_string().into())
            }
        })?
}

pub fn parallel_upload_chunks(
    chunk_paths: Vec<String>,
) -> Vec<Result<String, Box<dyn std::error::Error + Send + Sync>>> {
    chunk_paths
        .par_iter()
        .map(|chunk_path| upload_chunk_to_cloud_with_retries(chunk_path))
        .collect()
}
