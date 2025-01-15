use std::env;
use reqwest::blocking::{Client, multipart::Part};
use reqwest::blocking::multipart::Form;
use backoff::{ExponentialBackoff, Error as BackoffError, retry};
use std::sync::Arc;

pub fn upload_chunk_to_cloud_with_retries(
    chunk_data: &[u8],  
    file_name: &str,    
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let file_name_owned = file_name.to_owned();

    let retry_operation = || -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let client = Client::new();

        let cloud_name = env::var("CLOUDINARY_CLOUD_NAME")?;
        let api_key = env::var("CLOUDINARY_API_KEY")?;
        let api_secret = env::var("CLOUDINARY_API_SECRET")?;
        let upload_preset = env::var("CLOUDINARY_UPLOAD_PRESET")?;
        let url = format!("https://api.cloudinary.com/v1_1/{}/upload", cloud_name);

        // Construct the form with chunk data
        let form = Form::new()
            .text("upload_preset", upload_preset)
            .text("api_key", api_key)
            .text("api_secret", api_secret)
            .text("resource_type", "raw")
            .part(
                "file",
                Part::bytes(chunk_data.to_vec()).file_name(file_name_owned.clone()),
            );

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


// pub fn parallel_upload_chunks(
//     chunks: Vec<(Vec<u8>, String)>, // Each chunk is a tuple of data and its file name
// ) -> Vec<Result<String, Box<dyn std::error::Error + Send + Sync>>> {
//     chunks
//         .par_iter()
//         .map(|(chunk_data, file_name)| upload_chunk_to_cloud_with_retries(chunk_data, file_name))
//         .collect()
// }