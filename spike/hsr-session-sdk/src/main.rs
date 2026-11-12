use base64::Engine;
use etherparse::{NetSlice, SlicedPacket, TransportSlice};
use futures::StreamExt;
use hsr_session_sdk::{GetCurrentScene, GetInventory, SessionRequester, TrafficListener};
use pktmon::{
    Capture, PacketPayload,
    filter::{PktMonFilter, TransportProtocol},
};
use reliquary::network::{ConnectionPacket, GamePacket, GameSniffer, command::GameCommand};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, HashMap},
    fs,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    time::Duration,
};

#[derive(Default)]
struct Stats {
    frames: u64,
    commands: BTreeMap<String, u64>,
    parse_errors: u64,
    rejected: u64,
    confirmed: u64,
}
fn ingest(
    listener: &TrafficListener,
    flow: u64,
    conv: u32,
    command: &GameCommand,
    stats: &mut Stats,
) {
    let name = command.get_command_name().unwrap_or("<unknown>");
    *stats.commands.entry(name.to_owned()).or_default() += 1;
    let previous = listener.active_session();
    if listener.ingest_on_flow(flow, conv, command).is_err() {
        stats.rejected += 1;
    }
    if listener.active_session().is_some() && listener.active_session() != previous {
        stats.confirmed += 1;
    }
}
fn game_detected() -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        return std::process::Command::new("tasklist.exe")
            .args(["/FI", "IMAGENAME eq StarRail.exe", "/FO", "CSV", "/NH"])
            .creation_flags(0x08000000)
            .output()
            .ok()
            .is_some_and(|out| String::from_utf8_lossy(&out.stdout).contains("StarRail.exe"));
    }
    #[cfg(not(windows))]
    {
        false
    }
}
fn replay(path: &PathBuf, listener: &TrafficListener, stats: &mut Stats) -> Result<(), String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    for line in text.lines() {
        let row: Value = match serde_json::from_str(line) {
            Ok(row) => row,
            Err(_) => {
                stats.parse_errors += 1;
                continue;
            }
        };
        let (Some(id), Some(conv), Some(b64)) = (
            row["id"].as_u64(),
            row["conv"].as_u64(),
            row["b64"].as_str(),
        ) else {
            stats.parse_errors += 1;
            continue;
        };
        let body = match base64::prelude::BASE64_STANDARD.decode(b64) {
            Ok(body) => body,
            Err(_) => {
                stats.parse_errors += 1;
                continue;
            }
        };
        let (Ok(id), Ok(conv)) = (u16::try_from(id), u32::try_from(conv)) else {
            stats.parse_errors += 1;
            continue;
        };
        let command = GameCommand {
            command_id: id,
            header_len: 0,
            data_len: body.len() as u32,
            proto_header: vec![],
            proto_data: body,
        };
        ingest(listener, 0, conv, &command, stats);
    }
    Ok(())
}

#[derive(Hash, PartialEq, Eq)]
struct Flow {
    client: SocketAddr,
    server: SocketAddr,
}
fn flow_for(bytes: &[u8]) -> Option<Flow> {
    let frame = SlicedPacket::from_ethernet(bytes).ok()?;
    let TransportSlice::Udp(udp) = frame.transport? else {
        return None;
    };
    let (src, dst) = match frame.net? {
        NetSlice::Ipv4(ip) => (
            IpAddr::V4(ip.header().source_addr()),
            IpAddr::V4(ip.header().destination_addr()),
        ),
        NetSlice::Ipv6(ip) => (
            IpAddr::V6(ip.header().source_addr()),
            IpAddr::V6(ip.header().destination_addr()),
        ),
        _ => return None,
    };
    if [23301, 23302].contains(&udp.source_port()) {
        Some(Flow {
            client: SocketAddr::new(dst, udp.destination_port()),
            server: SocketAddr::new(src, udp.source_port()),
        })
    } else if [23301, 23302].contains(&udp.destination_port()) {
        Some(Flow {
            client: SocketAddr::new(src, udp.source_port()),
            server: SocketAddr::new(dst, udp.destination_port()),
        })
    } else {
        None
    }
}
async fn capture(
    seconds: u64,
    listener: &TrafficListener,
    stats: &mut Stats,
) -> Result<(), String> {
    let mut capture = Capture::new().map_err(|e| e.to_string())?;
    for port in [23301, 23302] {
        capture
            .add_filter(PktMonFilter {
                name: format!("nous-session-{port}"),
                transport_protocol: Some(TransportProtocol::UDP),
                port: port.into(),
                ..Default::default()
            })
            .map_err(|e| e.to_string())?;
    }
    capture.start().map_err(|e| e.to_string())?;
    let mut stream = capture.stream().map_err(|e| e.to_string())?;
    let deadline = tokio::time::sleep(Duration::from_secs(seconds));
    tokio::pin!(deadline);
    let mut flows: HashMap<Flow, (u64, GameSniffer)> = HashMap::new();
    let mut component = None;
    loop {
        let frame = tokio::select! {
            _ = &mut deadline => break,
            packet = stream.next() => match packet { Some(frame) => frame, None => break },
        };
        if *component.get_or_insert(frame.component_id) != frame.component_id {
            continue;
        }
        stats.frames += 1;
        let bytes = match frame.payload {
            PacketPayload::Ethernet(bytes) => bytes,
            PacketPayload::IP(bytes) => {
                let ether_type: u16 = if bytes.first().is_some_and(|b| b >> 4 == 6) {
                    0x86dd
                } else {
                    0x0800
                };
                let mut framed = vec![0; 12];
                framed.extend(ether_type.to_be_bytes());
                framed.extend(bytes);
                framed
            }
            _ => continue,
        };
        let Some(flow) = flow_for(&bytes) else {
            stats.parse_errors += 1;
            continue;
        };
        if flows.len() >= 32 && !flows.contains_key(&flow) {
            stats.rejected += 1;
            continue;
        }
        let next_id = flows.len() as u64 + 1;
        let (flow_id, sniffer) = flows
            .entry(flow)
            .or_insert_with(|| (next_id, GameSniffer::new()));
        // The upstream decoder contains panic paths. A failed decoder never leaves a confirmed session alive.
        let packets = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            sniffer.receive_packet(bytes)
        })) {
            Ok(Ok(packets)) => packets,
            _ => {
                stats.parse_errors += 1;
                listener.invalidate_flow(*flow_id);
                *sniffer = GameSniffer::new();
                continue;
            }
        };
        for packet in packets {
            match packet {
                GamePacket::Connection(
                    ConnectionPacket::Disconnected | ConnectionPacket::HandshakeEstablished { .. },
                ) => listener.invalidate_flow(*flow_id),
                GamePacket::Commands {
                    conv_id,
                    result: Ok(command),
                } => ingest(listener, *flow_id, conv_id, &command, stats),
                GamePacket::Commands { result: Err(_), .. } => stats.parse_errors += 1,
                _ => {}
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let is_live = args.iter().any(|a| a == "--live");
    let input = value("--replay").map(PathBuf::from);
    if is_live == input.is_some() {
        eprintln!(
            "Use --live --seconds 90 --out report.json OR --replay capture.jsonl --out report.json"
        );
        std::process::exit(2);
    }
    let listener = if is_live {
        TrafficListener::new()
    } else {
        TrafficListener::replay()
    };
    let mut stats = Stats::default();
    let game_before = game_detected();
    let outcome = if is_live {
        capture(
            value("--seconds")
                .and_then(|s| s.parse().ok())
                .unwrap_or(90),
            &listener,
            &mut stats,
        )
        .await
    } else {
        replay(&input.unwrap(), &listener, &mut stats)
    };
    let session = listener.active_session();
    let scene = session
        .as_ref()
        .and_then(|s| listener.read::<GetCurrentScene>(s).ok().flatten());
    let inventory = session
        .as_ref()
        .and_then(|s| listener.read::<GetInventory>(s).ok().flatten());
    let requester = SessionRequester::new(listener.clone());
    let probe = match session.as_ref() {
        Some(session) => requester
            .query(session, GetCurrentScene, Duration::from_millis(100))
            .await
            .map(|_| "reply".to_owned())
            .unwrap_or_else(|e| e.to_string()),
        None => "NoSession".to_owned(),
    };
    let report = json!({
        "mode": if is_live { "live_passive_capture" } else { "offline_replay" },
        "game_process_before": game_before, "game_process_after": game_detected(),
        "capture_or_replay_error": outcome.as_ref().err(), "frames": stats.frames,
        "decoded_commands": stats.commands.values().sum::<u64>(), "command_counts": stats.commands,
        "confirmed_session_count": stats.confirmed, "session_at_end": session.is_some(),
        "parse_errors": stats.parse_errors, "rejected_commands": stats.rejected,
        "scene_response_observed": scene.is_some(), "inventory_response_observed": inventory.is_some(),
        "requester_probe": probe, "official_game_requests_sent": 0,
        "privacy": "No UID, endpoint addresses, tokens, keys or payload values exported.",
        "limits": ["Port-based capture does not establish PID ownership.", "Capture must see login to learn the key.", "No synchronized official KCP transport is implemented."]
    });
    let text = serde_json::to_string_pretty(&report).unwrap();
    if let Some(path) = value("--out") {
        if let Err(error) = fs::write(path, &text) {
            eprintln!("Could not write report: {error}");
            std::process::exit(1)
        }
    }
    println!("{text}");
    if outcome.is_err() {
        std::process::exit(1)
    }
}
