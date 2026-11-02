//! Throwaway spike: capture HSR traffic with PktMon, decrypt it with reliquary
//! and dump every game command to disk. Map/chest related commands are also
//! decoded to readable JSON so we can see what the server actually sends.
//!
//! Must run as administrator (PktMon requirement). Start it BEFORE logging in,
//! the session key is only learned from the login handshake.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use base64::prelude::*;
use futures::StreamExt;
use pktmon::filter::{PktMonFilter, TransportProtocol};
use pktmon::{Capture, PacketPayload};
use reliquary::network::command::proto;
use reliquary::network::command::{GameCommand, command_id};
use reliquary::network::{ConnectionPacket, GamePacket, GameSniffer};

const PORTS: [u16; 2] = [23301, 23302];

struct Dumper {
    dir: PathBuf,
    all: File,
    counter: u32,
}

impl Dumper {
    fn new() -> Self {
        // next to the exe, so the location doesn't depend on how it was launched
        let base = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .unwrap_or_default();
        let dir = base.join("dumps").join(chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string());
        fs::create_dir_all(dir.join("decoded")).expect("create dump dir");
        let all = OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("all_commands.jsonl"))
            .expect("open all_commands.jsonl");
        println!("dumping to {}", dir.display());
        Self { dir, all, counter: 0 }
    }

    fn write_raw(&mut self, conv_id: u32, cmd: &GameCommand) {
        let line = serde_json::json!({
            "t": chrono::Local::now().to_rfc3339(),
            "conv": conv_id,
            "id": cmd.command_id,
            "name": cmd.get_command_name(),
            "len": cmd.proto_data.len(),
            "b64": BASE64_STANDARD.encode(&cmd.proto_data),
        });
        writeln!(self.all, "{line}").ok();
    }

    fn write_decoded(&mut self, name: &str, json: String) -> PathBuf {
        self.counter += 1;
        let path = self.dir.join("decoded").join(format!("{:04}_{name}.json", self.counter));
        fs::write(&path, json).ok();
        path
    }
}

fn to_json<M: protobuf::MessageFull>(msg: &M) -> String {
    let compact = protobuf_json_mapping::print_to_string(msg).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"));
    // re-indent for humans
    serde_json::from_str::<serde_json::Value>(&compact)
        .and_then(|v| serde_json::to_string_pretty(&v))
        .unwrap_or(compact)
}

/// Decode the commands we care about and print a short summary.
fn handle_interesting(dumper: &mut Dumper, cmd: &GameCommand) {
    macro_rules! decode {
        ($ty:ident) => {{
            match cmd.parse_proto::<proto::$ty::$ty>() {
                Ok(msg) => {
                    let path = dumper.write_decoded(stringify!($ty), to_json(&msg));
                    Some((msg, path))
                }
                Err(e) => {
                    eprintln!("failed to parse {}: {e}", stringify!($ty));
                    None
                }
            }
        }};
    }

    match cmd.command_id {
        command_id::GetSceneMapInfoScRsp => {
            if let Some((msg, path)) = decode!(GetSceneMapInfoScRsp) {
                println!("[map] GetSceneMapInfoScRsp: {} floor(s) -> {}", msg.scene_map_info.len(), path.display());
                for info in &msg.scene_map_info {
                    let chests: Vec<String> = info
                        .chest_list
                        .iter()
                        .map(|c| format!("{:?} {}/{}", c.chest_type.enum_value_or_default(), c.opened_num, c.exist_num))
                        .collect();
                    println!(
                        "      floor {} entry {}: chests [{}], opened_chest_ids {}, maze_props {}, groups {}",
                        info.floor_id,
                        info.entry_id,
                        chests.join(", "),
                        info.opened_chest_id_list.len(),
                        info.maze_prop_list.len(),
                        info.maze_group_list.len(),
                    );
                }
            }
        }
        command_id::GetSceneMapInfoCsReq => {
            decode!(GetSceneMapInfoCsReq);
        }
        command_id::OpenChestScNotify => {
            if let Some((msg, _)) = decode!(OpenChestScNotify) {
                println!("[chest] OpenChestScNotify chest_id={}", msg.chest_id);
            }
        }
        command_id::GetMarkChestScRsp => {
            decode!(GetMarkChestScRsp);
        }
        command_id::MarkChestChangedScNotify => {
            decode!(MarkChestChangedScNotify);
        }
        _ => {}
    }
}

/// reliquary expects ethernet frames; prefix a bare IP packet with a zeroed
/// ethernet header carrying the right ethertype (same trick reliquary-archiver
/// uses for RAW pcap files).
fn with_fake_ethernet_header(ip: Vec<u8>) -> Vec<u8> {
    let ethertype: u16 = match ip.first().map(|b| b >> 4) {
        Some(6) => 0x86DD,
        _ => 0x0800,
    };
    let mut framed = Vec::with_capacity(14 + ip.len());
    framed.extend_from_slice(&[0u8; 12]);
    framed.extend_from_slice(&ethertype.to_be_bytes());
    framed.extend_from_slice(&ip);
    framed
}

fn start_capture() -> Capture {
    let mut capture = Capture::new().expect("PktMon init failed (are you running as administrator?)");
    for port in PORTS {
        capture
            .add_filter(PktMonFilter {
                name: format!("hsr udp {port}"),
                transport_protocol: Some(TransportProtocol::UDP),
                port: port.into(),
                ..PktMonFilter::default()
            })
            .expect("add pktmon filter");
    }
    capture.start().expect("start pktmon capture");
    capture
}

/// Counters to see at which layer things break: capture, connection/kcp parsing or decryption.
#[derive(Default)]
struct Stats {
    raw_packets: u64,
    non_ethernet: u64,
    commands_ok: u64,
    errors: BTreeMap<String, u64>,
}

impl Stats {
    fn error(&mut self, kind: &str, msg: String) {
        // strip numbers so similar errors group together
        let key = format!("{kind}: {}", msg.chars().filter(|c| !c.is_ascii_digit()).collect::<String>());
        let n = self.errors.entry(key.clone()).or_default();
        *n += 1;
        if *n == 1 {
            eprintln!("[err] first {key}  (full: {msg})");
        }
    }

    fn print(&self) {
        println!(
            "[stats] raw packets {}, non-ethernet {}, commands decoded {}",
            self.raw_packets, self.non_ethernet, self.commands_ok
        );
        for (k, v) in &self.errors {
            println!("        {v:>6} x {k}");
        }
    }
}

#[tokio::main]
async fn main() {
    let mut dumper = Dumper::new();

    // reliquary's own debug logs (game version detection, handshake, key setup) go to a file
    let log = File::create(dumper.dir.join("reliquary.log")).expect("create reliquary.log");
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("reliquary=debug"))
        .with_writer(std::sync::Mutex::new(log))
        .with_ansi(false)
        .init();

    let mut sniffer = GameSniffer::new();
    let mut stats = Stats::default();
    let mut ticker = tokio::time::interval(std::time::Duration::from_secs(5));
    let mut locked_component: Option<u16> = None;

    // stream() takes ownership of the capture; dropping the stream stops it
    let mut packets = start_capture().stream().expect("pktmon stream");

    println!("capturing. Start the game (or log out to the 'Click to Start' screen), then log in.");
    println!("Open the map and browse areas. Ctrl+C to stop.");

    loop {
        let packet = tokio::select! {
            p = packets.next() => match p {
                Some(p) => p,
                None => break,
            },
            _ = ticker.tick() => {
                stats.print();
                continue;
            }
            _ = tokio::signal::ctrl_c() => break,
        };

        stats.raw_packets += 1;

        // pktmon sees every packet once per component it passes through (e.g. the
        // WireGuard adapter and the stack above it); stick to the first one we see
        let component = *locked_component.get_or_insert(packet.component_id);
        if packet.component_id != component {
            continue;
        }

        let data = match packet.payload {
            PacketPayload::Ethernet(data) => data,
            // tunnel adapters (VPN/WireGuard) hand out bare IP packets
            PacketPayload::IP(ip) => with_fake_ethernet_header(ip),
            _ => {
                stats.non_ethernet += 1;
                continue;
            }
        };

        let game_packets = match sniffer.receive_packet(data) {
            Ok(p) => p,
            Err(e) => {
                stats.error("packet", e.to_string());
                continue;
            }
        };

        for gp in game_packets {
            match gp {
                GamePacket::Connection(ConnectionPacket::HandshakeEstablished { conv_id }) => {
                    println!("[net] connection established (conv {conv_id})");
                }
                GamePacket::Connection(ConnectionPacket::Disconnected) => println!("[net] disconnected"),
                GamePacket::Connection(_) => {}
                GamePacket::Commands { conv_id, result } => match result {
                    Ok(cmd) => {
                        stats.commands_ok += 1;
                        if cmd.command_id == command_id::PlayerGetTokenScRsp {
                            println!("[net] login seen, session key acquired");
                        }
                        dumper.write_raw(conv_id, &cmd);
                        handle_interesting(&mut dumper, &cmd);
                    }
                    Err(e) => stats.error("command", e.to_string()),
                },
            }
        }
    }

    drop(packets);
    stats.print();
    println!("stopped. Output: {}", dumper.dir.display());
}
