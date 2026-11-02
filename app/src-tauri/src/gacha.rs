//! Warps (gacha). Three sources, none of them touching the game:
//! - `GetGachaInfoScRsp` at login: current banners;
//! - `DoGachaScRsp`: pulls made while Nous is running;
//! - the full history from the official record API, the same request the
//!   in-game history page makes. Its `authkey` comes from the game's web cache
//!   on disk (`StarRail_Data/webCaches/<ver>/Cache/Cache_Data/data_2`), written
//!   when the player opens the warp history in game; it stays valid ~24 h.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use reliquary::network::command::GameCommand;
use reliquary::network::command::command_id as cmd;
use reliquary::network::command::proto;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::game::Shared;
use crate::launch::LaunchState;

const LOG_API: &str = "https://public-operation-hkrpg-sg.hoyoverse.com/common/hkrpg_gacha_record/api/getGachaLog";
/// Standard, Departure, character event, light cone event, collab character, collab light cone.
const GACHA_TYPES: [u32; 6] = [1, 2, 11, 12, 21, 22];
/// Pause between pages, like a person paging the in-game history.
const PAGE_DELAY: Duration = Duration::from_millis(400);
/// Recent live pulls kept until the official history covers them.
const KEEP_LIVE: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Banner {
    pub gacha_id: u32,
    /// Unix seconds; 0 for permanent banners.
    pub begin: i64,
    pub end: i64,
    /// Featured item ids (rate-up).
    pub featured: Vec<u32>,
    /// 5* item ids of the banner.
    pub prize: Vec<u32>,
}

/// One pull from the official history.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pull {
    /// Record id (string of digits): unique, grows with time.
    pub id: String,
    pub gacha_type: u32,
    pub gacha_id: u32,
    pub item_id: u32,
    /// 3, 4 or 5 stars.
    pub rank: u32,
    /// "Персонажи" / "Световые конусы" (in the language of the request).
    pub item_type: String,
    pub name: String,
    /// Server local time, "YYYY-MM-DD HH:MM:SS".
    pub time: String,
}

/// A pull seen in the traffic (no record id or time from the server).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LivePull {
    pub gacha_id: u32,
    pub item_id: u32,
    pub is_new: bool,
    /// Unix seconds Nous saw it.
    pub at: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Gacha {
    pub banners: Vec<Banner>,
    /// Official history, record id -> pull.
    pub history: BTreeMap<String, Pull>,
    /// Unix seconds of the last history import.
    pub imported_at: Option<i64>,
    pub live: Vec<LivePull>,
}

impl Gacha {
    /// True if the command changed something.
    pub fn handle(&mut self, command: &GameCommand, now: i64) -> bool {
        match command.command_id {
            cmd::GetGachaInfoScRsp => {
                let Ok(r) = command.parse_proto::<proto::GetGachaInfoScRsp::GetGachaInfoScRsp>() else { return false };
                self.banners = r
                    .gacha_info_list
                    .iter()
                    .map(|g| Banner {
                        gacha_id: g.gacha_id,
                        begin: g.begin_time,
                        end: g.end_time,
                        featured: g.item_detail_list.clone(),
                        prize: g.prize_item_list.clone(),
                    })
                    .collect();
                true
            }
            cmd::DoGachaScRsp => {
                let Ok(r) = command.parse_proto::<proto::DoGachaScRsp::DoGachaScRsp>() else { return false };
                for item in &r.gacha_item_list {
                    let Some(id) = item.gacha_item.as_ref().map(|i| i.item_id) else { continue };
                    self.live.push(LivePull { gacha_id: r.gacha_id, item_id: id, is_new: item.is_new, at: now });
                }
                if self.live.len() > KEEP_LIVE {
                    self.live.drain(..self.live.len() - KEEP_LIVE);
                }
                true
            }
            _ => false,
        }
    }

    /// Adds imported pulls; returns how many were new.
    pub fn merge(&mut self, pulls: Vec<Pull>, now: i64) -> usize {
        let before = self.history.len();
        for p in pulls {
            self.history.insert(p.id.clone(), p);
        }
        self.imported_at = Some(now);
        self.history.len() - before
    }
}

/// The newest history URL the in-game page used, from the web cache.
pub fn find_log_url(game_exe: &Path) -> Result<String, String> {
    let caches = game_exe.parent().ok_or("неверный путь к игре")?.join("StarRail_Data").join("webCaches");
    let mut versions: Vec<PathBuf> = std::fs::read_dir(&caches)
        .map_err(|_| "веб-кеш игры не найден: откройте историю прыжков в игре".to_string())?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    // "2.54.0.0" > "2.9.0.0": compare numerically
    versions.sort_by_key(|p| {
        p.file_name().and_then(|n| n.to_str()).map(|n| n.split('.').map(|x| x.parse::<u32>().unwrap_or(0)).collect::<Vec<_>>())
    });
    for dir in versions.iter().rev() {
        let Ok(data) = std::fs::read(dir.join("Cache").join("Cache_Data").join("data_2")) else { continue };
        if let Some(url) = last_log_url(&data) {
            return Ok(url);
        }
    }
    Err("в веб-кеше нет ссылки на историю: откройте историю прыжков в игре".into())
}

fn last_log_url(data: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(data);
    let start = text.rfind("https://public-operation-hkrpg")?;
    let tail = &text[start..];
    let end = tail.find(|c: char| c == '\0' || c.is_whitespace() || c == '"').unwrap_or(tail.len());
    let url = &tail[..end];
    (url.contains("getGachaLog") && url.contains("authkey=")).then(|| url.to_string())
}

#[derive(Deserialize)]
struct LogResponse {
    retcode: i32,
    message: String,
    data: Option<LogPage>,
}

#[derive(Deserialize)]
struct LogPage {
    list: Vec<LogEntry>,
}

#[derive(Deserialize)]
struct LogEntry {
    uid: String,
    id: String,
    gacha_type: String,
    gacha_id: String,
    item_id: String,
    rank_type: String,
    item_type: String,
    name: String,
    time: String,
}

/// Downloads the history for `uid`, stopping at pulls already known.
pub fn download(log_url: &str, uid: u32, known: &BTreeMap<String, Pull>) -> Result<Vec<Pull>, String> {
    let (_, query) = log_url.split_once('?').ok_or("ссылка без параметров")?;
    // keep the auth parameters, replace paging ones
    let base: Vec<(&str, &str)> = query
        .split('&')
        .filter_map(|kv| kv.split_once('='))
        .filter(|(k, _)| !matches!(*k, "gacha_type" | "end_id" | "size" | "page" | "default_gacha_type"))
        .collect();

    let mut pulls = Vec::new();
    for gacha_type in GACHA_TYPES {
        let mut end_id = "0".to_string();
        'pages: loop {
            let mut url = format!("{LOG_API}?gacha_type={gacha_type}&size=20&end_id={end_id}");
            for (k, v) in &base {
                url.push_str(&format!("&{k}={v}"));
            }
            let rsp: LogResponse = ureq::get(&url)
                .call()
                .map_err(|e| format!("сеть: {e}"))?
                .body_mut()
                .read_json()
                .map_err(|e| format!("ответ: {e}"))?;
            match rsp.retcode {
                0 => {}
                -101 => return Err("ключ истории устарел: откройте историю прыжков в игре и повторите".into()),
                // collab banners are absent on some versions
                -100 if gacha_type >= 21 => break 'pages,
                code => return Err(format!("API вернул {code}: {}", rsp.message)),
            }
            let list = rsp.data.map(|d| d.list).unwrap_or_default();
            if list.is_empty() {
                break;
            }
            for e in &list {
                if e.uid != uid.to_string() {
                    return Err(format!("история другого аккаунта (UID {})", e.uid));
                }
                if known.contains_key(&e.id) {
                    // pages go newest first: everything older is already stored
                    break 'pages;
                }
                pulls.push(Pull {
                    id: e.id.clone(),
                    gacha_type: e.gacha_type.parse().unwrap_or(gacha_type),
                    gacha_id: e.gacha_id.parse().unwrap_or(0),
                    item_id: e.item_id.parse().unwrap_or(0),
                    rank: e.rank_type.parse().unwrap_or(0),
                    item_type: e.item_type.clone(),
                    name: e.name.clone(),
                    time: e.time.clone(),
                });
            }
            end_id = list.last().map(|e| e.id.clone()).unwrap_or_default();
            std::thread::sleep(PAGE_DELAY);
        }
    }
    Ok(pulls)
}

/// Imports the full warp history of the current account through the official
/// API (see the module docs). Returns how many pulls were new.
#[tauri::command]
pub async fn import_gacha_history(app: AppHandle) -> Result<usize, String> {
    let exe = app.state::<LaunchState>().game_path().ok_or("путь к игре неизвестен")?;
    let shared = app.state::<Shared>();
    let (uid, known) = {
        let game = shared.game.lock().unwrap();
        let uid = game.account.uid.ok_or("сначала войдите в игру, чтобы Nous узнал аккаунт")?;
        (uid, game.account.gacha.history.clone())
    };
    let url = find_log_url(Path::new(&exe))?;
    let pulls = tauri::async_runtime::spawn_blocking(move || download(&url, uid, &known))
        .await
        .map_err(|e| e.to_string())??;

    let added = {
        let mut game = shared.game.lock().unwrap();
        if game.account.uid != Some(uid) {
            return Err("аккаунт сменился во время загрузки".into());
        }
        let added = game.account.gacha.merge(pulls, crate::profile::now());
        game.save();
        added
    };
    app.emit("gacha-changed", ()).ok();
    Ok(added)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real request with the key from this PC's web cache:
    /// `NOUS_GAME_EXE=<StarRail.exe> NOUS_UID=<uid> cargo test --lib live_history -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn live_history() {
        let exe = std::env::var("NOUS_GAME_EXE").unwrap();
        let uid: u32 = std::env::var("NOUS_UID").unwrap().parse().unwrap();
        let url = find_log_url(Path::new(&exe)).unwrap();
        let pulls = download(&url, uid, &BTreeMap::new()).unwrap();
        let mut by_type = BTreeMap::new();
        for p in &pulls {
            *by_type.entry(p.gacha_type).or_insert(0) += 1;
        }
        let five = pulls.iter().filter(|p| p.rank == 5).count();
        println!("{} pulls by type {by_type:?}, {five} five-star", pulls.len());
    }

    #[test]
    fn finds_the_last_history_url_in_the_cache() {
        let mut data = b"junk\0https://public-operation-hkrpg-sg.hoyoverse.com/common/hkrpg_gacha_record/api/getGachaLog?authkey=OLD&lang=ru\0more"
            .to_vec();
        data.extend_from_slice(b"\0https://public-operation-hkrpg-sg.hoyoverse.com/common/hkrpg_gacha_record/api/getGachaLog?authkey=NEW&lang=ru\0tail");
        let url = last_log_url(&data).unwrap();
        assert!(url.ends_with("authkey=NEW&lang=ru"));
        assert!(last_log_url(b"nothing here").is_none());
    }

    #[test]
    fn merge_counts_new_pulls_only() {
        let pull = |id: &str| Pull {
            id: id.into(),
            gacha_type: 11,
            gacha_id: 0,
            item_id: 1310,
            rank: 5,
            item_type: String::new(),
            name: String::new(),
            time: String::new(),
        };
        let mut g = Gacha::default();
        assert_eq!(g.merge(vec![pull("1"), pull("2")], 1), 2);
        assert_eq!(g.merge(vec![pull("2"), pull("3")], 2), 1);
        assert_eq!(g.history.len(), 3);
    }
}
