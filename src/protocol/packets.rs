use crate::protocol::packet::Packet;
use crate::protocol::types::McWrite;
use crate::protocol::varint::VarInt;
use bytes::{BufMut, BytesMut};

// =============================================================================
// HANDSHAKE
// =============================================================================
pub struct HandshakePacket {
    pub protocol: i32,
    pub address: String,
    pub port: u16,
    pub next_state: i32,
}

impl HandshakePacket {
    pub fn decode(buf: &mut BytesMut) -> std::io::Result<Self> {
        let protocol = VarInt::decode(buf)?;
        let addr_len = VarInt::decode(buf)? as usize;
        let address = String::from_utf8_lossy(&buf.split_to(addr_len)).to_string();

        if buf.len() < 2 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Missing handshake port",
            ));
        }
        let port = u16::from_be_bytes(buf.split_to(2).as_ref().try_into().unwrap());
        let next_state = VarInt::decode(buf)?;
        Ok(Self { protocol, address, port, next_state })
    }
}

impl Packet for HandshakePacket {
    const ID: i32 = 0x00;
    fn encode(&self, buf: &mut BytesMut) {
        VarInt(self.protocol).encode(buf);
        self.address.mc_write(buf);
        buf.extend_from_slice(&self.port.to_be_bytes());
        VarInt(self.next_state).encode(buf);
    }
}

// =============================================================================
// LOGIN
// =============================================================================
pub struct LoginStartPacket {
    pub name: String,
    pub uuid: [u8; 16],
}

impl LoginStartPacket {
    pub fn decode(buf: &mut BytesMut) -> std::io::Result<Self> {
        let name_len = VarInt::decode(buf)? as usize;
        if buf.len() < name_len {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "LoginStart name truncated",
            ));
        }
        let name = String::from_utf8_lossy(&buf.split_to(name_len)).to_string();

        let mut uuid = [0u8; 16];
        if buf.len() >= 16 {
            uuid.copy_from_slice(&buf.split_to(16));
        }

        Ok(Self { name, uuid })
    }
}

impl Packet for LoginStartPacket {
    const ID: i32 = 0x00;
    fn encode(&self, buf: &mut BytesMut) {
        self.name.mc_write(buf);
        buf.extend_from_slice(&self.uuid);
    }
}

pub struct LoginSuccessPacket {
    pub uuid: [u8; 16],
    pub username: String,
}

impl Packet for LoginSuccessPacket {
    const ID: i32 = 0x02;
    fn encode(&self, buf: &mut BytesMut) {
        buf.put_slice(&self.uuid);
        self.username.mc_write(buf);
        VarInt(0).encode(buf); // properties count
    }
}

// =============================================================================
// CONFIGURATION
// =============================================================================

/// Registry Data packet - posílá se v configuration phase
/// Obsahuje všechny dostupné registry (dimension, biome, atd.)
pub struct RegistryDataPacket {
    pub registry_id: String,
    pub entries: Vec<RegistryEntry>,
}

pub struct RegistryEntry {
    pub key: String,          // Registry key (e.g., "minecraft:overworld")
    pub data: Option<Vec<u8>>, // option<anonymousNbt>
}

impl Packet for RegistryDataPacket {
    const ID: i32 = 0x07; // 1.21.11 Registry Data (config state clientbound)
    fn encode(&self, buf: &mut BytesMut) {
        // Registry ID string
        self.registry_id.mc_write(buf);

        // Počet entries
        VarInt(self.entries.len() as i32).encode(buf);

        // Entries
        for entry in &self.entries {
            entry.key.mc_write(buf);
            let has_data = entry.data.is_some();
            buf.put_u8(has_data as u8);
            if let Some(data) = &entry.data {
                buf.put_slice(data);
            }
        }
    }
}

/// Helper funkce - vrací minimální ne-prázdné registry.
/// `data: None` znamená, že klient použije lokální data z known packu.
pub fn create_empty_registries() -> Vec<RegistryDataPacket> {
    vec![
        // Dimension type musí mít aspoň jeden entry
        RegistryDataPacket {
            registry_id: "minecraft:dimension_type".to_string(),
            entries: vec![RegistryEntry {
                key: "minecraft:overworld".to_string(),
                data: None,
            }],
        },
        // Worldgen biome
        RegistryDataPacket {
            registry_id: "minecraft:worldgen/biome".to_string(),
            entries: vec![RegistryEntry {
                key: "minecraft:plains".to_string(),
                data: None,
            }],
        },
        // Dimension
        RegistryDataPacket {
            registry_id: "minecraft:dimension".to_string(),
            entries: vec![RegistryEntry {
                key: "minecraft:overworld".to_string(),
                data: None,
            }],
        },
        RegistryDataPacket {
            registry_id: "minecraft:timeline".to_string(),
            entries: vec![RegistryEntry {
                key: "minecraft:day".to_string(),
                data: None,
            }],
        },
        // Damage type (required)
        RegistryDataPacket {
            registry_id: "minecraft:damage_type".to_string(),
            entries: vec![
                "minecraft:arrow",
                "minecraft:bad_respawn_point",
                "minecraft:cactus",
                "minecraft:campfire",
                "minecraft:cramming",
                "minecraft:dragon_breath",
                "minecraft:drown",
                "minecraft:dry_out",
                "minecraft:ender_pearl",
                "minecraft:explosion",
                "minecraft:fall",
                "minecraft:falling_anvil",
                "minecraft:falling_block",
                "minecraft:falling_stalactite",
                "minecraft:fireball",
                "minecraft:fireworks",
                "minecraft:fly_into_wall",
                "minecraft:freeze",
                "minecraft:generic",
                "minecraft:generic_kill",
                "minecraft:hot_floor",
                "minecraft:in_fire",
                "minecraft:in_wall",
                "minecraft:indirect_magic",
                "minecraft:lava",
                "minecraft:lightning_bolt",
                "minecraft:mace_smash",
                "minecraft:magic",
                "minecraft:mob_attack",
                "minecraft:mob_attack_no_aggro",
                "minecraft:mob_projectile",
                "minecraft:on_fire",
                "minecraft:out_of_world",
                "minecraft:outside_border",
                "minecraft:player_attack",
                "minecraft:player_explosion",
                "minecraft:sonic_boom",
                "minecraft:spear",
                "minecraft:spit",
                "minecraft:stalagmite",
                "minecraft:starve",
                "minecraft:sting",
                "minecraft:sweet_berry_bush",
                "minecraft:thorns",
                "minecraft:thrown",
                "minecraft:trident",
                "minecraft:unattributed_fireball",
                "minecraft:wind_charge",
                "minecraft:wither",
                "minecraft:wither_skull",
            ]
            .into_iter()
            .map(|key| RegistryEntry {
                key: key.to_string(),
                data: None,
            })
            .collect(),
        },
        // Cat variant (non-empty!)
        RegistryDataPacket {
            registry_id: "minecraft:cat_variant".to_string(),
            entries: vec![RegistryEntry {
                key: "minecraft:tabby".to_string(),
                data: None,
            }],
        },
        RegistryDataPacket {
            registry_id: "minecraft:chicken_variant".to_string(),
            entries: vec![RegistryEntry {
                key: "minecraft:temperate".to_string(),
                data: None,
            }],
        },
        RegistryDataPacket {
            registry_id: "minecraft:cow_variant".to_string(),
            entries: vec![RegistryEntry {
                key: "minecraft:temperate".to_string(),
                data: None,
            }],
        },
        RegistryDataPacket {
            registry_id: "minecraft:frog_variant".to_string(),
            entries: vec![RegistryEntry {
                key: "minecraft:temperate".to_string(),
                data: None,
            }],
        },
        RegistryDataPacket {
            registry_id: "minecraft:painting_variant".to_string(),
            entries: vec![RegistryEntry {
                key: "minecraft:kebab".to_string(),
                data: None,
            }],
        },
        RegistryDataPacket {
            registry_id: "minecraft:pig_variant".to_string(),
            entries: vec![RegistryEntry {
                key: "minecraft:temperate".to_string(),
                data: None,
            }],
        },
        RegistryDataPacket {
            registry_id: "minecraft:wolf_sound_variant".to_string(),
            entries: vec![RegistryEntry {
                key: "minecraft:classic".to_string(),
                data: None,
            }],
        },
        RegistryDataPacket {
            registry_id: "minecraft:wolf_variant".to_string(),
            entries: vec![RegistryEntry {
                key: "minecraft:pale".to_string(),
                data: None,
            }],
        },
        RegistryDataPacket {
            registry_id: "minecraft:zombie_nautilus_variant".to_string(),
            entries: vec![RegistryEntry {
                key: "minecraft:temperate".to_string(),
                data: None,
            }],
        },
    ]
}

pub struct FinishConfigurationPacket;
impl Packet for FinishConfigurationPacket {
    const ID: i32 = 0x03; // 1.21.11 Finish Configuration (clientbound)
    fn encode(&self, _buf: &mut BytesMut) {}
}

// PING packet (serverbound - klient pošle)
pub struct PingPacket {
    pub id: i32,
}

impl PingPacket {
    pub fn decode(buf: &mut BytesMut) -> std::io::Result<Self> {
        if buf.len() < 4 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Ping truncated",
            ));
        }
        let id = i32::from_be_bytes(buf.split_to(4).as_ref().try_into().unwrap());
        Ok(Self { id })
    }
}

// ClientBound PING packet (my pošleme klientovi, klient odpověhní PONG)
pub struct ClientboundPingPacket {
    pub id: i32,
}

impl Packet for ClientboundPingPacket {
    const ID: i32 = 0x05; // 1.21.11 Ping (clientbound)
    fn encode(&self, buf: &mut BytesMut) {
        buf.put_i32(self.id);
    }
}

// PONG packet (serverbound - klient odpověhní na náš PING)
pub struct PongPacket {
    pub id: i32,
}

impl PongPacket {
    pub fn decode(buf: &mut BytesMut) -> std::io::Result<Self> {
        if buf.len() < 4 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Pong truncated",
            ));
        }
        let id = i32::from_be_bytes(buf.split_to(4).as_ref().try_into().unwrap());
        Ok(Self { id })
    }
}

// Clientbound Feature Flags (1.21.11)
pub struct FeatureFlagsPacket {
    pub features: Vec<String>,
}

impl Packet for FeatureFlagsPacket {
    const ID: i32 = 0x0C; // 1.21.11 Feature Flags
    fn encode(&self, buf: &mut BytesMut) {
        VarInt(self.features.len() as i32).encode(buf);
        for feature in &self.features {
            feature.mc_write(buf);
        }
    }
}

// Clientbound Tags (1.21.11)
pub struct TagsPacket {
    pub tags: Vec<(String, Vec<(String, Vec<i32>)>)>, // (registry_id, [(tag_name, ids)])
}

impl Packet for TagsPacket {
    const ID: i32 = 0x0D; // 1.21.11 Tags
    fn encode(&self, buf: &mut BytesMut) {
        VarInt(self.tags.len() as i32).encode(buf);
        for (registry_id, registry_tags) in &self.tags {
            registry_id.mc_write(buf);
            VarInt(registry_tags.len() as i32).encode(buf);
            for (tag_name, ids) in registry_tags {
                tag_name.mc_write(buf);
                VarInt(ids.len() as i32).encode(buf);
                for id in ids {
                    VarInt(*id).encode(buf);
                }
            }
        }
    }
}

// =============================================================================
// PLAY
// =============================================================================

// Clientbound Join Game
pub struct JoinGamePacket {
    pub entity_id: i32,
    pub is_hardcore: bool,
    pub dimensions: Vec<String>,
    pub max_players: VarInt,
    pub view_distance: VarInt,
    pub simulation_distance: VarInt,
    pub reduced_debug_info: bool,
    pub enable_respawn_screen: bool,
    pub do_limited_crafting: bool,
    pub dimension_type: VarInt,
    pub dimension_name: String,
    pub hashed_seed: i64,
    pub gamemode: u8,
    pub previous_gamemode: i8,
    pub is_debug: bool,
    pub is_flat: bool,
    pub portal_cooldown: VarInt,
    pub sea_level: VarInt,
}

impl Packet for JoinGamePacket {
    const ID: i32 = 0x30; // 1.21.11 Login (join game)
    fn encode(&self, buf: &mut BytesMut) {
        buf.put_i32(self.entity_id);
        buf.put_u8(self.is_hardcore as u8);

        VarInt(self.dimensions.len() as i32).encode(buf);
        for dim in &self.dimensions {
            dim.mc_write(buf);
        }

        self.max_players.encode(buf);
        self.view_distance.encode(buf);
        self.simulation_distance.encode(buf);
        buf.put_u8(self.reduced_debug_info as u8);
        buf.put_u8(self.enable_respawn_screen as u8);
        buf.put_u8(self.do_limited_crafting as u8);

        self.dimension_type.encode(buf);
        self.dimension_name.mc_write(buf);
        buf.put_i64(self.hashed_seed);
        buf.put_u8(self.gamemode);
        buf.put_i8(self.previous_gamemode);
        buf.put_u8(self.is_debug as u8);
        buf.put_u8(self.is_flat as u8);

        buf.put_u8(0); // death location absent
        self.portal_cooldown.encode(buf);
        self.sea_level.encode(buf);
        buf.put_u8(0); // enforces secure chat = false
    }
}

// Clientbound synchronize position
pub struct PlayerPositionPacket {
    pub teleport_id: VarInt,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub dx: f64,
    pub dy: f64,
    pub dz: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub flags: u32,
}

impl Packet for PlayerPositionPacket {
    const ID: i32 = 0x46; // 1.21.11 position
    fn encode(&self, buf: &mut BytesMut) {
        self.teleport_id.encode(buf);
        buf.put_f64(self.x);
        buf.put_f64(self.y);
        buf.put_f64(self.z);
        buf.put_f64(self.dx);
        buf.put_f64(self.dy);
        buf.put_f64(self.dz);
        buf.put_f32(self.yaw);
        buf.put_f32(self.pitch);
        buf.put_u32(self.flags);
    }
}

// Clientbound KeepAlive
pub struct KeepAliveClientboundPacket {
    pub id: i64,
}
impl Packet for KeepAliveClientboundPacket {
    const ID: i32 = 0x2B; // 1.21.11 Keep Alive (clientbound)
    fn encode(&self, buf: &mut BytesMut) {
        buf.put_i64(self.id);
    }
}

// Serverbound KeepAlive response
pub struct KeepAliveServerboundPacket {
    pub id: i64,
}
impl KeepAliveServerboundPacket {
    pub fn decode(buf: &mut BytesMut) -> std::io::Result<Self> {
        if buf.len() < 8 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "KeepAlive response truncated",
            ));
        }
        let bytes = buf.split_to(8);
        let id = i64::from_be_bytes(bytes.as_ref().try_into().unwrap());
        Ok(Self { id })
    }
}

// Serverbound Chunk Batch Received (Play 0x0A)
pub struct ChunkBatchReceivedPacket {
    pub chunks_per_tick: f32,
}

impl ChunkBatchReceivedPacket {
    pub fn decode(buf: &mut BytesMut) -> std::io::Result<Self> {
        if buf.len() < 4 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "ChunkBatchReceived truncated",
            ));
        }
        let bytes = buf.split_to(4);
        let chunks_per_tick = f32::from_be_bytes(bytes.as_ref().try_into().unwrap());
        Ok(Self { chunks_per_tick })
    }
}

// =============================================================================
// 1.21.11 Additional Packets
// =============================================================================

// Serverbound Confirm Teleport
pub struct ConfirmTeleportPacket {
    pub teleport_id: i32,
}
impl ConfirmTeleportPacket {
    pub fn decode(buf: &mut BytesMut) -> std::io::Result<Self> {
        let teleport_id = VarInt::decode(buf)?;
        Ok(Self { teleport_id })
    }
}

// Clientbound Set Default Spawn Position
pub struct SetDefaultSpawnPositionPacket {
    pub dimension_name: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub yaw: f32,
    pub pitch: f32,
}

fn pack_block_pos(x: i32, y: i32, z: i32) -> i64 {
    // Minecraft BlockPos: x (26 bits), z (26 bits), y (12 bits)
    let x = (x as i64 & 0x3ffffff) << 38;
    let z = (z as i64 & 0x3ffffff) << 12;
    let y = y as i64 & 0xfff;
    x | z | y
}

impl Packet for SetDefaultSpawnPositionPacket {
    const ID: i32 = 0x5F; // 1.21.11 spawn_position
    fn encode(&self, buf: &mut BytesMut) {
        self.dimension_name.mc_write(buf);
        buf.put_i64(pack_block_pos(self.x, self.y, self.z));
        buf.put_f32(self.yaw);
        buf.put_f32(self.pitch);
    }
}

// Clientbound Chunk Data with Light Data
pub struct ChunkDataPacket {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub data: Vec<u8>,
}
impl Packet for ChunkDataPacket {
    const ID: i32 = 0x23; // 1.21.11 Chunk Data and Update Light
    fn encode(&self, buf: &mut BytesMut) {
        buf.put_i32(self.chunk_x);
        buf.put_i32(self.chunk_z);
        buf.put_slice(&self.data);
    }
}

// Clientbound Game Event
pub struct GameEventPacket {
    pub event: u8,
    pub value: f32,
}
impl Packet for GameEventPacket {
    const ID: i32 = 0x26; // 1.21.11 game_state_change
    fn encode(&self, buf: &mut BytesMut) {
        buf.put_u8(self.event);
        buf.put_f32(self.value);
    }
}

// Clientbound Player Abilities
pub struct PlayerAbilitiesPacket {
    pub flags: u8,
    pub flying_speed: f32,
    pub fov_modifier: f32,
}

// Clientbound Update View Position
pub struct UpdateViewPositionPacket {
    pub chunk_x: VarInt,
    pub chunk_z: VarInt,
}

impl Packet for UpdateViewPositionPacket {
    const ID: i32 = 0x5C; // 1.21.11 update_view_position
    fn encode(&self, buf: &mut BytesMut) {
        self.chunk_x.encode(buf);
        self.chunk_z.encode(buf);
    }
}

// Clientbound Update View Distance
pub struct UpdateViewDistancePacket {
    pub view_distance: VarInt,
}

impl Packet for UpdateViewDistancePacket {
    const ID: i32 = 0x5D; // 1.21.11 update_view_distance
    fn encode(&self, buf: &mut BytesMut) {
        self.view_distance.encode(buf);
    }
}

// Clientbound Simulation Distance
pub struct SimulationDistancePacket {
    pub simulation_distance: VarInt,
}

impl Packet for SimulationDistancePacket {
    const ID: i32 = 0x6D; // 1.21.11 simulation_distance
    fn encode(&self, buf: &mut BytesMut) {
        self.simulation_distance.encode(buf);
    }
}

// Clientbound Chunk Batch Start
pub struct ChunkBatchStartPacket;

impl Packet for ChunkBatchStartPacket {
    const ID: i32 = 0x0C; // 1.21.11 chunk_batch_start
    fn encode(&self, _buf: &mut BytesMut) {}
}

// Clientbound Chunk Batch Finished
pub struct ChunkBatchFinishedPacket {
    pub batch_size: VarInt,
}

impl Packet for ChunkBatchFinishedPacket {
    const ID: i32 = 0x0B; // 1.21.11 chunk_batch_finished
    fn encode(&self, buf: &mut BytesMut) {
        self.batch_size.encode(buf);
    }
}

pub struct MapChunkPacket {
    pub x: i32,
    pub z: i32,
    pub motion_blocking_heightmap: Vec<i64>,
    pub chunk_data: Vec<u8>,
}

impl Packet for MapChunkPacket {
    const ID: i32 = 0x2C;
    fn encode(&self, buf: &mut BytesMut) {
        buf.put_i32(self.x);
        buf.put_i32(self.z);

        VarInt(1).encode(buf);
        VarInt(4).encode(buf);
        VarInt(self.motion_blocking_heightmap.len() as i32).encode(buf);
        for v in &self.motion_blocking_heightmap {
            buf.put_i64(*v);
        }

        VarInt(self.chunk_data.len() as i32).encode(buf);
        buf.put_slice(&self.chunk_data);

        VarInt(0).encode(buf);

        VarInt(0).encode(buf);
        VarInt(0).encode(buf);

        VarInt(1).encode(buf);
        buf.put_i64(0x3FFFFFF);
        VarInt(1).encode(buf);
        buf.put_i64(0x3FFFFFF);

        VarInt(0).encode(buf);
        VarInt(0).encode(buf);
    }
}
impl Packet for PlayerAbilitiesPacket {
    const ID: i32 = 0x3E;
    fn encode(&self, buf: &mut BytesMut) {
        buf.put_u8(self.flags);
        buf.put_f32(self.flying_speed);
        buf.put_f32(self.fov_modifier);
    }
}

pub struct UpdateTagsPacket {
    pub tags: Vec<(String, Vec<i32>)>,
}
impl Packet for UpdateTagsPacket {
    const ID: i32 = 0x84;
    fn encode(&self, buf: &mut BytesMut) {
        VarInt(self.tags.len() as i32).encode(buf);
        for (registry_id, ids) in &self.tags {
            registry_id.mc_write(buf);
            VarInt(ids.len() as i32).encode(buf);
            for id in ids {
                VarInt(*id).encode(buf);
            }
        }
    }
}

pub struct SelectKnownPacksPacket {
    pub packs: Vec<(String, String, String)>, // (namespace, id, version)
}

impl Packet for SelectKnownPacksPacket {
    const ID: i32 = 0x0e; // 1.21.11 Select Known Packs (clientbound config)
    fn encode(&self, buf: &mut BytesMut) {
        VarInt(self.packs.len() as i32).encode(buf);
        for (namespace, id, version) in &self.packs {
            namespace.mc_write(buf);
            id.mc_write(buf);
            version.mc_write(buf);
        }
    }
}

pub struct SelectKnownPacksServerboundPacket {
    pub packs: Vec<(String, String, String)>, // (namespace, id, version)
}

impl SelectKnownPacksServerboundPacket {
    pub fn decode(buf: &mut BytesMut) -> std::io::Result<Self> {
        let count = VarInt::decode(buf)? as usize;
        let mut packs = Vec::new();
        for _ in 0..count {
            let namespace_len = VarInt::decode(buf)? as usize;
            let namespace = String::from_utf8_lossy(&buf.split_to(namespace_len)).to_string();
            let id_len = VarInt::decode(buf)? as usize;
            let id = String::from_utf8_lossy(&buf.split_to(id_len)).to_string();
            let version_len = VarInt::decode(buf)? as usize;
            let version = String::from_utf8_lossy(&buf.split_to(version_len)).to_string();
            packs.push((namespace, id, version));
        }
        Ok(Self { packs })
    }
}

