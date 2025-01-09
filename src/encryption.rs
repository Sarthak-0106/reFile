use aes::{Aes256};
use block_modes::{BlockMode, Cbc};
use block_modes::block_padding::Pkcs7;
use rand::Rng;
use std::io::{self};

type Aes256Cbc = Cbc<Aes256, Pkcs7>;

const IV_SIZE: usize = 16;

// Generate a random encryption key
pub fn generate_key() -> Vec<u8> {
    rand::thread_rng().gen::<[u8; 32]>().to_vec()
}

// Encrypt data
pub fn encrypt(data: &[u8], key: &[u8]) -> io::Result<(Vec<u8>, Vec<u8>)> {
    let iv = rand::thread_rng().gen::<[u8; IV_SIZE]>().to_vec();
    let cipher = Aes256Cbc::new_from_slices(key, &iv).expect("Invalid key or IV");
    let encrypted_data = cipher.encrypt_vec(data);
    Ok((encrypted_data, iv))
}

// Decrypt data
pub fn decrypt(data: &[u8], key: &[u8], iv: &[u8]) -> io::Result<Vec<u8>> {
    let cipher = Aes256Cbc::new_from_slices(key, iv).expect("Invalid key or IV");
    let decrypted_data = cipher.decrypt_vec(data).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    Ok(decrypted_data)
}
