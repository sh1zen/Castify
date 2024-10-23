use brotli::{CompressorWriter, Decompressor};
use std::io;
use std::io::{Read, Write};

pub fn capitalize_first_letter(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

pub fn compress_string(data: &str) -> io::Result<Vec<u8>> {
    let mut compressed_data = Vec::new();
    {
        let mut compressor = CompressorWriter::new(&mut compressed_data, 4096, 11, 22);
        compressor.write_all(data.as_bytes())?;
    }
    Ok(compressed_data)
}

pub fn decompress_string(compressed_data: &[u8]) -> io::Result<String> {
    let mut decompressor = Decompressor::new(compressed_data, 4096);
    let mut decompressed_data = String::new();
    decompressor.read_to_string(&mut decompressed_data)?;
    Ok(decompressed_data)
}