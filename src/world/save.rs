use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Write, Cursor};
use std::path::Path;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;

use super::chunk::ChunkColumn;

const WORLD_DIR: &str = "world";
const SAVE_FILE: &str = "world/chunks.dat";

pub struct World {
    pub chunks: HashMap<(i32, i32), ChunkColumn>,
}

impl World {
    pub fn new() -> Self {
        Self { chunks: HashMap::new() }
    }

    pub fn load_from_disk(&mut self) -> bool {
        let path = Path::new(SAVE_FILE);
        if !path.exists() {
            return false;
        }

        let compressed = match fs::read(path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("[World] Failed to read {}: {}", SAVE_FILE, e);
                return false;
            }
        };

        let mut decoder = ZlibDecoder::new(Cursor::new(compressed));
        let mut json_bytes = Vec::new();
        if decoder.read_to_end(&mut json_bytes).is_err() {
            eprintln!("[World] Failed to decompress save file.");
            return false;
        }

        let chunks: Vec<ChunkColumn> = match serde_json::from_slice(&json_bytes) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[World] Failed to parse save file: {}", e);
                return false;
            }
        };

        for col in chunks {
            self.chunks.insert((col.x, col.z), col);
        }

        println!("[World] Loaded {} chunks from disk.", self.chunks.len());
        true
    }

    pub fn save_to_disk(&self) -> io::Result<()> {
        fs::create_dir_all(WORLD_DIR)?;

        let all_chunks: Vec<&ChunkColumn> = self.chunks.values().collect();
        let json_bytes = serde_json::to_vec(&all_chunks)?;

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&json_bytes)?;
        let compressed = encoder.finish()?;

        fs::write(SAVE_FILE, compressed)?;
        println!("[World] Saved {} chunks to disk.", self.chunks.len());
        Ok(())
    }

    pub fn get_or_generate(&mut self, cx: i32, cz: i32) -> &ChunkColumn {
        if !self.chunks.contains_key(&(cx, cz)) {
            let col = super::generation::generate_flat_chunk(cx, cz);
            self.chunks.insert((cx, cz), col);
        }
        self.chunks.get(&(cx, cz)).unwrap()
    }
}
