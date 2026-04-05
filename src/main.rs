mod world;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use serde_json::Value;

const SUPPORTED_PROTO: i32 = 773;
const LOGIN_PACKET_PATH: &str = "src/loginPacket.json";

fn main() {
    let listener = TcpListener::bind("127.0.0.1:25565").expect("Port 25565 is occupied");
    println!("Server running on 127.0.0.1:25565");
    println!("### BUILD: MC_1_21_10_VOID_BOOTSTRAP_FULL ###");

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                println!("\n--- NEW CONNECTION ---");
                if let Err(e) = handle_connection(s) {
                    eprintln!("[ERR] {}", e);
                }
            }
            Err(e) => eprintln!("accept err: {}", e),
        }
    }
}

fn handle_connection(mut s: TcpStream) -> io::Result<()> {
    s.set_nodelay(true)?;

    let (hs_id, hs_data) = read_packet(&mut s)?;
    if hs_id != 0x00 { return Ok(()); }

    let mut c = Cursor::new(hs_data);
    let proto = c.read_varint()?;
    let addr = c.read_string()?;
    let port = c.read_u16()?;
    let next = c.read_varint()?;

    println!("[HS] proto={} addr={} port={} next={}", proto, addr, port, next);

    if proto != SUPPORTED_PROTO {
        login_disconnect(&mut s, &format!("{{\"text\":\"Unsupported protocol {}. Expected {}.\"}}", proto, SUPPORTED_PROTO))?;
        return Ok(());
    }

    match next {
        1 => { handle_status(&mut s)?; return Ok(()); }
        2 => {}
        _ => return Ok(()),
    }

    let (login_id, login_data) = read_packet(&mut s)?;
    if login_id != 0x00 { return Ok(()); }

    let mut lc = Cursor::new(login_data);
    let name = lc.read_string()?;
    let uuid = lc.read_uuid_or_zeroed();
    println!("[LOGIN] name={}", name);

    {
        let mut d = Vec::new();
        d.extend_from_slice(&uuid);
        write_string(&mut d, &name);
        write_varint(&mut d, 0);
        send_packet_dbg(&mut s, 0x02, &d, "LoginSuccess")?;
    }

    println!("[CFG] waiting LoginAck sb:0x03");
    drain_until_id(&mut s, 0x03, "CFG")?;

    {
        let mut d = Vec::new();
        write_varint(&mut d, 1);
        write_string(&mut d, "minecraft");
        write_string(&mut d, "core");
        write_string(&mut d, "1.21.10");
        send_packet_dbg(&mut s, 0x0E, &d, "KnownPacks")?;
    }

    println!("[CFG] waiting ServerboundKnownPacks sb:0x07");
    drain_until_id(&mut s, 0x07, "CFG")?;

    send_registries_and_tags_from_login_packet(&mut s, LOGIN_PACKET_PATH)?;
    send_packet_dbg(&mut s, 0x03, &[], "FinishConfiguration")?;

    println!("[CFG] waiting ConfigAck sb:0x03");
    drain_until_id(&mut s, 0x03, "CFG")?;

    println!(">>> SWITCH TO PLAY <<<");

    {
        let mut d = Vec::new();
        d.extend_from_slice(&1i32.to_be_bytes());
        d.push(0);
        write_varint(&mut d, 1);
        write_string(&mut d, "minecraft:overworld");
        write_varint(&mut d, 20);
        write_varint(&mut d, 2);
        write_varint(&mut d, 2);
        d.push(0); d.push(1); d.push(0);
        write_varint(&mut d, 0);
        write_string(&mut d, "minecraft:overworld");
        d.extend_from_slice(&0i64.to_be_bytes());
        d.push(1u8); d.push(255u8); d.push(0); d.push(1); d.push(0);
        write_varint(&mut d, 0);
        write_varint(&mut d, -63);
        d.push(0);
        send_packet_dbg(&mut s, 0x30, &d, "JoinGame")?;
    }

    {
        let mut d = Vec::new();
        write_varint(&mut d, 0x15);
        write_varint(&mut d, 1);
        d.extend_from_slice(&uuid);
        write_string(&mut d, &name);
        write_varint(&mut d, 0);
        write_varint(&mut d, 1);
        write_varint(&mut d, 0);
        send_packet_dbg(&mut s, 0x44, &d, "PlayerInfoUpdate")?;
    }

    {
        let mut d = Vec::new();
        d.push(15);
        d.extend_from_slice(&0.05f32.to_be_bytes());
        d.extend_from_slice(&0.1f32.to_be_bytes());
        send_packet_dbg(&mut s, 0x3E, &d, "Abilities")?;
    }

    {
        let mut d = Vec::new();
        d.push(13u8);
        d.extend_from_slice(&0.0f32.to_be_bytes());
        send_packet_dbg(&mut s, 0x26, &d, "GameStateChange(ChunksLoadStart)")?;
    }

    {
        let mut d = Vec::new();
        write_string(&mut d, "minecraft:overworld");
        d.extend_from_slice(&100i64.to_be_bytes());
        d.extend_from_slice(&0.0f32.to_be_bytes());
        d.extend_from_slice(&0.0f32.to_be_bytes());
        send_packet_dbg(&mut s, 0x5F, &d, "SpawnPosition")?;
    }

    send_play_bootstrap_void(&mut s)?;

    {
        let mut d = Vec::new();
        write_varint(&mut d, 1);
        d.extend_from_slice(&0.0f64.to_be_bytes());
        d.extend_from_slice(&100.0f64.to_be_bytes());
        d.extend_from_slice(&0.0f64.to_be_bytes());
        d.extend_from_slice(&0.0f64.to_be_bytes());
        d.extend_from_slice(&0.0f64.to_be_bytes());
        d.extend_from_slice(&0.0f64.to_be_bytes());
        d.extend_from_slice(&0.0f32.to_be_bytes());
        d.extend_from_slice(&0.0f32.to_be_bytes());
        d.extend_from_slice(&0u32.to_be_bytes());
        send_packet_dbg(&mut s, 0x46, &d, "Position")?;
    }

    println!("[PLAY] waiting TeleportConfirm sb:0x00");
    drain_until_id(&mut s, 0x00, "PLAY")?;

    println!("[OK] player entered play, listening...");


    s.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
    let mut last_keep_alive = std::time::Instant::now();

    loop {
        if last_keep_alive.elapsed().as_secs() >= 10 {
            let mut d = Vec::new();
            d.extend_from_slice(&123456789i64.to_be_bytes());

            if let Err(e) = send_packet_dbg(&mut s, 0x2B, &d, "KeepAlive(Clientbound)") {
                eprintln!("[DISCONNECT] Ztráta spojení s klientem: {}", e);
                break;
            }
            last_keep_alive = std::time::Instant::now();
        }

        match read_packet(&mut s) {
            Ok((id, data)) => {
                if id == 0x1B {
                } else {
                }
            }
            Err(e) => {
                if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut {
                    continue;
                }

                if e.kind() != io::ErrorKind::UnexpectedEof {
                    eprintln!("[LOOP ERR] {}", e);
                }
                break;
            }
        }
    }
    Ok(())
}

fn send_play_bootstrap_void(s: &mut TcpStream) -> io::Result<()> {
    let mut d = Vec::new(); write_varint(&mut d, 2);
    send_packet_dbg(s, 0x5D, &d, "UpdateViewDistance")?;

    let mut d = Vec::new(); write_varint(&mut d, 2);
    send_packet_dbg(s, 0x6D, &d, "SimulationDistance")?;

    let mut d = Vec::new(); write_varint(&mut d, 0); write_varint(&mut d, 0);
    send_packet_dbg(s, 0x5C, &d, "UpdateViewPosition")?;

    send_packet_dbg(s, 0x0C, &[], "ChunkBatchStart")?;

    println!("[SEND] Odesílám 25 chunků čtverce dohledu, prosím čekejte...");

    for cx in -2..=2i32 {
        for cz in -2..=2i32 {
            let mut d = Vec::new();
            d.extend_from_slice(&cx.to_be_bytes());
            d.extend_from_slice(&cz.to_be_bytes());

            d.push(0); // Prázdné heightmaps

            let mut sections = Vec::new();
            for _ in 0..24 {
                sections.extend_from_slice(&0u16.to_be_bytes());

                sections.push(4);
                write_varint(&mut sections, 1);
                write_varint(&mut sections, 0);
                write_varint(&mut sections, 256);
                for _ in 0..256 { sections.extend_from_slice(&0i64.to_be_bytes()); }

                sections.push(1);
                write_varint(&mut sections, 1);
                write_varint(&mut sections, 0);
                write_varint(&mut sections, 1);
                sections.extend_from_slice(&0i64.to_be_bytes());
            }
            write_varint(&mut d, sections.len() as i32);
            d.extend_from_slice(&sections);

            write_varint(&mut d, 0); // 0 Block entities

            write_varint(&mut d, 0); // skyLightMask
            write_varint(&mut d, 0); // blockLightMask
            write_varint(&mut d, 1); d.extend_from_slice(&(0x3FFFFFFi64).to_be_bytes()); // emptySkyLightMask
            write_varint(&mut d, 1); d.extend_from_slice(&(0x3FFFFFFi64).to_be_bytes()); // emptyBlockLightMask
            write_varint(&mut d, 0); // skyLight arrays
            write_varint(&mut d, 0); // blockLight arrays

            send_packet(s, 0x2C, &d)?;
        }
    }

    let mut d = Vec::new();
    write_varint(&mut d, 25);
    send_packet_dbg(s, 0x0B, &d, "ChunkBatchFinished")?;
    Ok(())
}

fn handle_status(s: &mut TcpStream) -> io::Result<()> {
    let (id0, _) = read_packet(s)?;
    if id0 != 0x00 { return Ok(()); }

    let json = "{\"version\":{\"name\":\"1.21.10\",\"protocol\":773},\"players\":{\"max\":20,\"online\":0,\"sample\":[]},\"description\":{\"text\":\"Rust 1.21.10 void server\"}}";
    let mut d = Vec::new();
    write_string(&mut d, json);
    send_packet_dbg(s, 0x00, &d, "StatusResponse")?;

    let (id1, data1) = read_packet(s)?;
    if id1 == 0x01 && data1.len() == 8 {
        send_packet_dbg(s, 0x01, &data1, "Pong")?;
    }
    Ok(())
}

fn send_registries_and_tags_from_login_packet(s: &mut TcpStream, path: &str) -> io::Result<()> {
    let raw = fs::read_to_string(path).unwrap_or_else(|_| panic!("Failed to read {}", path));
    let root: Value = serde_json::from_str(&raw).expect("Invalid JSON");

    let dim_codec = root.get("dimensionCodec").unwrap().as_object().unwrap();
    let blocked = ["minecraft:enchantment", "minecraft:dialog", "minecraft:test_environment", "minecraft:test_instance", "minecraft:timeline"];
    let mut sent_registry_ids = Vec::new();

    for (name, reg_obj) in dim_codec {
        let id = reg_obj.get("id").unwrap().as_str().unwrap();
        if blocked.contains(&id) { continue; }

        let entries = reg_obj.get("entries").unwrap().as_array().unwrap();
        let mut d = Vec::new();
        write_string(&mut d, id);
        write_varint(&mut d, entries.len() as i32);

        for e in entries {
            write_string(&mut d, e.get("key").unwrap().as_str().unwrap());
            d.push(1);
            write_anonymous_nbt_from_prismarine_node(&mut d, e.get("value").unwrap())?;
        }
        send_packet_dbg(s, 0x07, &d, &format!("RegistryData({})", id))?;
        sent_registry_ids.push(id.to_string());
    }
    send_tags_for_registries(s, &sent_registry_ids)
}

fn send_tags_for_registries(s: &mut TcpStream, registry_ids: &[String]) -> io::Result<()> {
    let mut ids = registry_ids.to_vec();
    ids.sort();
    let mut d = Vec::new();
    write_varint(&mut d, ids.len() as i32);
    for reg in &ids {
        write_string(&mut d, reg);
        write_varint(&mut d, 0);
    }
    send_packet_dbg(s, 0x0D, &d, "UpdateTags")
}

fn write_anonymous_nbt_from_prismarine_node(out: &mut Vec<u8>, node: &Value) -> io::Result<()> {
    let t = node.get("type").unwrap().as_str().unwrap();
    if t != "compound" { return Err(io::Error::new(io::ErrorKind::InvalidData, "anonymousNbt root must be compound")); }
    out.push(10);
    write_nbt_payload_by_type(out, t, node.get("value").unwrap())
}

fn write_named_nbt_tag(out: &mut Vec<u8>, name: &str, node: &Value) -> io::Result<()> {
    let t = node.get("type").unwrap().as_str().unwrap();
    out.push(nbt_tag_id(t)?);
    nbt_write_name(out, name);
    write_nbt_payload_by_type(out, t, node.get("value").unwrap())
}

fn write_nbt_payload_by_type(out: &mut Vec<u8>, t: &str, value: &Value) -> io::Result<()> {
    match t {
        "byte" => out.push(value.as_i64().unwrap() as i8 as u8),
        "short" => out.extend_from_slice(&(value.as_i64().unwrap() as i16).to_be_bytes()),
        "int" => out.extend_from_slice(&(value.as_i64().unwrap() as i32).to_be_bytes()),
        "long" => out.extend_from_slice(&(value.as_i64().unwrap()).to_be_bytes()),
        "float" => out.extend_from_slice(&(value.as_f64().unwrap() as f32).to_be_bytes()),
        "double" => out.extend_from_slice(&(value.as_f64().unwrap()).to_be_bytes()),
        "string" => nbt_write_string(out, value.as_str().unwrap()),
        "list" => {
            let obj = value.as_object().unwrap();
            let elem_type = obj.get("type").unwrap().as_str().unwrap();
            let arr = obj.get("value").unwrap().as_array().unwrap();
            out.push(nbt_tag_id(elem_type)?);
            out.extend_from_slice(&(arr.len() as i32).to_be_bytes());
            for el in arr {
                if let (Some(et), Some(ev)) = (el.get("type").and_then(|v| v.as_str()), el.get("value")) {
                    write_nbt_payload_by_type(out, et, ev)?;
                } else {
                    write_nbt_payload_by_type(out, elem_type, el)?;
                }
            }
        }
        "compound" => {
            let obj = value.as_object().unwrap();
            if obj.contains_key("has_skylight") {
                if !obj.contains_key("natural") { out.push(1); nbt_write_name(out, "natural"); out.push(1); }
                if !obj.contains_key("bed_works") { out.push(1); nbt_write_name(out, "bed_works"); out.push(1); }
                if !obj.contains_key("respawn_anchor_works") { out.push(1); nbt_write_name(out, "respawn_anchor_works"); out.push(0); }
                if !obj.contains_key("ultrawarm") { out.push(1); nbt_write_name(out, "ultrawarm"); out.push(0); }
                if !obj.contains_key("piglin_safe") { out.push(1); nbt_write_name(out, "piglin_safe"); out.push(0); }
                if !obj.contains_key("has_raids") { out.push(1); nbt_write_name(out, "has_raids"); out.push(1); }
                if !obj.contains_key("effects") { out.push(8); nbt_write_name(out, "effects"); nbt_write_string(out, "minecraft:overworld"); }
            }
            for (k, v) in obj {
                if k == "timelines" || k == "attributes" || k == "skybox" || k == "cardinal_light" || k == "temperature_modifier" {
                    continue;
                }
                if k == "effects" && obj.contains_key("downfall") {
                    out.push(10); nbt_write_name(out, "effects");
                    out.push(3); nbt_write_name(out, "fog_color"); out.extend_from_slice(&(12638463i32).to_be_bytes());
                    out.push(3); nbt_write_name(out, "sky_color"); out.extend_from_slice(&(7907327i32).to_be_bytes());
                    out.push(3); nbt_write_name(out, "water_color"); out.extend_from_slice(&(4159204i32).to_be_bytes());
                    out.push(3); nbt_write_name(out, "water_fog_color"); out.extend_from_slice(&(329011i32).to_be_bytes());
                    out.push(0);
                    continue;
                }
                write_named_nbt_tag(out, k, v)?;
            }
            out.push(0);
        }
        "byteArray" | "byte_array" => {
            let arr = value.as_array().unwrap();
            out.extend_from_slice(&(arr.len() as i32).to_be_bytes());
            for x in arr { out.push(x.as_i64().unwrap() as i8 as u8); }
        }
        "intArray" | "int_array" => {
            let arr = value.as_array().unwrap();
            out.extend_from_slice(&(arr.len() as i32).to_be_bytes());
            for x in arr { out.extend_from_slice(&(x.as_i64().unwrap() as i32).to_be_bytes()); }
        }
        "longArray" | "long_array" => {
            let arr = value.as_array().unwrap();
            out.extend_from_slice(&(arr.len() as i32).to_be_bytes());
            for x in arr { out.extend_from_slice(&(x.as_i64().unwrap()).to_be_bytes()); }
        }
        _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "unsupported type")),
    }
    Ok(())
}

fn nbt_tag_id(t: &str) -> io::Result<u8> {
    Ok(match t {
        "end" => 0, "byte" => 1, "short" => 2, "int" => 3, "long" => 4, "float" => 5,
        "double" => 6, "byteArray" | "byte_array" => 7, "string" => 8, "list" => 9,
        "compound" => 10, "intArray" | "int_array" => 11, "longArray" | "long_array" => 12,
        _ => return Err(io::Error::new(io::ErrorKind::InvalidData, format!("unknown nbt type {}", t))),
    })
}

fn nbt_write_name(out: &mut Vec<u8>, name: &str) {
    out.extend_from_slice(&(name.len() as u16).to_be_bytes());
    out.extend_from_slice(name.as_bytes());
}

fn nbt_write_string(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as u16).to_be_bytes());
    out.extend_from_slice(s.as_bytes());
}

fn login_disconnect(s: &mut TcpStream, reason: &str) -> io::Result<()> {
    let mut d = Vec::new();
    write_string(&mut d, reason);
    send_packet_dbg(s, 0x00, &d, "LoginDisconnect")
}

fn read_packet(s: &mut TcpStream) -> io::Result<(i32, Vec<u8>)> {
    let len = read_varint(s)?;
    let mut payload = vec![0u8; len as usize];
    s.read_exact(&mut payload)?;
    let mut c = Cursor::new(payload);
    Ok((c.read_varint()?, c.read_remaining()))
}

fn drain_until_id(s: &mut TcpStream, target: i32, phase: &str) -> io::Result<()> {
    loop {
        let (id, data) = read_packet(s)?;
        println!("[RECV {}] id=0x{:02X} payload={}B", phase, id, data.len());
        if id == target { return Ok(()); }
    }
}

fn send_packet_dbg(s: &mut TcpStream, id: i32, data: &[u8], name: &str) -> io::Result<()> {
    println!("[SEND] {} id=0x{:02X} payload={}B", name, id, data.len());
    let mut body = Vec::new();
    write_varint(&mut body, id);
    body.extend_from_slice(data);
    let mut out = Vec::new();
    write_varint(&mut out, body.len() as i32);
    out.extend_from_slice(&body);
    s.write_all(&out)?;
    s.flush()
}

fn send_packet(s: &mut TcpStream, id: i32, data: &[u8]) -> io::Result<()> {
    let mut body = Vec::new();
    write_varint(&mut body, id);
    body.extend_from_slice(data);
    let mut out = Vec::new();
    write_varint(&mut out, body.len() as i32);
    out.extend_from_slice(&body);
    s.write_all(&out)?;
    s.flush()
}

fn read_varint(s: &mut TcpStream) -> io::Result<i32> {
    let mut res = 0; let mut shift = 0;
    loop {
        let mut b = [0u8; 1];
        s.read_exact(&mut b)?;
        res |= ((b[0] & 0x7F) as i32) << shift;
        if (b[0] & 0x80) == 0 { return Ok(res); }
        shift += 7;
    }
}

fn write_varint(v: &mut Vec<u8>, mut val: i32) {
    loop {
        let mut b = (val & 0x7F) as u8;
        val = ((val as u32) >> 7) as i32;
        if val != 0 { b |= 0x80; }
        v.push(b);
        if val == 0 { break; }
    }
}

fn write_string(v: &mut Vec<u8>, x: &str) {
    write_varint(v, x.len() as i32);
    v.extend_from_slice(x.as_bytes());
}

struct Cursor { buf: Vec<u8>, pos: usize }
impl Cursor {
    fn new(buf: Vec<u8>) -> Self { Self { buf, pos: 0 } }
    fn read_exact_n(&mut self, n: usize) -> io::Result<&[u8]> {
        let s = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }
    fn read_varint(&mut self) -> io::Result<i32> {
        let mut res = 0; let mut shift = 0;
        loop {
            let b = *self.read_exact_n(1)?.first().unwrap();
            res |= ((b & 0x7F) as i32) << shift;
            if (b & 0x80) == 0 { return Ok(res); }
            shift += 7;
        }
    }
    fn read_u16(&mut self) -> io::Result<u16> { Ok(u16::from_be_bytes(self.read_exact_n(2)?.try_into().unwrap())) }
    fn read_string(&mut self) -> io::Result<String> {
        let len = self.read_varint()?;
        Ok(String::from_utf8_lossy(self.read_exact_n(len as usize)?).into_owned())
    }
    fn read_uuid_or_zeroed(&mut self) -> [u8; 16] {
        let mut out = [0u8; 16];
        if self.pos + 16 <= self.buf.len() { out.copy_from_slice(&self.buf[self.pos..self.pos + 16]); self.pos += 16; }
        out
    }
    fn read_remaining(&mut self) -> Vec<u8> {
        let r = self.buf[self.pos..].to_vec();
        self.pos = self.buf.len();
        r
    }
}