mod chunk;
mod cloud;
mod encryption;

use dotenv::dotenv;
use std::env;
use encryption::{generate_key};

fn main() {
    dotenv().ok();
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: cargo run <input_file> <output_dir> [chunk_size]");
        std::process::exit(1);
    }

    let file_path = &args[1];
    let output_dir = &args[2];
    let num_chunks = if args.len() > 3 {
        args[3].parse::<usize>().ok()
    } else {
        None
    };

    let encryption_key = generate_key();
    
    let manifest_path = "./REC/manifest.json";
    let output_file_path = "./REC";
    // let file_path = "/home/sarthak/Desktop/rustPro/re_file/src/test.jpg";
    // let output_dir = "./chunks";

    std::fs::create_dir_all(output_dir).expect("Failed to create output directory");

    match chunk::split_file(file_path, output_dir, num_chunks, &encryption_key) {
        Ok(_) => println!("File successfully split into chunks."),
        Err(e) => eprintln!("Error splitting file: {}", e),
    }

    match chunk::reconstruct_file(manifest_path, output_file_path,  &encryption_key) {
        Ok(_) => println!("File successfully reconstructed."),
        Err(e) => eprintln!("Error reconstructing file: {}", e),
    }
}