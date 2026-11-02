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

use crate::game::{Changes, Shared};
use crate::profile::PlayerProfile;
use crate::world::{World, WorldChange};
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
                    let (ch, profile, world) = {
                        let mut game = shared.game.lock().unwrap();
                        let ch = game.handle(&command);
                        let world = (ch.world == WorldChange::Changed).then(|| game.account.world.clone());
                        (ch, game.profile().clone(), world)
                    };
                    emit_changes(app, ch, profile, world);
                }
                _ => {}
            }
        }
    }
    Err("capture stream ended".into())
}

/// Tells the UI what changed. Event names and payloads: docs "API данных игры".
fn emit_changes(app: &AppHandle, ch: Changes, profile: PlayerProfile, world: Option<World>) {
    if ch.profile {
        app.emit("profile-changed", &profile).ok();
    }
    if ch.inventory_reset {
        app.emit("inventory-changed", InventoryEvent::Reset).ok();
    } else if !ch.inventory.is_empty() {
        app.emit("inventory-changed", InventoryEvent::Changed(ch.inventory)).ok();
    }
    if let Some(world) = world {
        app.emit("world-changed", &world).ok();
    } else if ch.world == WorldChange::Moved {
        let player = app.state::<Shared>().game.lock().unwrap().account.world.player;
        app.emit("player-moved", &player).ok();
    }
    if let Some(battle) = ch.battle {
        app.emit("battle-finished", &battle).ok();
    }
    if ch.gacha {
        app.emit("gacha-changed", ()).ok();
    }
    if let Some(name) = ch.snapshot {
        app.emit("snapshot-changed", &name).ok();
    }
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

/// Relaunches Nous elevated. Waits for the UAC answer: on "Yes" this instance
/// quits (the new one waits for that, see `wait_for_previous`), on "No" it
/// keeps running and resolves to false.
#[tauri::command]
pub async fn restart_as_admin(app: AppHandle) -> Result<bool, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let command = format!(
        "Start-Process -FilePath {} -ArgumentList '{}','{}' -Verb RunAs",
        crate::launch::ps_quote(&exe.display().to_string()),
        WAIT_PID_ARG,
        std::process::id()
    );
    let started = tauri::async_runtime::spawn_blocking(move || crate::launch::shell_start(&command))
        .await
        .map_err(|e| e.to_string())??;
    if started {
        app.exit(0);
    }
    Ok(started)
}

/// Argument of an elevated restart: the pid of the instance being replaced.
pub const WAIT_PID_ARG: &str = "--wait-pid";

/// Called first thing at startup: after an elevated restart, waits (up to 15 s)
/// for the old instance to quit, so the single-instance guard doesn't send
/// this new one away.
pub fn wait_for_previous() {
    let args: Vec<String> = std::env::args().collect();
    let Some(pid) = args.iter().position(|a| a == WAIT_PID_ARG).and_then(|i| args.get(i + 1)).and_then(|p| p.parse::<u32>().ok())
    else {
        return;
    };
    let pid = sysinfo::Pid::from_u32(pid);
    let mut sys = sysinfo::System::new();
    for _ in 0..150 {
        sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
        if sys.process(pid).is_none() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
