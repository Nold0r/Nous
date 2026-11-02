//! Window metadata only: no OpenProcess, game memory or injection APIs.
use super::{
    geometry::{Rect, place},
    host::{Host, Presentation},
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
pub fn refresh(app: &AppHandle) {
    let host = app.state::<Host>();
    let enabled = host.replay || app.state::<super::VeritasState>().enabled();
    if !enabled {
        if let Some(window) = app.get_webview_window("veritas-overlay") {
            if window.is_visible().unwrap_or(false) {
                let _ = window.hide();
            }
        }
        host.update_presentation(app, Presentation::default());
        return;
    }
    let Some(overlay) = overlay(app) else {
        return;
    };
    let target = target(app);
    let settings = host.settings();
    let mut presentation = Presentation::default();
    let mut rectangle = None;
    if let Some(hwnd) = target {
        presentation.target_found = unsafe { IsWindow(hwnd) } != 0;
        let foreground = host.replay || unsafe { GetForegroundWindow() } == hwnd;
        presentation.target_visible = presentation.target_found
            && foreground
            && unsafe { IsIconic(hwnd) } == 0
            && unsafe { IsWindowVisible(hwnd) } != 0;
        let mut native: RECT = unsafe { std::mem::zeroed() };
        let mut origin = POINT { x: 0, y: 0 };
        if unsafe { GetClientRect(hwnd, &mut native) } != 0
            && unsafe { ClientToScreen(hwnd, &mut origin) } != 0
        {
            let bounds = Rect {
                x: origin.x,
                y: origin.y,
                width: (native.right - native.left).max(1) as u32,
                height: (native.bottom - native.top).max(1) as u32,
            };
            rectangle = Some(place(
                bounds,
                unsafe { GetDpiForWindow(hwnd) },
                settings.width,
                settings.height,
                &settings.anchor,
                settings.offset_x,
                settings.offset_y,
            ));
        }
        let mut notification = 0;
        presentation.exclusive_fullscreen = if host.replay {
            host.exclusive.load(std::sync::atomic::Ordering::Relaxed)
        } else {
            presentation.target_visible
                && unsafe { SHQueryUserNotificationState(&mut notification) } >= 0
                && notification == QUNS_RUNNING_D3D_FULL_SCREEN
        };
    }
    let snapshot = host.snapshot();
    presentation.visible = nous_veritas::visible(
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
    if let Some(rectangle) = rectangle {
        apply_geometry(&overlay, rectangle, &settings);
    }
    if presentation.visible {
        if !overlay.is_visible().unwrap_or(false) {
            let _ = overlay.show();
        }
    } else if overlay.is_visible().unwrap_or(false) {
        let _ = overlay.hide();
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
