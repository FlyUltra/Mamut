use bytes::{BufMut, BytesMut};
use crate::protocol::varint::VarInt;

pub trait McWrite {
    fn mc_write(&self, buf: &mut BytesMut);
}

impl McWrite for i16 {
    fn mc_write(&self, buf: &mut BytesMut) {
        buf.put_i16(*self);
    }
}

impl McWrite for String {
    fn mc_write(&self, buf: &mut BytesMut) {
        VarInt(self.len() as i32).encode(buf);
        buf.put_slice(self.as_bytes());
    }
}

impl McWrite for &str {
    fn mc_write(&self, buf: &mut BytesMut) {
        VarInt(self.len() as i32).encode(buf);
        buf.put_slice(self.as_bytes());
    }
}