
# RustPro Project

This project is a Rust-based application designed to demonstrate file handling, encryption, cloud upload, and checksum verification. It showcases various Rust programming concepts, including file manipulation, encryption, and interacting with external services like Cloudinary.

## Project Structure

- `src/`: Contains the source code for the Rust application.
  - `chunk.rs`: Handles file splitting, chunking, and reconstruction.
  - `cloud.rs`: Manages interactions with the Cloudinary API to upload and retrieve file chunks.
  - `encryption.rs`: Handles encryption and decryption of file chunks.
  - `main.rs`: The entry point of the application.
  
- `Cargo.toml`: Contains the project dependencies and metadata, such as libraries for HTTP requests (`reqwest`), encryption (`sha2`, `serde_json`), and file I/O operations.
- `readme.txt`: This file, providing an overview of the project and instructions for building and running the project.
  
## Features

- **File Chunking:** Split large files into smaller chunks for easier handling and cloud storage.
- **Encryption:** Encrypt chunks to ensure security and integrity during upload and download.
- **Cloud Upload:** Upload file chunks to Cloudinary for remote storage.
- **Checksum Verification:** Calculate and store checksum values for chunks to ensure data integrity during reconstruction.
- **File Reconstruction:** Download file chunks from Cloudinary and reconstruct the original file with encryption and checksum verification.

## Getting Started

To get started with this project, you need to have Rust installed on your system. You can install Rust by following the instructions on the [official Rust website](https://www.rust-lang.org/).

Additionally, you will need a Cloudinary account to upload files. After creating an account, make sure to set your Cloudinary credentials (Cloud Name, API Key, API Secret, and Upload Preset) as environment variables. Here’s how to set them:

### Environment Variables
```bash
export CLOUDINARY_CLOUD_NAME="your_cloud_name"
export CLOUDINARY_API_KEY="your_api_key"
export CLOUDINARY_API_SECRET="your_api_secret"
export CLOUDINARY_UPLOAD_PRESET="your_upload_preset"
```

## Installing Dependencies

The project uses several Rust dependencies. To install them, navigate to the project directory and run the following command to fetch the dependencies:

```bash
cargo build
```

This will download and compile all the necessary dependencies.

## Building the Project

To build the project, navigate to the project directory and run the following command:

```bash
cargo run <Path to your file> <Output Directory> <Number of Chunks>
```

### Parameters:
- `<Path to your file>`: The file that you want to split and upload.
- `<Output Directory>`: The directory where chunks and the manifest file will be stored.
- `<Number of Chunks>`: The number of chunks you want to split the file into. If not specified, the default is 5.

## Example

For example, to split a file called `large_file.txt` into 8 chunks and store the chunks in the `./chunks` directory:

```bash
cargo run large_file.txt ./chunks 8
```

This will split the `large_file.txt` into 8 chunks, encrypt the chunks, upload them to Cloudinary, and create a `manifest.json` file in the output directory.

## Running the File Reconstruction

To reconstruct the file from the uploaded chunks, you can run the following command, specifying the path to the manifest file:

```bash
cargo run <Path to manifest.json> <Reconstructed File Directory> <Encryption Key>
```

This will download the chunks from Cloudinary, decrypt them, and reconstruct the original file in the specified directory.

### Example:

```bash
cargo run ./chunks/manifest.json ./reconstructed_files my_secret_key
```

This will reconstruct the file from the chunks in the `./chunks` directory, using the encryption key `my_secret_key`, and save the reconstructed file in the `./reconstructed_files` directory.

## Contributing

Feel free to fork this project, open issues, and submit pull requests. Contributions are welcome!
