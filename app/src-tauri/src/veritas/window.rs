//! Window metadata only: no OpenProcess, game memory or injection APIs.
use super::{
    geometry::{Rect, place},
    host::{Host, Presentation},
    overlay_host::{self, Placement, Status},
    settings::OverlaySettings,
};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, POINT, RECT},
    Graphics::Gdi::ClientToScreen,
    UI::{
        HiDpi::GetDpiForWindow,
        Shell::{QUNS_RUNNING_D3D_FULL_SCREEN, SHQueryUserNotificationState},
        WindowsAndMessaging::*,
    },
};
use windows_sys::core::BOOL;

/// The overlay webview is created on first need, so Nous with Veritas off
/// carries no extra window.
fn overlay(app: &AppHandle) -> Option<tauri::WebviewWindow> {
    if let Some(window) = app.get_webview_window("veritas-overlay") {
        return Some(window);
    }
    match build_overlay(app) {
        Ok(window) => Some(window),
        Err(e) => {
            log::warn!("Veritas overlay window: {e}");
            None
        }
    }
}
fn build_overlay(app: &AppHandle) -> tauri::Result<tauri::WebviewWindow> {
    let mut builder = WebviewWindowBuilder::new(
        app,
        "veritas-overlay",
        WebviewUrl::App("index.html#/veritas-overlay".into()),
    )
    .title("Nous — урон в бою")
    .transparent(true)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .focused(false)
    .focusable(false)
    .shadow(false)
    .resizable(false)
    .visible(false)
    .inner_size(520.0, 620.0);
    if let Some(args) = app
        .config()
        .app
        .windows
        .first()
        .and_then(|w| w.additional_browser_args.as_ref())
    {
        builder = builder.additional_browser_args(args);
    }
    let overlay = builder.build()?;
    overlay.set_ignore_cursor_events(true)?;
    Ok(overlay)
}
pub fn setup(app: &AppHandle, replay: bool) -> tauri::Result<()> {
    if replay {
        let mut target = WebviewWindowBuilder::new(
            app,
            "veritas-replay-target",
            WebviewUrl::App("index.html#/veritas-replay-target".into()),
        )
        .title("Nous — тестовая сцена, игра не запущена")
        .inner_size(1280.0, 800.0)
        .position(80.0, 80.0)
        .focused(false)
        .visible(true);
        if let Some(args) = app
            .config()
            .app
            .windows
            .first()
            .and_then(|w| w.additional_browser_args.as_ref())
        {
            target = target.additional_browser_args(args);
        }
        target.build()?;
    }
    Ok(())
}
struct Search {
    pid: u32,
    window: HWND,
}
unsafe extern "system" fn find_window(hwnd: HWND, param: LPARAM) -> BOOL {
    let search = unsafe { &mut *(param as *mut Search) };
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, &mut pid);
    }
    if pid == search.pid
        && unsafe { IsWindowVisible(hwnd) } != 0
        && unsafe { GetWindow(hwnd, GW_OWNER) }.is_null()
    {
        search.window = hwnd;
        0
    } else {
        1
    }
}
fn target(app: &AppHandle) -> Option<HWND> {
    let host = app.state::<Host>();
    if host.replay {
        if !host
            .target_available
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            return None;
        }
        return app
            .get_webview_window("veritas-replay-target")
            .and_then(|w| w.hwnd().ok())
            .map(|h| h.0);
    }
    let pid = app.state::<crate::launch::LaunchState>().game_pid()?;
    let mut search = Search {
        pid,
        window: std::ptr::null_mut(),
    };
    unsafe {
        EnumWindows(Some(find_window), &mut search as *mut Search as LPARAM);
    }
    (!search.window.is_null()).then_some(search.window)
}
/// The game's client area on screen, in physical pixels, and its DPI.
fn client_area(hwnd: HWND) -> Option<(Rect, u32)> {
    let mut native: RECT = unsafe { std::mem::zeroed() };
    let mut origin = POINT { x: 0, y: 0 };
    if unsafe { GetClientRect(hwnd, &mut native) } == 0
        || unsafe { ClientToScreen(hwnd, &mut origin) } == 0
    {
        return None;
    }
    let bounds = Rect {
        x: origin.x,
        y: origin.y,
        width: (native.right - native.left).max(1) as u32,
        height: (native.bottom - native.top).max(1) as u32,
    };
    Some((bounds, unsafe { GetDpiForWindow(hwnd) }))
}
pub fn target_rect(app: &AppHandle) -> Option<Rect> {
    target(app).and_then(client_area).map(|(rect, _)| rect)
}
pub fn refresh(app: &AppHandle) {
    let host = app.state::<Host>();
    let enabled = host.replay || app.state::<super::VeritasState>().enabled();
    if !enabled {
        if let Some(window) = app.get_webview_window("veritas-overlay") {
            if window.is_visible().unwrap_or(false) {
                let _ = window.hide();
            }
        }
        overlay_host::hide(app);
        host.update_presentation(app, Presentation::default());
        return;
    }
    // composition widgets first; one Tauri window only if they can't be created
    let surface = overlay_host::status(app);
    let fallback = if surface == Status::Failed {
        match overlay(app) {
            Some(window) => Some(window),
            None => return,
        }
    } else {
        None
    };
    let target = target(app);
    let settings = host.settings();
    let mut presentation = Presentation::default();
    let mut area = None;
    if let Some(hwnd) = target {
        presentation.target_found = unsafe { IsWindow(hwnd) } != 0;
        // shown while the game is on screen, focused or not
        presentation.target_visible = presentation.target_found
            && unsafe { IsIconic(hwnd) } == 0
            && unsafe { IsWindowVisible(hwnd) } != 0;
        area = client_area(hwnd);
        let mut notification = 0;
        presentation.exclusive_fullscreen = if host.replay {
            host.exclusive.load(std::sync::atomic::Ordering::Relaxed)
        } else {
            (unsafe { GetForegroundWindow() } == hwnd)
                && presentation.target_visible
                && unsafe { SHQueryUserNotificationState(&mut notification) } >= 0
                && notification == QUNS_RUNNING_D3D_FULL_SCREEN
        };
    }
    let snapshot = host.snapshot();
    // nothing to arrange over once the game is gone
    if !presentation.target_found {
        host.arranging.store(false, std::sync::atomic::Ordering::Relaxed);
    }
    let arranging = host.arranging.load(std::sync::atomic::Ordering::Relaxed);
    presentation.visible = (arranging && presentation.target_visible)
        || nous_veritas::visible(
            settings.enabled,
            snapshot.connected,
            snapshot.active,
            presentation.target_visible,
            host.hidden.load(std::sync::atomic::Ordering::Relaxed),
            host.replay && presentation.exclusive_fullscreen,
        );
    presentation.message = if presentation.exclusive_fullscreen {
        Some("Windows сообщает полноэкранный Direct3D-режим. Если окно Nous не видно, включите в игре оконный или безрамочный режим.".into())
    } else if !presentation.target_found {
        Some(
            if host.replay {
                "Тестовое окно закрыто."
            } else {
                "Окно игры не найдено."
            }
            .into(),
        )
    } else {
        None
    };
    if let Some(overlay) = fallback {
        if let Some((bounds, dpi)) = area {
            apply_geometry(&overlay, place(bounds, dpi, 520.0, 620.0, "top-right", 24.0, 24.0), &settings);
        }
        if presentation.visible {
            if !overlay.is_visible().unwrap_or(false) {
                let _ = overlay.show();
            }
        } else if overlay.is_visible().unwrap_or(false) {
            let _ = overlay.hide();
        }
    } else {
        let sizes = host.sizes.lock().unwrap().clone();
        for (id, _) in super::settings::WIDGETS {
            let on = settings.views.get(id).copied().unwrap_or(false);
            let placement = match area {
                Some((bounds, dpi)) if on => {
                    let size = sizes.get(id).copied().unwrap_or((340.0, 160.0));
                    let rect = super::geometry::widget(
                        bounds,
                        dpi,
                        settings.scale,
                        settings.widget_position(id),
                        size,
                    );
                    Placement {
                        visible: presentation.visible,
                        x: rect.x,
                        y: rect.y,
                        width: rect.width,
                        height: rect.height,
                        zoom_percent: (f64::from(dpi.max(96)) / 96.0 * settings.scale * 100.0)
                            .round() as u32,
                        click_through: settings.click_through && !arranging,
                        draggable: arranging,
                    }
                }
                _ => Placement::default(),
            };
            overlay_host::apply(app, id, placement);
        }
    }
    host.update_presentation(app, presentation);
}
fn apply_geometry(window: &tauri::WebviewWindow, rect: Rect, settings: &OverlaySettings) {
    if window.outer_position().ok() != Some(PhysicalPosition::new(rect.x, rect.y)) {
        let _ = window.set_position(PhysicalPosition::new(rect.x, rect.y));
    }
    if window.inner_size().ok() != Some(PhysicalSize::new(rect.width, rect.height)) {
        let _ = window.set_size(PhysicalSize::new(rect.width, rect.height));
    }
    let _ = window.set_ignore_cursor_events(settings.click_through);
}
