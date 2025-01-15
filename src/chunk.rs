use std::fs::{File, OpenOptions};
use serde_json::Value;
use std::path::Path;
use reqwest::blocking::Client;
use std::io::{Read, Write, BufReader};
use serde_json::json;
use crate::cloud::upload_chunk_to_cloud_with_retries;
use crate::encryption::{decrypt, encrypt};
use sha2::{Sha256, Digest};
use base64::{engine::general_purpose::STANDARD, Engine};

pub fn split_file(
    file_path: &str,
    output_dir: &str,
    optional_num_chunks: Option<usize>,
    encryption_key: &[u8],
) -> std::io::Result<()> {
    // Ensure input file exists
    if !Path::new(file_path).exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Input file not found",
        ));
    }

    let mut input_file = File::open(file_path)?;
    let file_size = input_file.metadata()?.len();

    // Handle empty file
    if file_size == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Input file is empty",
        ));
    }

    let num_chunks = optional_num_chunks.unwrap_or(5);
    if num_chunks == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Number of chunks must be greater than 0",
        ));
    }

    let chunk_size = (file_size as f64 / num_chunks as f64).ceil() as usize;

    // Calculate checksum of the entire file
    let mut hasher = Sha256::new();
    let mut file_data = Vec::new();
    input_file.read_to_end(&mut file_data)?;
    hasher.update(&file_data);
    let checksum = format!("{:x}", hasher.finalize());

    input_file = File::open(file_path)?; // Re-open file for reading
    let mut reader = BufReader::new(input_file);
    let mut buffer = vec![0; chunk_size];

    let file_extension = Path::new(file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("txt")
        .to_string();

    let mut manifest_chunks = vec![];

    // Split the file into chunks
    while let Ok(bytes_read) = reader.read(&mut buffer) {
        if bytes_read == 0 {
            break;
        }

        let chunk_data = &buffer[..bytes_read];
        let (encrypted_data, iv) = encrypt(chunk_data, encryption_key)?;

        // Use retry-enabled upload function
        let chunk_url = match upload_chunk_to_cloud_with_retries(&encrypted_data, "chunk_file_name") {
            Ok(url) => url,
            Err(err) => {
                eprintln!("Failed to upload chunk: {}", err);
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to upload chunk",
                ));
            }
        };

        manifest_chunks.push(json!({
            "url": chunk_url,
            "iv": STANDARD.encode(iv), // Encode IV in Base64
        }));
    }

    // Generate manifest with checksum
    let manifest_path = format!("{}/manifest.json", output_dir);
    let manifest_file = OpenOptions::new().create(true).write(true).open(&manifest_path)?;
    let json_manifest = json!({
        "extension": file_extension,
        "chunks": manifest_chunks,
        "checksum": checksum,
    });

    serde_json::to_writer_pretty(manifest_file, &json_manifest)?;

    println!("File split and uploaded successfully. Manifest saved at {}", manifest_path);
    Ok(())
}

pub fn reconstruct_file(
    manifest_path: &str,
    output_dir: &str,
    encryption_key: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let manifest_file = File::open(manifest_path)?;
    let manifest: Value = serde_json::from_reader(manifest_file)?;

    let file_extension = manifest["extension"]
        .as_str()
        .ok_or("Failed to read file extension from manifest")?;
    let original_checksum = manifest["checksum"]
        .as_str()
        .ok_or("Failed to read checksum from manifest")?;
    let chunks = manifest["chunks"]
        .as_array()
        .ok_or("Failed to read chunks from the manifest")?;

    let client = Client::new();
    let reconstructed_file_path = format!("{}/reconstructed_file.{}", output_dir, file_extension);
    let mut output_file = File::create(&reconstructed_file_path)?;

    // Download and decrypt the chunks
    for chunk_info in chunks {
        let url = chunk_info["url"]
            .as_str()
            .ok_or("Invalid URL format in manifest")?;
        let iv = chunk_info["iv"]
            .as_str()
            .ok_or("Missing IV for chunk in manifest")?;
        let iv = STANDARD.decode(iv)?;

        let response = client.get(url).send()?;
        if !response.status().is_success() {
            return Err(format!("Failed to download chunk from URL: {}", url).into());
        }

        let encrypted_chunk = response.bytes()?;
        let decrypted_data = decrypt(&encrypted_chunk, encryption_key, &iv)?;

        output_file.write_all(&decrypted_data)?;
    }

    // Verify checksum
    let mut reconstructed_file = File::open(&reconstructed_file_path)?;
    let mut hasher = Sha256::new();
    let mut buffer = Vec::new();
    reconstructed_file.read_to_end(&mut buffer)?;
    hasher.update(&buffer);
    let reconstructed_checksum = format!("{:x}", hasher.finalize());

    if reconstructed_checksum != original_checksum {
        return Err(format!(
            "Checksum mismatch! Original: {}, Reconstructed: {}",
            original_checksum, reconstructed_checksum
        )
        .into());
    }

    println!("File reconstruction successful. Checksums match.");
    Ok(())
}