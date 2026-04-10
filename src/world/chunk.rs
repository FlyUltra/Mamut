use bytes::{BufMut, BytesMut};
use crate::protocol::varint::VarInt;
use serde::{Serialize, Deserialize};

pub const CHUNK_SECTIONS: usize = 24;
pub const MIN_Y: i32 = -64;

#[derive(Clone, Serialize, Deserialize)]
pub struct ChunkSection {
    pub block_states: Vec<u16>, // 4096 entries
}

impl ChunkSection {
    pub fn new_air() -> Self {
        Self { block_states: vec![0u16; 4096] }
    }

    pub fn new_filled(block_id: u16) -> Self {
        Self { block_states: vec![block_id; 4096] }
    }

    fn is_single_valued(&self) -> bool {
        let first = self.block_states[0];
        self.block_states.iter().all(|&b| b == first)
    }

    fn non_air_count(&self) -> i16 {
        self.block_states.iter().filter(|&&b| b != 0).count() as i16
    }

    pub fn encode_to_network(&self, buf: &mut BytesMut) {
        // Block count
        buf.put_i16(self.non_air_count());

        // Conservative format: block palette with one value and full fixed-size data array.
        // This is verbose but very robust with strict clients.
        let block_id = self.block_states[0] as i32;
        buf.put_u8(4); // bits per entry
        VarInt(1).encode(buf); // palette len
        VarInt(block_id).encode(buf); // palette[0]
        VarInt(256).encode(buf); // 4096 * 4 / 64
        for _ in 0..256 {
            buf.put_i64(0);
        }

        // Biomes container (single plains value via palette)
        buf.put_u8(1); // bits per entry
        VarInt(1).encode(buf); // palette len
        VarInt(0).encode(buf); // plains
        VarInt(1).encode(buf); // one long for 64 biome entries
        buf.put_i64(0);
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChunkColumn {
    pub x: i32,
    pub z: i32,
    pub sections: Vec<ChunkSection>,
}

impl ChunkColumn {
    pub fn new_empty(x: i32, z: i32) -> Self {
        let sections = (0..CHUNK_SECTIONS).map(|_| ChunkSection::new_air()).collect();
        Self { x, z, sections }
    }

    pub fn set_block(&mut self, bx: u8, y: i32, bz: u8, block_id: u16) {
        let section_idx = ((y - MIN_Y) / 16) as usize;
        let local_y = ((y - MIN_Y) % 16) as usize;
        if section_idx < self.sections.len() {
            let idx = (local_y << 8) | ((bz as usize) << 4) | (bx as usize);
            self.sections[section_idx].block_states[idx] = block_id;
        }
    }

    pub fn encode_sections(&self) -> Vec<u8> {
        let mut buf = BytesMut::new();
        for section in &self.sections {
            section.encode_to_network(&mut buf);
        }
        buf.to_vec()
    }

    pub fn encode_heightmaps_nbt(&self, top_y: i32) -> Vec<u8> {
        let height_value: i64 = (top_y - MIN_Y + 1) as i64;
        let bits_per_entry = 9;
        let entries_per_long = 64 / bits_per_entry; // 7
        let total_longs = (256 + entries_per_long - 1) / entries_per_long; // 37

        let mut longs = vec![0i64; total_longs];
        for i in 0..256 {
            let long_idx = i / entries_per_long;
            let bit_idx = (i % entries_per_long) * bits_per_entry;
            longs[long_idx] |= (height_value & ((1 << bits_per_entry) - 1)) << bit_idx;
        }

        #[derive(serde::Serialize)]
        struct Heightmaps {
            #[serde(rename = "MOTION_BLOCKING")]
            motion_blocking: fastnbt::LongArray,
        }

        let hm = Heightmaps {
            motion_blocking: fastnbt::LongArray::new(longs),
        };

        fastnbt::to_bytes(&hm).unwrap_or_default()
    }

    pub fn encode_sections_legacy_void(&self) -> Vec<u8> {
        fn push_varint(out: &mut Vec<u8>, value: i32) {
            let mut tmp = BytesMut::new();
            VarInt(value).encode(&mut tmp);
            out.extend_from_slice(&tmp);
        }

        let mut sections = Vec::new();
        for _ in 0..CHUNK_SECTIONS {
            sections.extend_from_slice(&0i16.to_be_bytes());

            // Block states container
            sections.push(4); // bits per entry
            push_varint(&mut sections, 1);   // palette length
            push_varint(&mut sections, 0);   // palette[0] = air
            push_varint(&mut sections, 256); // data array length
            for _ in 0..256 {
                sections.extend_from_slice(&0i64.to_be_bytes());
            }

            // Biomes container
            sections.push(1); // bits per entry
            push_varint(&mut sections, 1); // palette length
            push_varint(&mut sections, 0); // plains
            push_varint(&mut sections, 1); // one long
            sections.extend_from_slice(&0i64.to_be_bytes());
        }
        sections
    }
}