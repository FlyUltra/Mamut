use bytes::{Buf, BufMut, BytesMut};
use crate::protocol::varint::VarIntExt;

pub trait McBufWritable {
    fn write_into(&self, buf: &mut BytesMut);
}

pub trait McBufReadable {
    fn read_from(buf: &mut BytesMut) -> Self;
}

impl McBufWritable for String {
    fn write_into(&self, buf: &mut BytesMut) {
        buf.put_var_int(self.len() as i32);
        buf.put_slice(self.as_bytes());
    }
}

impl McBufWritable for i32 {
    fn write_into(&self, buf: &mut BytesMut) {
        buf.put_i32(self.to_be());
    }
}

// Helper pro UUID
impl McBufWritable for [u8; 16] {
    fn write_into(&self, buf: &mut BytesMut) {
        buf.put_slice(self);
    }
}