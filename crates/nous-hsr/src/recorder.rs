//! Records the login phase of each game session (decrypted commands, from the
//! token to the full inventory lists) to `<app data>/recordings/`. Tests replay
//! these offline (see tests in replay.rs), so checking the data code doesn't
//! need a fresh login every time. Same line format as spike/hsr-sniff dumps.
//!
//! Recordings hold the account's data: they stay on this computer.

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use base64::Engine;
use reliquary::network::command::GameCommand;
use reliquary::network::command::command_id as cmd;

pub const DIR: &str = "recordings";
/// Older recordings are deleted beyond this many.
const KEEP: usize = 20;
/// Safety stop if the login never completes.
const MAX_DURATION: Duration = Duration::from_secs(60);

pub struct Recorder {
    dir: Option<PathBuf>,
    current: Option<Recording>,
}

struct Recording {
    file: BufWriter<File>,
    started: Instant,
    bag: bool,
    avatars: bool,
    finished: bool,
}

impl Recorder {
    pub fn new(app_data: Option<PathBuf>) -> Self {
        Self {
            dir: app_data.map(|d| d.join(DIR)),
            current: None,
        }
    }

    pub fn on_command(&mut self, command: &GameCommand) {
        if command.command_id == cmd::PlayerGetTokenScRsp {
            self.current = self.start();
        }
        let Some(rec) = &mut self.current else { return };

        let line = serde_json::json!({
            "id": command.command_id,
            "name": command.get_command_name(),
            "b64": base64::engine::general_purpose::STANDARD.encode(&command.proto_data),
        });
        writeln!(rec.file, "{line}").ok();

        match command.command_id {
            cmd::GetBagScRsp => rec.bag = true,
            cmd::GetAvatarDataScRsp => rec.avatars = true,
            cmd::PlayerLoginFinishScRsp => rec.finished = true,
            _ => {}
        }
        if (rec.bag && rec.avatars && rec.finished) || rec.started.elapsed() > MAX_DURATION {
            rec.file.flush().ok();
            log::info!("login recorded (complete: {})", rec.bag && rec.avatars);
            self.current = None;
        }
    }

    fn start(&self) -> Option<Recording> {
        let dir = self.dir.as_ref()?;
        fs::create_dir_all(dir).ok()?;
        prune(dir);
        let name = chrono_like_now();
        let file = File::create(dir.join(format!("{name}.jsonl"))).ok()?;
        Some(Recording {
            file: BufWriter::new(file),
            started: Instant::now(),
            bag: false,
            avatars: false,
            finished: false,
        })
    }
}

/// Keeps the newest KEEP - 1 recordings (one is about to be added).
fn prune(dir: &PathBuf) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<_> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "jsonl"))
        .collect();
    files.sort();
    let excess = (files.len() + 1).saturating_sub(KEEP);
    for old in files.into_iter().take(excess) {
        fs::remove_file(old).ok();
    }
}

/// `YYYYMMDD-HHMMSS` (UTC): sortable file names without a date crate.
fn chrono_like_now() -> String {
    let secs = nous_core::profile::now();
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // civil-from-days (Howard Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}{m:02}{d:02}-{:02}{:02}{:02}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(id: u16, data: &[u8]) -> GameCommand {
        GameCommand {
            command_id: id,
            header_len: 0,
            data_len: data.len() as u32,
            proto_header: vec![],
            proto_data: data.to_vec(),
        }
    }

    #[test]
    fn records_from_token_to_login_finish() {
        let dir = std::env::temp_dir().join(format!("nous-recorder-test-{}", std::process::id()));
        let mut rec = Recorder::new(Some(dir.clone()));
        rec.on_command(&command(cmd::GetBagScRsp, b"before login: not recorded"));
        for id in [
            cmd::PlayerGetTokenScRsp,
            cmd::GetBagScRsp,
            cmd::GetAvatarDataScRsp,
            cmd::PlayerLoginFinishScRsp,
        ] {
            rec.on_command(&command(id, b"x"));
        }
        rec.on_command(&command(cmd::PlayerSyncScNotify, b"after: not recorded"));
        assert!(rec.current.is_none(), "stopped after the login phase");

        let files: Vec<_> = fs::read_dir(dir.join(DIR)).unwrap().flatten().collect();
        assert_eq!(files.len(), 1);
        let text = fs::read_to_string(files[0].path()).unwrap();
        let names: Vec<_> = text
            .lines()
            .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap()["name"].clone())
            .collect();
        assert_eq!(names.len(), 4);
        assert_eq!(names[0], "PlayerGetTokenScRsp");
        assert_eq!(names[3], "PlayerLoginFinishScRsp");
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn file_name_is_a_utc_timestamp() {
        let name = super::chrono_like_now();
        assert_eq!(name.len(), 15);
        assert!(name.starts_with("20"));
    }
}
