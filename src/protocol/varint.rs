use bytes::{Buf, BufMut, BytesMut};
use std::io;

pub struct VarInt(pub i32);

impl VarInt {
    pub fn encode(&self, buf: &mut BytesMut) {
        let mut v = self.0 as u32;
        while v >= 0x80 {
            buf.put_u8((v as u8 & 0x7F) | 0x80);
            v >>= 7;
        }
        buf.put_u8(v as u8);
    }

    pub fn decode(buf: &mut BytesMut) -> io::Result<i32> {
        let mut res = 0;
        let mut shift = 0;
        loop {
            if !buf.has_remaining() { return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Incomplete VarInt")); }
            let b = buf.get_u8();
            res |= ((b & 0x7F) as i32) << shift;
            if (b & 0x80) == 0 { break; }
            shift += 7;
        }
        Ok(res)
    }
}

// Extension trait so BytesMut can do buf.put_var_int(x) directly
pub trait VarIntExt {
    fn put_var_int(&mut self, value: i32);
}

impl VarIntExt for BytesMut {
    fn put_var_int(&mut self, value: i32) {
        VarInt(value).encode(self);
    }
}