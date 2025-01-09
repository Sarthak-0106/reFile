use std::fs::{File, OpenOptions};
use serde_json::Value;
use std::path::Path;
use reqwest::blocking::Client;
use std::io::{Read, Write, BufReader};
use serde_json::json;
use crate::cloud::upload_chunk_to_cloud;
use crate::encryption::{decrypt, encrypt};
use sha2::{Sha256, Digest};


pub fn split_file(file_path: &str, output_dir: &str, optional_num_chunks: Option<usize>, encryption_key: &[u8]) -> std::io::Result<()> {
    if !Path::new(file_path).exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Input file not found",
        ));
    }

    let mut input_file = File::open(file_path)?;
    let file_size = input_file.metadata()?.len();

    // Check if the file is empty
    if file_size == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Input file is empty",
        ));
    }

    let mut hasher = Sha256::new();
    let mut buffer = Vec::new();
    input_file.read_to_end(&mut buffer)?;
    hasher.update(&buffer);
    let checksum = format!("{:x}", hasher.finalize());

    input_file = File::open(file_path)?;

    let num_chunks = optional_num_chunks.unwrap_or(5); // Default to 5 chunks
    if num_chunks <= 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Number of chunks must be greater than 0",
        ));
    }

    // Ensure chunk size is at least 1 byte
    let chunk_size = (file_size as f64 / num_chunks as f64).ceil() as usize;
    if chunk_size < 1 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Chunk size must be at least 1 byte",
        ));
    }
    println!("Optional Provided Chunk Size: {:?}", num_chunks);

    let mut reader = BufReader::new(input_file);
    let mut buffer = vec![0; chunk_size];
    let mut chunk_count = 0;

    let mut manifest = vec![];

    let file_extension = Path::new(file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("txt") // Default to txt if no extension
        .to_string();

    while let Ok(bytes_read) = reader.read(&mut buffer) {
        if bytes_read == 0 {
            break;
        }

        let chunk_data = &buffer[..bytes_read];
        let (encrypted_data, iv) = encrypt(chunk_data, encryption_key)?;

        let chunk_filename = format!("{}/chunk_{}.txt", output_dir, chunk_count);
        let mut chunk_file = File::create(&chunk_filename)?;
        chunk_file.write_all(&iv)?;  // Store the IV at the beginning of the chunk file
        chunk_file.write_all(&encrypted_data)?;

        // Upload to cloud
        match upload_chunk_to_cloud(&chunk_filename) {
            Ok(url) => {
                manifest.push(url);
            },
            Err(e) => eprintln!("Error uploading chunk: {}", e),
        }

        chunk_count += 1;
    }

    // Write the manifest to a file
    let manifest_path = format!("{}/manifest.json", output_dir);
    let manifest_file = OpenOptions::new().create(true).write(true).open(manifest_path)?;
    let json_manifest = json!({ 
        "extension": file_extension,
        "chunks": manifest,
        "checksum": checksum,
    });
    serde_json::to_writer_pretty(manifest_file, &json_manifest)?;
    Ok(())
}


pub fn reconstruct_file(manifest_path: &str, output_dir: &str, encryption_key: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let manifest_file = File::open(manifest_path)?;
    let manifest: Value = serde_json::from_reader(manifest_file)?;

    // Extract file extension
    let file_extension = manifest["extension"]
        .as_str()
        .ok_or("Failed to read file extension from manifest")?;

    let original_checksum = manifest["checksum"]
        .as_str()
        .ok_or("Failed to read checksum from manifest")?;

    // Extract chunk URLs
    let chunk_urls = manifest["chunks"]
        .as_array()
        .ok_or("Failed to read chunks from the manifest")?;

    let client = Client::new();

    // Define the output file path with the correct extension
    let reconstructed_file_path = format!("{}/reconstructed_file.{}", output_dir, file_extension);

    let mut output_file = File::create(&reconstructed_file_path)?;

    // Download and write each chunk in order
    for (i, chunk_url) in chunk_urls.iter().enumerate() {
        let url = chunk_url.as_str().ok_or("Invalid URL format in manifest")?;

        // Download the chunk
        let chunk_data = client.get(url).send()?.bytes()?;
        println!("Downloaded chunk {}: {} bytes", i, chunk_data.len());

        // Split the data into IV and encrypted chunk
        let (iv, encrypted_chunk) = chunk_data.split_at(16); // Assuming 16 bytes for IV

        // Decrypt the chunk
        let decrypted_data = decrypt(encrypted_chunk, encryption_key, iv)?;

        // Write the chunk to the output file
        output_file.write_all(&decrypted_data)?;
    }

    // Verify checksum of the reconstructed file
    let mut reconstructed_file = File::open(&reconstructed_file_path)?;
    let mut hasher = Sha256::new();
    let mut buffer = Vec::new();
    reconstructed_file.read_to_end(&mut buffer)?;
    hasher.update(&buffer);
    let reconstructed_checksum = format!("{:x}", hasher.finalize());

    if reconstructed_checksum == original_checksum {
        println!("File reconstruction successful. Checksums match.");
    } else {
        return Err(format!(
            "Checksum mismatch! Original: {}, Reconstructed: {}",
            original_checksum, reconstructed_checksum
        )
        .into());
    }
    
    println!("File reconstruction complete: {}", reconstructed_file_path);

    Ok(())
}

