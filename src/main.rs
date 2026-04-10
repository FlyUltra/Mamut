mod protocol;
mod world;

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bytes::BytesMut;

use crate::protocol::packet::{frame_packet, Packet};
use crate::protocol::packets::*;
use crate::protocol::varint::VarInt;
use crate::world::generation::generate_flat_chunk;

const HS_HANDSHAKE_ID: i32 = 0x00;

const LOGIN_START_ID: i32 = 0x00;
const LOGIN_LOGIN_ACKNOWLEDGED_ID: i32 = 0x03;

const CONFIG_CLIENT_INFORMATION_ID: i32 = 0x00;
const CONFIG_PONG_ID: i32 = 0x05;
const CONFIG_ACK_FINISH_ID: i32 = 0x03;
const CONFIG_SELECT_KNOWN_PACKS_ID: i32 = 0x07;

const CONFIG_SELECT_KNOWN_PACKS_CLIENTBOUND_ID: i32 = 0x0E;

const PLAY_CONFIRM_TELEPORT_ID: i32 = 0x00;
const PLAY_CHUNK_BATCH_RECEIVED_ID: i32 = 0x0A;
const PLAY_TICK_END_ID: i32 = 0x0C;
const PLAY_KEEPALIVE_SERVERBOUND_ID: i32 = 0x1B;
const PLAY_PLAYER_POSITION_ID: i32 = 0x1D;
const PLAY_PLAYER_POSITION_LOOK_ID: i32 = 0x1E;
const PLAY_PLAYER_LOOK_ID: i32 = 0x1F;

const SPAWN_BLOCK_X: i32 = 8;
const SPAWN_BLOCK_Y: i32 = 20;
const SPAWN_BLOCK_Z: i32 = 8;
const SPAWN_PLAYER_X: f64 = 8.5;
const SPAWN_PLAYER_Y: f64 = 20.0;
const SPAWN_PLAYER_Z: f64 = 8.5;

fn main() {
    let addr = "127.0.0.1:25568";
    let listener = TcpListener::bind(addr).expect("Port is taken");

    println!("--- PROJEKT MAMUT: MC_1_21_11 [PLAY_READY] ---");
    println!("Server port: {}", addr);

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                thread::spawn(|| {
                    if let Err(e) = handle_client(s) {
                        eprintln!("[Connection] Player left: {}", e);
                    }
                });
            }
            Err(e) => eprintln!("[Error] Decline: {}", e),
        }
    }
}

fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {
    let mut buffer = BytesMut::with_capacity(1024 * 16);
    let mut current_state = 0; // 0 hs, 2 login, 3 config, 4 play
    let mut player_name = String::new();
    let mut player_uuid = [0u8; 16];
    let mut last_keepalive_id: i64 = -1;
    let mut teleport_id: i32 = 0;
    let mut config_got_client_info = false;
    let mut config_got_known_packs_ack = false;
    let mut config_sent_known_packs = false;
    let mut config_sent_finish = false;

    stream.set_read_timeout(Some(Duration::from_secs(30)))?;

    loop {
        let (id, mut data) = read_frame(&mut stream, &mut buffer)?;

        match current_state {
            0 => {
                if id == HS_HANDSHAKE_ID {
                    let hs = HandshakePacket::decode(&mut data)?;
                    println!(
                        "[HS] Player is connecting: Protocol: {}, Status: {}, adress={}, port={}",
                        hs.protocol, hs.next_state, hs.address, hs.port
                    );
                    current_state = hs.next_state;
                } else {
                    eprintln!("[HS] Unknown packet: 0x{:02X}", id);
                }
            }

            2 => {
                if id == LOGIN_START_ID {
                    let login_start = LoginStartPacket::decode(&mut data)?;
                    player_name = login_start.name.clone();
                    player_uuid = login_start.uuid;
                    println!("[Login] Player: {}", player_name);

                    send_packet(
                        &mut stream,
                        &LoginSuccessPacket {
                            uuid: player_uuid,
                            username: player_name.clone(),
                        },
                    )?;
                    println!("[Login] Login Success Acknowledged.");
                } else if id == LOGIN_LOGIN_ACKNOWLEDGED_ID {
                    println!("[Login] Login Acknowledged CONFIGURATION.");
                    current_state = 3;

                    let ping_id = 123456i32;
                    send_packet(&mut stream, &ClientboundPingPacket { id: ping_id })?;
                    println!("[Config] PING (id={}), PONG...", ping_id);
                } else {
                    println!("[Login] Unkown packet: 0x{:02X}", id);
                }
            }

            3 => {
                if id == CONFIG_PONG_ID {
                    match PongPacket::decode(&mut data) {
                        Ok(pkt) => {
                            println!("[Config] PONG (id={})", pkt.id);
                        }
                        Err(e) => {
                            eprintln!("[Config] PONG decode fail: {}", e);
                        }
                    }
                } else if id == 0x02 {
                    println!("[Config] Client sent disconnect during config, closing connection.");
                } else if id == CONFIG_SELECT_KNOWN_PACKS_ID {
                    match SelectKnownPacksServerboundPacket::decode(&mut data) {
                        Ok(pkt) => {
                            println!("[Config] Client getting packets: {:?}", pkt.packs);
                            let has_core = pkt
                                .packs
                                .iter()
                                .any(|(namespace, id, _version)| namespace == "minecraft" && id == "core");

                            if !has_core {
                                eprintln!(
                                    "[Config] Client didnt confirm minecraft:core (registry data:None )."
                                );
                                return Ok(());
                            }

                            config_got_known_packs_ack = true;

                            if config_got_client_info && !config_sent_finish {
                                send_configuration_sequence(&mut stream)?;
                                config_sent_finish = true;
                                println!("[Config] Finish Configuration sent, waiting for ACK...");
                            }
                        }
                        Err(e) => {
                            eprintln!("[Config] Select Known Packs decode fail: {}", e);
                        }
                    }
                } else if id == CONFIG_CLIENT_INFORMATION_ID {
                    println!("[Config] + Client Information (Settings).");
                    config_got_client_info = true;

                    if !config_sent_known_packs {
                        send_packet(
                            &mut stream,
                            &SelectKnownPacksPacket {
                                packs: vec![(
                                    "minecraft".to_string(),
                                    "core".to_string(),
                                    "1.21.11".to_string(),
                                )],
                            },
                        )?;
                        config_sent_known_packs = true;
                        println!(
                            "[Config] Select Known Packs odeslány (id=0x{:02X})",
                            CONFIG_SELECT_KNOWN_PACKS_CLIENTBOUND_ID
                        );
                    }

                    if config_got_known_packs_ack && !config_sent_finish {
                        send_configuration_sequence(&mut stream)?;
                        config_sent_finish = true;
                        println!("[Config] ✓ Finish Configuration odeslán, čekám na ACK...");
                    }
                } else if id == CONFIG_ACK_FINISH_ID {
                    println!("[Config] ✓ ACK finish přijat, přepínám do PLAY.");
                    current_state = 4;

                    send_packet(
                        &mut stream,
                        &JoinGamePacket {
                            entity_id: 1,
                            is_hardcore: false,
                            dimensions: vec!["minecraft:overworld".to_string()],
                            max_players: VarInt(20),
                            view_distance: VarInt(8),
                            simulation_distance: VarInt(8),
                            reduced_debug_info: false,
                            enable_respawn_screen: true,
                            do_limited_crafting: false,
                            dimension_type: VarInt(0),
                            dimension_name: "minecraft:overworld".to_string(),
                            hashed_seed: 0,
                            gamemode: 1,
                            previous_gamemode: -1,
                            is_debug: false,
                            is_flat: true,
                            portal_cooldown: VarInt(0),
                            sea_level: VarInt(63),
                        },
                    )?;

                    send_packet(
                        &mut stream,
                        &GameEventPacket {
                            event: 13,
                            value: 0.0,
                        },
                    )?;

                    // Play bootstrap: center + chunk batch + chunks
                    send_packet(
                        &mut stream,
                        &UpdateViewPositionPacket {
                            chunk_x: VarInt(0),
                            chunk_z: VarInt(0),
                        },
                    )?;
                    send_packet(
                        &mut stream,
                        &UpdateViewDistancePacket {
                            view_distance: VarInt(2),
                        },
                    )?;
                    send_packet(
                        &mut stream,
                        &SimulationDistancePacket {
                            simulation_distance: VarInt(2),
                        },
                    )?;
                    send_packet(&mut stream, &ChunkBatchStartPacket)?;

                    let mut sent_chunks = 0;
                    for cx in -1..=1 {
                        for cz in -1..=1 {
                            let chunk = generate_flat_chunk(cx, cz);
                            let chunk_data = chunk.encode_sections();
                            let motion_blocking_heightmap = build_flat_heightmap(15);
                            send_packet(
                                &mut stream,
                                &MapChunkPacket {
                                    x: cx,
                                    z: cz,
                                    motion_blocking_heightmap,
                                    chunk_data,
                                },
                            )?;
                            sent_chunks += 1;
                        }
                    }

                    send_packet(
                        &mut stream,
                        &ChunkBatchFinishedPacket {
                            batch_size: VarInt(sent_chunks),
                        },
                    )?;

                    send_packet(
                        &mut stream,
                        &SetDefaultSpawnPositionPacket {
                            dimension_name: "minecraft:overworld".to_string(),
                            x: SPAWN_BLOCK_X,
                            y: SPAWN_BLOCK_Y,
                            z: SPAWN_BLOCK_Z,
                            yaw: 0.0,
                            pitch: 0.0,
                        },
                    )?;

                    teleport_id = 1;
                    send_packet(
                        &mut stream,
                        &PlayerPositionPacket {
                            teleport_id: VarInt(teleport_id),
                            x: SPAWN_PLAYER_X,
                            y: SPAWN_PLAYER_Y,
                            z: SPAWN_PLAYER_Z,
                            dx: 0.0,
                            dy: 0.0,
                            dz: 0.0,
                            yaw: 0.0,
                            pitch: 0.0,
                            flags: 0,
                        },
                    )?;

                    send_packet(
                        &mut stream,
                        &PlayerAbilitiesPacket {
                            flags: 0x04,
                            flying_speed: 0.05,
                            fov_modifier: 0.1,
                        },
                    )?;

                    last_keepalive_id = now_millis_i64();
                    send_packet(
                        &mut stream,
                        &KeepAliveClientboundPacket {
                            id: last_keepalive_id,
                        },
                    )?;

                    println!("[Play] Player '{}' get to game!", player_name);
                } else {
                    println!("[Config] Unknown packet in config state: 0x{:02X}", id);
                }
            }

            4 => {
                match id {
                    PLAY_CHUNK_BATCH_RECEIVED_ID => {
                        match ChunkBatchReceivedPacket::decode(&mut data) {
                            Ok(pkt) => {
                                println!("[Play] Chunk batch received, chunks/tick={:.2}", pkt.chunks_per_tick);
                            }
                            Err(e) => {
                                eprintln!("[Play] ChunkBatchReceived decode fail: {}", e);
                            }
                        }
                    }
                    PLAY_TICK_END_ID => {
                    }
                    PLAY_KEEPALIVE_SERVERBOUND_ID => {
                        match KeepAliveServerboundPacket::decode(&mut data) {
                            Ok(pkt) => {
                                if pkt.id == last_keepalive_id {
                                    println!("[Play] KeepAlive OK");
                                } else {
                                    println!(
                                        "[Play] KeepAlive mismatch: {} != {}",
                                        pkt.id, last_keepalive_id
                                    );
                                }
                            }
                            Err(e) => {
                                eprintln!("[Play] KeepAlive decode fail: {}", e);
                            }
                        }

                        thread::sleep(Duration::from_millis(300));
                        last_keepalive_id = now_millis_i64();
                        send_packet(
                            &mut stream,
                            &KeepAliveClientboundPacket {
                                id: last_keepalive_id,
                            },
                        )?;
                    }
                    PLAY_CONFIRM_TELEPORT_ID => {
                        match ConfirmTeleportPacket::decode(&mut data) {
                            Ok(pkt) => {
                                println!("[Play] Confirm Teleport: {}", pkt.teleport_id);
                            }
                            Err(e) => {
                                eprintln!("[Play] Confirm Teleport decode fail: {}", e);
                            }
                        }
                    }
                    PLAY_PLAYER_POSITION_ID => {
                        println!("[Play] Player Position update");
                    }
                    PLAY_PLAYER_POSITION_LOOK_ID => {
                        println!("[Play] Player Position+Look update");
                    }
                    PLAY_PLAYER_LOOK_ID => {
                        println!("[Play] Player Look update");
                    }
                    _ => {
                        println!("[Play] Unknown packet: 0x{:02X}", id);
                    }
                }
            }

            _ => {
                eprintln!("[?] Unknown state: {}", current_state);
                return Ok(());
            }
        }
    }
}

fn now_millis_i64() -> i64 {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0));
    dur.as_millis() as i64
}

fn build_flat_heightmap(top_y: i32) -> Vec<i64> {
    let height_value: i64 = (top_y - (-64) + 1) as i64;
    let bits_per_entry = 9usize;
    let entries_per_long = 64 / bits_per_entry;
    let total_longs = (256 + entries_per_long - 1) / entries_per_long;
    let mut longs = vec![0i64; total_longs];

    for i in 0..256usize {
        let long_idx = i / entries_per_long;
        let bit_idx = (i % entries_per_long) * bits_per_entry;
        longs[long_idx] |= (height_value & ((1 << bits_per_entry) - 1)) << bit_idx;
    }

    longs
}

fn send_configuration_sequence(stream: &mut TcpStream) -> std::io::Result<()> {
    for registry in create_empty_registries() {
        println!("[Config] Sending registry: {}", registry.registry_id);
        send_packet(stream, &registry)?;
    }

    send_packet(
        stream,
        &FeatureFlagsPacket {
            features: vec![],
        },
    )?;
    println!("[Config] Feature Flags odeslány");

    send_packet(
        stream,
        &TagsPacket {
            tags: vec![(
                "minecraft:timeline".to_string(),
                vec![("minecraft:in_overworld".to_string(), vec![0])],
            )],
        },
    )?;
    println!("[Config] Tags odeslány");

    send_packet(stream, &FinishConfigurationPacket)
}

fn send_packet<P: Packet>(stream: &mut TcpStream, packet: &P) -> std::io::Result<()> {
    let frame = frame_packet(packet);
    println!(
        "[SEND] id=0x{:02X} type={} frame={}B",
        P::ID,
        std::any::type_name::<P>(),
        frame.len()
    );
    stream.write_all(&frame)?;
    stream.flush()
}

fn read_frame(stream: &mut TcpStream, buffer: &mut BytesMut) -> std::io::Result<(i32, BytesMut)> {
    loop {
        let mut temp_buf = buffer.clone();
        match VarInt::decode(&mut temp_buf) {
            Ok(len) => {
                let data_len = len as usize;
                let varint_len = buffer.len() - temp_buf.len();
                let total_frame_len = varint_len + data_len;

                while buffer.len() < total_frame_len {
                    let mut temp = [0u8; 1024];
                    let n = stream.read(&mut temp)?;
                    if n == 0 {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "Client closed while reading frame",
                        ));
                    }
                    buffer.extend_from_slice(&temp[..n]);
                }

                let _ = VarInt::decode(buffer)?;

                let mut packet_raw = buffer.split_to(data_len);
                let packet_id = VarInt::decode(&mut packet_raw)?;

                return Ok((packet_id, packet_raw));
            }
            Err(_) => {
                let mut temp = [0u8; 1024];
                let n = stream.read(&mut temp)?;
                if n == 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "Client closed",
                    ));
                }
                buffer.extend_from_slice(&temp[..n]);
            }
        }
    }
}