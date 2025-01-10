use std::fs::{File, OpenOptions};
use serde_json::Value;
use std::path::Path;
use reqwest::blocking::Client;
use std::io::{Read, Write, BufReader};
use serde_json::json;
use crate::cloud::parallel_upload_chunks;
use crate::encryption::{decrypt, encrypt};
use sha2::{Sha256, Digest};

pub fn split_file(
    file_path: &str,
    output_dir: &str,
    optional_num_chunks: Option<usize>,
    encryption_key: &[u8],
) -> std::io::Result<()> {
    if !Path::new(file_path).exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Input file not found",
        ));
    }

    let mut input_file = File::open(file_path)?;
    let file_size = input_file.metadata()?.len();

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
    if chunk_size < 1 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Chunk size must be at least 1 byte",
        ));
    }

    // Calculate checksum of the entire file
    let mut hasher = Sha256::new();
    let mut buffer = Vec::new();
    input_file.read_to_end(&mut buffer)?;
    hasher.update(&buffer);
    let checksum = format!("{:x}", hasher.finalize());

    input_file = File::open(file_path)?;
    let mut reader = BufReader::new(input_file);
    let mut buffer = vec![0; chunk_size];
    let mut chunk_paths = vec![];

    let file_extension = Path::new(file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("txt")
        .to_string();

    let mut chunk_count = 0;

    // Split the file into chunks
    while let Ok(bytes_read) = reader.read(&mut buffer) {
        if bytes_read == 0 {
            break;
        }

        let chunk_data = &buffer[..bytes_read];
        
        let (encrypted_data, iv) = encrypt(chunk_data, encryption_key)?;

        let mut chunk_with_iv = Vec::new();
        chunk_with_iv.extend_from_slice(&iv);
        chunk_with_iv.extend_from_slice(&encrypted_data);

        let chunk_filename = format!("{}/chunk_{}.txt", output_dir, chunk_count);
        let mut chunk_file = File::create(&chunk_filename)?;
        chunk_file.write_all(&chunk_with_iv)?;

        chunk_paths.push(chunk_filename);
        chunk_count += 1;
    }

    // Parallel upload chunks
    let upload_results = parallel_upload_chunks(chunk_paths);

    // Generate manifest with checksum
    let mut manifest = vec![];
    for (i, result) in upload_results.iter().enumerate() {
        match result {
            Ok(url) => manifest.push(url.clone()),
            Err(e) => eprintln!("Error uploading chunk {}: {}", i, e),
        }
    }

    // Save manifest to file
    let manifest_path = format!("{}/manifest.json", output_dir);
    let manifest_file = OpenOptions::new().create(true).write(true).open(&manifest_path)?;
    let json_manifest = json!({
        "extension": file_extension,
        "chunks": manifest,
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
    let chunk_urls = manifest["chunks"]
        .as_array()
        .ok_or("Failed to read chunks from the manifest")?;

    let client = Client::new();
    let reconstructed_file_path = format!("{}/reconstructed_file.{}", output_dir, file_extension);
    let mut output_file = File::create(&reconstructed_file_path)?;

    // Download and decrypt the chunks
    for (_i, chunk_url) in chunk_urls.iter().enumerate() {
        let url = chunk_url.as_str().ok_or("Invalid URL format in manifest")?;
        let chunk_data = client.get(url).send()?.bytes()?;

        let (iv, encrypted_chunk) = chunk_data.split_at(16);

        let decrypted_data = decrypt(encrypted_chunk, encryption_key, iv)?;

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
