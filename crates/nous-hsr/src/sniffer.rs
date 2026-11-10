//! Passive Windows PktMon adapter. It never opens the game process or sends packets.
use crate::{SnifferStatus, mapscan::MapScan, recorder::Recorder};
use futures::StreamExt;
use nous_core::{Client, Event};
use pktmon::filter::{PktMonFilter, TransportProtocol};
use pktmon::{Capture, PacketPayload};
use reliquary::network::command::command_id as cmd;
use reliquary::network::{ConnectionPacket, GamePacket, GameSniffer};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
const PORTS: [u16; 2] = [23301, 23302];
fn open_capture() -> Result<Capture, String> {
    let mut capture = Capture::new().map_err(|e| format!("PktMon: {e}"))?;
    for port in PORTS {
        capture
            .add_filter(PktMonFilter {
                name: format!("nous hsr udp {port}"),
                transport_protocol: Some(TransportProtocol::UDP),
                port: port.into(),
                ..PktMonFilter::default()
            })
            .map_err(|e| format!("PktMon filter: {e}"))?;
    }
    capture.start().map_err(|e| format!("PktMon start: {e}"))?;
    Ok(capture)
}

pub(crate) async fn run(
    client: Client,
    status: Arc<Mutex<SnifferStatus>>,
    data_dir: Option<PathBuf>,
    mut stop: tokio::sync::oneshot::Receiver<()>,
) -> Result<(), String> {
    let mut packets = open_capture()?
        .stream()
        .map_err(|e| format!("PktMon stream: {e}"))?;
    set_status(&client, &status, SnifferStatus::WaitingForLogin);
    let mut sniffer = GameSniffer::new();
    let mut recorder = Recorder::new(data_dir.clone());
    let mut map_scan = MapScan::new(data_dir);
    let mut component: Option<u16> = None;
    loop {
        let packet = tokio::select! {
            biased;
            _ = &mut stop => return Ok(()),
            packet = packets.next() => packet.ok_or("capture stream ended")?,
        };
        if *component.get_or_insert(packet.component_id) != packet.component_id {
            continue;
        }
        let data = match packet.payload {
            PacketPayload::Ethernet(data) => data,
            PacketPayload::IP(ip) => with_fake_ethernet_header(ip),
            _ => continue,
        };
        let Ok(game_packets) = sniffer.receive_packet(data) else {
            continue;
        };
        for gp in game_packets {
            match gp {
                GamePacket::Connection(ConnectionPacket::Disconnected) => {
                    // the next session may come through another adapter (VPN on/off)
                    component = None;
                    set_status(&client, &status, SnifferStatus::WaitingForLogin)
                }
                GamePacket::Commands {
                    result: Ok(command),
                    ..
                } => {
                    if command.command_id == cmd::PlayerGetTokenScRsp {
                        set_status(&client, &status, SnifferStatus::Connected);
                    }
                    recorder.on_command(&command);
                    map_scan.on_command(&command);
                    // Errors are already published as `sdk-error`; one failed
                    // command (e.g. a locked file on account switch) must not end
                    // the capture: the core retries and drops nothing silently.
                    let _ = client.handle(&crate::decode(&command));
                }
                _ => {}
            }
        }
    }
}
pub(crate) fn set_status(client: &Client, shared: &Mutex<SnifferStatus>, status: SnifferStatus) {
    *shared.lock().unwrap() = status.clone();
    client.publish(Event::new("sniffer-status", status));
}
/// reliquary expects ethernet frames: prefix a bare IP packet with a zeroed
/// ethernet header carrying the right ethertype.
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
