use crate::protocol::types::McWrite;
use crate::protocol::varint::VarInt;
use bytes::{BufMut, BytesMut};

pub struct ChunkSection {
    pub block_count: i16,
    pub palette: Vec<String>,
    pub data: Vec<u64>,
}

impl ChunkSection {
    pub fn encode_to_network(&self, buf: &mut BytesMut) {
        self.block_count.mc_write(buf);

        if self.palette.len() == 1 {
            buf.put_u8(0);
            VarInt(102).encode(buf);
            VarInt(0).encode(buf); // Data length 0
        } else {
            let bpe = (f32::log2(self.palette.len() as f32).ceil() as u8).max(4);
            buf.put_u8(bpe);

            VarInt(self.palette.len() as i32).encode(buf);
            for block_name in &self.palette {
                block_name.to_string().mc_write(buf);
            }

            VarInt(self.data.len() as i32).encode(buf);
            for long in &self.data {
                buf.put_u64(*long);
            }
        }

        buf.put_u8(0); // BPE 0
        VarInt(1).encode(buf); // Biome ID (Plains)
        VarInt(0).encode(buf); // Data length 0
    }
}