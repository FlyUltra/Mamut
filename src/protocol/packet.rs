use bytes::BytesMut;
use std::io;

pub trait Packet: Send + Sync {
    const ID: i32;
    fn encode(&self, buf: &mut BytesMut);

    fn name(&self) -> &'static str {
        "Unknown"
    }
}

pub trait PacketDecode: Sized {
    fn decode(buf: &mut BytesMut) -> io::Result<Self>;
}

pub fn frame_packet<P: Packet>(packet: &P) -> BytesMut {
    let mut data = BytesMut::new();
    crate::protocol::varint::VarInt(P::ID).encode(&mut data);
    packet.encode(&mut data);

    let mut frame = BytesMut::new();
    crate::protocol::varint::VarInt(data.len() as i32).encode(&mut frame);
    frame.extend_from_slice(&data);
    frame
}

pub fn debug_packet_id(id: i32, state: i32) -> String {
    match state {
        0 => format!("HANDSHAKE(0x{:02X})", id),
        2 => format!("LOGIN(0x{:02X})", id),
        3 => format!("CONFIG(0x{:02X})", id),
        4 => format!("PLAY(0x{:02X})", id),
        _ => format!("UNKNOWN(0x{:02X})", id),
    }
}
