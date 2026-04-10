use bytes::{BufMut, BytesMut}; // put_slice je v BufMut trait
use crate::protocol::packet::Packet;
use crate::protocol::varint::VarInt;
use crate::protocol::types::McWrite;

pub struct FmlLoginWrapper {
    pub channel: String,
    pub inner_packet_id: i32,
    pub data: Vec<u8>,
}

impl Packet for FmlLoginWrapper {
    const ID: i32 = 0x02; 

    fn encode(&self, buf: &mut BytesMut) {
        self.channel.mc_write(buf);
        VarInt(self.inner_packet_id).encode(buf);
        buf.put_slice(&self.data);
    }
}