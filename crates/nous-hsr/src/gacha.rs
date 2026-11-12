use nous_core::gacha::*;
use reliquary::network::command::{GameCommand, command_id as cmd};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
const LOG_HOST_PREFIX: &str = "https://public-operation-hkrpg";
const LOG_DOMAINS: [&str; 2] = [".hoyoverse.com", ".mihoyo.com"];
/// A stalled connection must not leave the import button busy forever.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
/// Standard, Departure, character event, light cone event, collab character, collab light cone.
const GACHA_TYPES: [u32; 6] = [1, 2, 11, 12, 21, 22];
/// Pause between pages, like a person paging the in-game history.
const PAGE_DELAY: Duration = Duration::from_millis(400);
pub(crate) enum Update {
    Banners(Vec<Banner>),
    Pulls(Vec<LivePull>),
}
pub(crate) fn decode(command: &GameCommand, now: i64) -> Option<Update> {
    match command.command_id {
        cmd::GetGachaInfoScRsp => {
            let r = rsp!(command, GetGachaInfoScRsp)?;
            Some(Update::Banners(
                r.gacha_info_list
                    .iter()
                    .map(|g| Banner {
                        gacha_id: g.gacha_id,
                        begin: g.begin_time,
                        end: g.end_time,
                        featured: g.item_detail_list.clone(),
                        prize: g.prize_item_list.clone(),
                    })
                    .collect(),
            ))
        }
        cmd::DoGachaScRsp => {
            let r = rsp!(command, DoGachaScRsp)?;
            Some(Update::Pulls(
                r.gacha_item_list
                    .iter()
                    .filter_map(|i| {
                        Some(LivePull {
                            gacha_id: r.gacha_id,
                            item_id: i.gacha_item.as_ref()?.item_id,
                            is_new: i.is_new,
                            at: now,
                        })
                    })
                    .collect(),
            ))
        }
        _ => None,
    }
}

/// The newest history URL the in-game page used, from the web cache.
pub fn find_log_url(game_exe: &Path) -> Result<String, String> {
    let caches = game_exe
        .parent()
        .ok_or("неверный путь к игре")?
        .join("StarRail_Data")
        .join("webCaches");
    let mut versions: Vec<PathBuf> = std::fs::read_dir(&caches)
        .map_err(|_| "веб-кеш игры не найден: откройте историю прыжков в игре".to_string())?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    // "2.54.0.0" > "2.9.0.0": compare numerically
    versions.sort_by_key(|p| {
        p.file_name().and_then(|n| n.to_str()).map(|n| {
            n.split('.')
                .map(|x| x.parse::<u32>().unwrap_or(0))
                .collect::<Vec<_>>()
        })
    });
    for dir in versions.iter().rev() {
        let Ok(data) = std::fs::read(dir.join("Cache").join("Cache_Data").join("data_2")) else {
            continue;
        };
        if let Some(url) = last_log_url(&data) {
            return Ok(url);
        }
    }
    Err("в веб-кеше нет ссылки на историю: откройте историю прыжков в игре".into())
}

/// The newest history link in the cache. Other pages of the same server opened
/// later (banner details) sit after it, so every link is looked at, last first.
fn last_log_url(data: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(data);
    text.rmatch_indices(LOG_HOST_PREFIX).find_map(|(start, _)| {
        let tail = &text[start..];
        let end = tail
            .find(|c: char| c == '\0' || c.is_whitespace() || c == '"')
            .unwrap_or(tail.len());
        let url = &tail[..end];
        (url.contains("getGachaLog") && url.contains("authkey=")).then(|| url.to_string())
    })
}

/// The API address without the query, on the server the game itself used
/// (global and Chinese accounts live on different hosts). The key is sent only
/// to the publisher's own domains.
fn api_base(log_url: &str) -> Result<String, String> {
    let address = log_url.split(['?', '#']).next().unwrap_or_default();
    let host = address
        .strip_prefix("https://")
        .and_then(|rest| rest.split('/').next())
        .unwrap_or_default();
    let trusted = host.starts_with("public-operation-hkrpg")
        && LOG_DOMAINS.iter().any(|domain| host.ends_with(domain));
    if trusted && address.ends_with("/getGachaLog") {
        Ok(address.to_string())
    } else {
        Err("ссылка на историю ведёт не на сервер игры".into())
    }
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
pub fn download(
    log_url: &str,
    uid: u32,
    known: &BTreeMap<String, Pull>,
) -> Result<Vec<Pull>, String> {
    let (_, query) = log_url.split_once('?').ok_or("ссылка без параметров")?;
    // keep the auth parameters, replace paging ones
    let base: Vec<(&str, &str)> = query
        .split('&')
        .filter_map(|kv| kv.split_once('='))
        .filter(|(k, _)| {
            !matches!(
                *k,
                "gacha_type" | "end_id" | "size" | "page" | "default_gacha_type"
            )
        })
        .collect();

    let api = api_base(log_url)?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(REQUEST_TIMEOUT))
        .build()
        .into();
    let mut pulls = Vec::new();
    for gacha_type in GACHA_TYPES {
        let mut end_id = "0".to_string();
        'pages: loop {
            let mut url = format!("{api}?gacha_type={gacha_type}&size=20&end_id={end_id}");
            for (k, v) in &base {
                url.push_str(&format!("&{k}={v}"));
            }
            let rsp: LogResponse = agent
                .get(&url)
                .call()
                .map_err(|e| format!("сеть: {e}"))?
                .body_mut()
                .read_json()
                .map_err(|e| format!("ответ: {e}"))?;
            match rsp.retcode {
                0 => {}
                -101 => {
                    return Err(
                        "ключ истории устарел: откройте историю прыжков в игре и повторите".into(),
                    );
                }
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
        println!(
            "{} pulls by type {by_type:?}, {five} five-star",
            pulls.len()
        );
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
    fn another_page_opened_after_the_history_does_not_hide_it() {
        let data = b"\0https://public-operation-hkrpg.mihoyo.com/common/hkrpg_gacha_record/api/getGachaLog?authkey=KEY&lang=ru\0https://public-operation-hkrpg-sg.hoyoverse.com/common/gacha_info/api/unrelated\0";
        let url = last_log_url(data).expect("the history link is still found");
        assert!(url.ends_with("authkey=KEY&lang=ru"));
    }

    #[test]
    fn the_history_is_asked_from_the_server_the_game_used() {
        assert_eq!(
            api_base("https://public-operation-hkrpg.mihoyo.com/common/hkrpg_gacha_record/api/getGachaLog?authkey=KEY").unwrap(),
            "https://public-operation-hkrpg.mihoyo.com/common/hkrpg_gacha_record/api/getGachaLog"
        );
        assert_eq!(api_base("https://public-operation-hkrpg-sg.hoyoverse.com/x/getGachaLog?a=1").unwrap(), "https://public-operation-hkrpg-sg.hoyoverse.com/x/getGachaLog");
        // the key must never be sent anywhere but the publisher's servers
        assert!(api_base("https://public-operation-hkrpg.evil.example/getGachaLog?authkey=KEY").is_err());
        assert!(api_base("https://public-operation-hkrpg-sg.hoyoverse.com.evil.example/getGachaLog?authkey=KEY").is_err());
        assert!(api_base("http://public-operation-hkrpg-sg.hoyoverse.com/getGachaLog?authkey=KEY").is_err());
    }
}
