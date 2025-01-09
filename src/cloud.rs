use std::env;
use std::io::Read;
use reqwest::blocking::Client;
use reqwest::blocking::multipart::Form;
use serde_json;

pub fn upload_chunk_to_cloud(chunk_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();
    let mut file = std::fs::File::open(chunk_path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    let cloud_name = env::var("CLOUDINARY_CLOUD_NAME").map_err(|_| "Cloudinary cloud name missing")?;
    let api_key = env::var("CLOUDINARY_API_KEY").map_err(|_| "API key missing")?;
    let api_secret = env::var("CLOUDINARY_API_SECRET").map_err(|_| "API secret missing")?;
    let upload_preset = env::var("CLOUDINARY_UPLOAD_PRESET").map_err(|_| "Upload preset missing")?;

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
        let url = json["secure_url"].as_str().unwrap_or("");
        Ok(url.to_string())
    } else {
        let status = res.status();
        let body = res.text()?;
        Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Error uploading file: {} - {}", status, body))))
    }
}