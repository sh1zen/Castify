//! String utility functions

use brotli::{CompressorWriter, Decompressor};
use std::io;
use std::io::{Read, Write};

/// Capitalize the first letter of a string
pub fn capitalize_first_letter(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Compress a string using Brotli
pub fn compress_string(data: &str) -> io::Result<Vec<u8>> {
    let mut compressed_data = Vec::new();
    {
        let mut compressor = CompressorWriter::new(&mut compressed_data, 4096, 11, 22);
        compressor.write_all(data.as_bytes())?;
    }
    Ok(compressed_data)
}

/// Decompress a Brotli-compressed string
pub fn decompress_string(compressed_data: &[u8]) -> io::Result<String> {
    let mut decompressor = Decompressor::new(compressed_data, 4096);
    let mut decompressed_data = String::new();
    decompressor.read_to_string(&mut decompressed_data)?;
    Ok(decompressed_data)
}

/// Extract substring after a character
pub fn get_string_after(s: String, c: char) -> String {
    let index = s.find(c);
    if index.is_none() {
        return s;
    }
    s.clone().split_off(index.unwrap() + 1)
}

/// Format seconds as HH:MM:SS
pub fn format_seconds(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;

    format!("{}:{:02}:{:02}", hours, minutes, seconds)
}
