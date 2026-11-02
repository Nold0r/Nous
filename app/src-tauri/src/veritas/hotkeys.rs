use std::{collections::BTreeMap, sync::mpsc, time::Duration};
use tauri::{AppHandle, Manager};
use windows_sys::Win32::UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*};

pub struct Hotkeys {
    tx: mpsc::Sender<(BTreeMap<String, String>, mpsc::Sender<Result<(), String>>)>,
}
pub fn parse(value: &str) -> Result<(u32, u32), String> {
    let mut mods = MOD_NOREPEAT;
    let mut key = None;
    for part in value.split('+').map(str::trim) {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => mods |= MOD_CONTROL,
            "shift" => mods |= MOD_SHIFT,
            "alt" => mods |= MOD_ALT,
            "up" => key = Some(VK_UP as u32),
            "down" => key = Some(VK_DOWN as u32),
            "plus" => key = Some(VK_OEM_PLUS as u32),
            "minus" => key = Some(VK_OEM_MINUS as u32),
            other if other.len() == 1 && other.as_bytes()[0].is_ascii_alphanumeric() => {
                if key.is_some() {
                    return Err("Нужна одна клавиша в сочетании".into());
                }
                key = Some(other.to_ascii_uppercase().as_bytes()[0] as u32);
            }
            _ => return Err(format!("Неизвестная клавиша: {part}")),
        }
    }
    if mods & (MOD_CONTROL | MOD_SHIFT | MOD_ALT) == 0 {
        return Err("Укажите Ctrl, Shift или Alt".into());
    }
    Ok((mods, key.ok_or("Не задана клавиша")?))
}
fn bindings(values: &BTreeMap<String, String>) -> Result<Vec<(String, u32, u32)>, String> {
    let mut out = Vec::new();
    for (action, value) in values {
        if value.trim().is_empty() {
            continue;
        }
        if !["menu", "hide", "zoomIn", "zoomOut", "zoomReset"].contains(&action.as_str()) {
            return Err("Неизвестное действие горячей клавиши".into());
        }
        let (mods, key) = parse(value)?;
        if out.iter().any(|(_, m, k)| *m == mods && *k == key) {
            return Err("Сочетания клавиш не должны повторяться".into());
        }
        out.push((action.clone(), mods, key));
    }
    Ok(out)
}
impl Hotkeys {
    pub fn start(app: AppHandle) -> Self {
        let (tx, rx) =
            mpsc::channel::<(BTreeMap<String, String>, mpsc::Sender<Result<(), String>>)>();
        std::thread::spawn(move || {
            let mut current: Vec<(String, u32, u32)> = Vec::new();
            loop {
                if app
                    .try_state::<super::host::Host>()
                    .is_some_and(|host| host.stop.load(std::sync::atomic::Ordering::Relaxed))
                {
                    break;
                }
                match rx.recv_timeout(Duration::from_millis(20)) {
                    Ok((values, reply)) => {
                        let result = bindings(&values).and_then(|next| {
                            for i in 0..current.len() {
                                unsafe {
                                    UnregisterHotKey(std::ptr::null_mut(), i as i32 + 6000);
                                }
                            }
                            let mut failed = None;
                            for (i, (_, mods, key)) in next.iter().enumerate() {
                                if unsafe {
                                    RegisterHotKey(
                                        std::ptr::null_mut(),
                                        i as i32 + 6000,
                                        *mods,
                                        *key,
                                    )
                                } == 0
                                {
                                    failed = Some(format!(
                                        "Сочетание {} занято другой программой",
                                        values.get(&next[i].0).unwrap()
                                    ));
                                    break;
                                }
                            }
                            if let Some(error) = failed {
                                for i in 0..next.len() {
                                    unsafe {
                                        UnregisterHotKey(std::ptr::null_mut(), i as i32 + 6000);
                                    }
                                }
                                for (i, (_, mods, key)) in current.iter().enumerate() {
                                    unsafe {
                                        RegisterHotKey(
                                            std::ptr::null_mut(),
                                            i as i32 + 6000,
                                            *mods,
                                            *key,
                                        );
                                    }
                                }
                                Err(error)
                            } else {
                                current = next;
                                Ok(())
                            }
                        });
                        let _ = reply.send(result);
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
                let mut message: MSG = unsafe { std::mem::zeroed() };
                while unsafe {
                    PeekMessageW(
                        &mut message,
                        std::ptr::null_mut(),
                        WM_HOTKEY,
                        WM_HOTKEY,
                        PM_REMOVE,
                    )
                } != 0
                {
                    if let Some((action, _, _)) = current.get(message.wParam.saturating_sub(6000)) {
                        let action = match action.as_str() {
                            "menu" => "settings",
                            "hide" => "toggle",
                            other => other,
                        };
                        let app = app.clone();
                        let action = action.to_string();
                        tauri::async_runtime::spawn_blocking(move || {
                            if let Err(error) = super::host::action(&app, &action) {
                                super::host::diagnostic(&app, error);
                            }
                        });
                    }
                }
            }
            for i in 0..current.len() {
                unsafe {
                    UnregisterHotKey(std::ptr::null_mut(), i as i32 + 6000);
                }
            }
        });
        Self { tx }
    }
    pub fn configure(&self, values: &BTreeMap<String, String>) -> Result<(), String> {
        bindings(values)?;
        let (tx, rx) = mpsc::channel();
        self.tx
            .send((values.clone(), tx))
            .map_err(|e| e.to_string())?;
        rx.recv_timeout(Duration::from_secs(2))
            .map_err(|e| e.to_string())?
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_real_os_combinations_and_rejects_ambiguous_keys() {
        assert_eq!(
            parse("Ctrl+Shift+M").unwrap(),
            (MOD_CONTROL | MOD_SHIFT | MOD_NOREPEAT, b'M' as u32)
        );
        assert!(parse("M").is_err());
        assert!(parse("Ctrl+A+B").is_err());
        assert!(parse("Ctrl+Unknown").is_err());
    }
    #[test]
    fn rejects_duplicate_shortcuts() {
        let map = BTreeMap::from([
            ("menu".into(), "Ctrl+Shift+M".into()),
            ("hide".into(), "Ctrl+Shift+M".into()),
        ]);
        assert!(bindings(&map).is_err());
    }
}
