use std::net::TcpStream;
use bytes::BytesMut;
use crate::protocol::packet::Packet;

pub enum State {
    Handshake,
    Status,
    Login,
    Config,
    Play,
}

pub struct Client {
    stream: TcpStream,
    pub state: State,
    pub username: Option<String>,
}

impl Client {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            state: State::Handshake,
            username: None,
        }
    }

    pub fn send<P: Packet>(&mut self, packet: P) -> std::io::Result<()> {
        let mut payload = BytesMut::new();
        packet.encode(&mut payload);


        Ok(())
    }
}