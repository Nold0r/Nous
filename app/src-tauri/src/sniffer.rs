//! Passive capture of the game traffic (PktMon, built into Windows) decrypted
//! with reliquary. Nothing is sent to the game or injected into it.
//! Needs administrator rights; must run before the player logs in, because the
//! session key is only seen during login.

use futures::StreamExt;
use pktmon::filter::{PktMonFilter, TransportProtocol};
use pktmon::{Capture, PacketPayload};
use reliquary::network::command::command_id as cmd;
use reliquary::network::{ConnectionPacket, GamePacket, GameSniffer};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::game::Shared;
use crate::inventory::InventoryChanges;
use crate::recorder::Recorder;

const PORTS: [u16; 2] = [23301, 23302];

/// `inventory-changed` event: the UI refetches on `reset`, applies deltas otherwise.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum InventoryEvent {
    /// Everything was replaced (login, another account).
    Reset,
    Changed(InventoryChanges),
}

/// Shown to the user; emitted as the `sniffer-status` event.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SnifferStatus {
    Starting,
    /// PktMon needs an elevated process.
    NeedsAdmin,
    /// Capturing, but no login seen yet (the game must be started or logged in after Nous).
    WaitingForLogin,
    /// Login seen, commands are being decrypted.
    Connected,
    Error { message: String },
}

pub fn start(app: AppHandle) {
    if !is_elevated::is_elevated() {
        set_status(&app, SnifferStatus::NeedsAdmin);
        return;
    }
    tauri::async_runtime::spawn(async move {
        if let Err(message) = run(&app).await {
            log::error!("sniffer stopped: {message}");
            set_status(&app, SnifferStatus::Error { message });
        }
    });
}

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

async fn run(app: &AppHandle) -> Result<(), String> {
    set_status(app, SnifferStatus::Starting);
    // stream() owns the capture; dropping the stream stops it
    let mut packets = open_capture()?.stream().map_err(|e| format!("PktMon stream: {e}"))?;
    set_status(app, SnifferStatus::WaitingForLogin);

    let mut sniffer = GameSniffer::new();
    let mut recorder = Recorder::new(app.path().app_data_dir().ok());
    // pktmon reports a packet once per component it passes (e.g. VPN adapter + stack)
    let mut component: Option<u16> = None;

    while let Some(packet) = packets.next().await {
        if *component.get_or_insert(packet.component_id) != packet.component_id {
            continue;
        }
        let data = match packet.payload {
            PacketPayload::Ethernet(data) => data,
            // tunnel adapters (VPN, WireGuard) hand out bare IP packets
            PacketPayload::IP(ip) => with_fake_ethernet_header(ip),
            _ => continue,
        };
        let Ok(game_packets) = sniffer.receive_packet(data) else { continue };

        for gp in game_packets {
            match gp {
                GamePacket::Connection(ConnectionPacket::Disconnected) => {
                    set_status(app, SnifferStatus::WaitingForLogin);
                }
                GamePacket::Commands { result: Ok(command), .. } => {
                    if command.command_id == cmd::PlayerGetTokenScRsp {
                        set_status(app, SnifferStatus::Connected);
                    }
                    recorder.on_command(&command);
                    let shared = app.state::<Shared>();
                    let (changes, profile) = {
                        let mut game = shared.game.lock().unwrap();
                        let changes = game.handle(&command);
                        (changes, game.profile().clone())
                    };
                    if changes.profile {
                        app.emit("profile-changed", &profile).ok();
                    }
                    if changes.inventory_reset {
                        app.emit("inventory-changed", InventoryEvent::Reset).ok();
                    } else if !changes.inventory.is_empty() {
                        app.emit("inventory-changed", InventoryEvent::Changed(changes.inventory)).ok();
                    }
                }
                _ => {}
            }
        }
    }
    Err("capture stream ended".into())
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

fn set_status(app: &AppHandle, status: SnifferStatus) {
    log::info!("sniffer: {status:?}");
    *app.state::<Shared>().status.lock().unwrap() = status.clone();
    app.emit("sniffer-status", &status).ok();
}

#[tauri::command]
pub fn get_sniffer_status(shared: State<'_, Shared>) -> SnifferStatus {
    shared.status.lock().unwrap().clone()
}

/// Relaunches Nous elevated (UAC prompt) and closes this instance.
#[tauri::command]
pub fn restart_as_admin(app: AppHandle) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &format!("Start-Process -FilePath '{}' -Verb RunAs", exe.display()),
        ])
        .spawn()
        .map_err(|e| e.to_string())?;
    app.exit(0);
    Ok(())
}
