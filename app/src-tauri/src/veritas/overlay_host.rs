//! Damage overlay surface: our own popup with WebView2 in composition mode.
//!
//! The page is the same Vite bundle (`#/veritas-overlay`), rendered by a
//! WebView2 composition controller into a DirectComposition visual, so the
//! window has real per-pixel transparency, never takes focus and lets clicks
//! through to the game. It shares the main window's WebView2 environment (one
//! browser process). Tauri IPC isn't available there: a small bridge script
//! answers the three overlay commands over `postMessage`.
//!
//! All COM objects live on the main thread; other threads only queue work.
//! If anything fails, `status` reports `Failed` and the caller falls back to a
//! plain Tauri window.
use super::host::{self, Host};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    sync::{
        Mutex,
        atomic::{AtomicU8, Ordering},
    },
};
use tauri::{AppHandle, Manager};
use webview2_com::{
    AddScriptToExecuteOnDocumentCreatedCompletedHandler,
    CreateCoreWebView2CompositionControllerCompletedHandler, CursorChangedEventHandler,
    Microsoft::Web::WebView2::Win32::*, NavigationStartingEventHandler,
    NewWindowRequestedEventHandler, WebMessageReceivedEventHandler,
    WebResourceRequestedEventHandler, take_pwstr,
};
use windows::{
    Win32::{
        Foundation::*,
        Graphics::{DirectComposition::*, Gdi::ScreenToClient},
        System::LibraryLoader::GetModuleHandleW,
        UI::{Input::KeyboardAndMouse::*, Shell::SHCreateMemStream, WindowsAndMessaging::*},
    },
    core::*,
};
use windows::core::{Error, Result};

/// Not exported by the enabled `windows` features.
const WM_MOUSELEAVE: u32 = 0x02A3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Pending,
    Ready,
    Failed,
}
const NONE: u8 = 0;
const CREATING: u8 = 1;
const READY: u8 = 2;
const FAILED: u8 = 3;
static STATUS: AtomicU8 = AtomicU8::new(NONE);

/// Where and how the overlay should be, in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Placement {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub dpi: u32,
    pub click_through: bool,
}
static LAST: Mutex<Option<Placement>> = Mutex::new(None);

struct Surface {
    hwnd: HWND,
    controller: ICoreWebView2Controller,
    composition: ICoreWebView2CompositionController,
    webview: ICoreWebView2,
    _device: IDCompositionDesktopDevice,
    _target: IDCompositionTarget,
    _visual: IDCompositionVisual2,
}
thread_local! {
    static SURFACE: RefCell<Option<Surface>> = const { RefCell::new(None) };
    static TRACKING_LEAVE: Cell<bool> = const { Cell::new(false) };
}

const CLASS: PCWSTR = w!("NousVeritasOverlay");
const ROUTE: &str = "/index.html#/veritas-overlay";
/// Origin the release build serves its bundle from (as Tauri does on Windows).
const ASSET_ORIGIN: &str = "http://tauri.localhost";

/// Minimal `__TAURI_INTERNALS__` so the unchanged overlay page (invoke/listen)
/// works over WebView2 messages.
const BRIDGE: &str = r#"(() => {
  if (window.__TAURI_INTERNALS__) return
  const webview = window.chrome.webview
  const callbacks = new Map(), pending = new Map(), listeners = new Map()
  let next = 1
  window.isTauri = true
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: (event, id) => listeners.get(event)?.delete(id) }
  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: 'veritas-overlay' }, currentWebview: { windowLabel: 'veritas-overlay', label: 'veritas-overlay' } },
    transformCallback: (callback) => { const id = next++; callbacks.set(id, callback); return id },
    unregisterCallback: (id) => callbacks.delete(id),
    convertFileSrc: (path) => path,
    invoke: (cmd, args = {}) => {
      if (cmd === 'plugin:event|listen') {
        if (!listeners.has(args.event)) listeners.set(args.event, new Set())
        listeners.get(args.event).add(args.handler)
        return Promise.resolve(args.handler)
      }
      if (cmd === 'plugin:event|unlisten') return Promise.resolve()
      const id = next++
      return new Promise((resolve, reject) => {
        pending.set(id, { resolve, reject })
        webview.postMessage({ kind: 'invoke', id, cmd, args })
      })
    },
  }
  webview.addEventListener('message', ({ data }) => {
    if (data.kind === 'reply') {
      const call = pending.get(data.id)
      pending.delete(data.id)
      if (call) data.error == null ? call.resolve(data.value) : call.reject(data.error)
    } else if (data.kind === 'event') {
      for (const id of listeners.get(data.event) ?? []) callbacks.get(id)?.({ event: data.event, id, payload: data.payload })
    }
  })
})()"#;

/// Current state; the first call starts creating the surface.
pub fn status(app: &AppHandle) -> Status {
    if STATUS
        .compare_exchange(NONE, CREATING, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
    {
        start(app);
    }
    match STATUS.load(Ordering::Acquire) {
        READY => Status::Ready,
        FAILED => Status::Failed,
        _ => Status::Pending,
    }
}

/// Moves/shows/hides the surface. Cheap when nothing changed.
pub fn apply(app: &AppHandle, placement: Placement) {
    {
        let mut last = LAST.lock().unwrap();
        if *last == Some(placement) {
            return;
        }
        *last = Some(placement);
    }
    if STATUS.load(Ordering::Acquire) == READY {
        let _ = app.run_on_main_thread(move || apply_now(placement));
    }
}

pub fn hide(app: &AppHandle) {
    let last = *LAST.lock().unwrap();
    if let Some(p) = last.filter(|p| p.visible) {
        apply(app, Placement { visible: false, ..p });
    }
}

/// Delivers a Tauri-style event to the overlay page.
pub fn emit(app: &AppHandle, event: &str, payload: &impl serde::Serialize) {
    if STATUS.load(Ordering::Acquire) != READY {
        return;
    }
    let Ok(message) = serde_json::to_string(&json!({ "kind": "event", "event": event, "payload": payload }))
    else {
        return;
    };
    let _ = app.run_on_main_thread(move || post(&message));
}

/// Window facts for the replay harness.
pub fn inspect() -> Option<Value> {
    let hwnd = SURFACE.try_with(|s| s.borrow().as_ref().map(|s| s.hwnd.0 as isize)).ok()??;
    let hwnd = HWND(hwnd as _);
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let mut rect = RECT::default();
        let _ = GetWindowRect(hwnd, &mut rect);
        Some(json!({
            "surface": "composition",
            "visible": IsWindowVisible(hwnd).as_bool(),
            "clickThrough": style & WS_EX_TRANSPARENT.0 != 0,
            "noActivate": style & WS_EX_NOACTIVATE.0 != 0,
            "topmost": style & WS_EX_TOPMOST.0 != 0,
            "toolWindow": style & WS_EX_TOOLWINDOW.0 != 0,
            "x": rect.left, "y": rect.top,
            "width": rect.right - rect.left, "height": rect.bottom - rect.top,
        }))
    }
}

fn fail(error: impl std::fmt::Display) {
    log::warn!("Veritas composition overlay unavailable, falling back: {error}");
    SURFACE.with(|s| {
        if let Some(surface) = s.borrow_mut().take() {
            unsafe {
                let _ = surface.controller.Close();
                let _ = DestroyWindow(surface.hwnd);
            }
        }
    });
    STATUS.store(FAILED, Ordering::Release);
}

fn start(app: &AppHandle) {
    let Some(main) = app.get_webview_window("main") else {
        log::warn!("Veritas overlay: no main webview to share, falling back");
        STATUS.store(FAILED, Ordering::Release);
        return;
    };
    log::info!("Veritas overlay: creating composition surface");
    let handle = app.clone();
    let queued = main.with_webview(move |webview| {
        log::info!("Veritas overlay: on main thread");
        if let Err(e) = unsafe { create(handle, webview.controller()) } {
            fail(e);
        }
    });
    if let Err(e) = queued {
        log::warn!("Veritas composition overlay: {e}");
        STATUS.store(FAILED, Ordering::Release);
    }
}

/// Main thread. Window + DirectComposition now, WebView2 asynchronously.
unsafe fn create(app: AppHandle, main: ICoreWebView2Controller) -> Result<()> {
    unsafe {
        let env = main
            .CoreWebView2()?
            .cast::<ICoreWebView2_2>()?
            .Environment()?;
        let env3 = env.cast::<ICoreWebView2Environment3>()?;

        let instance = GetModuleHandleW(None)?;
        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance.into(),
            lpszClassName: CLASS,
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            ..Default::default()
        };
        RegisterClassW(&class);
        // NOREDIRECTIONBITMAP: only the composition visual is drawn, so
        // everything the page leaves transparent shows the game through.
        let hwnd = CreateWindowExW(
            WS_EX_NOREDIRECTIONBITMAP
                | WS_EX_TOPMOST
                | WS_EX_TOOLWINDOW
                | WS_EX_NOACTIVATE
                | WS_EX_LAYERED
                | WS_EX_TRANSPARENT,
            CLASS,
            w!("Nous — урон в бою"),
            WS_POPUP,
            0,
            0,
            520,
            620,
            None,
            None,
            Some(instance.into()),
            None,
        )?;
        // layered + transparent = clicks pass to the window below (the game)
        SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA)?;

        let device: IDCompositionDesktopDevice = DCompositionCreateDevice2(None)?;
        let target = device.CreateTargetForHwnd(hwnd, true)?;
        let visual = device.CreateVisual()?;
        target.SetRoot(&visual)?;
        device.Commit()?;

        let handler = CreateCoreWebView2CompositionControllerCompletedHandler::create(Box::new(
            move |result, composition| {
                let finished = (|| -> Result<()> {
                    result?;
                    let composition = composition.ok_or_else(|| Error::from(E_POINTER))?;
                    composition.SetRootVisualTarget(&visual)?;
                    device.Commit()?;
                    let controller: ICoreWebView2Controller = composition.cast()?;
                    let webview = setup_webview(&app, &env, &composition, &controller)?;
                    SURFACE.with(|s| {
                        *s.borrow_mut() = Some(Surface {
                            hwnd,
                            controller,
                            composition,
                            webview,
                            _device: device.clone(),
                            _target: target.clone(),
                            _visual: visual.clone(),
                        })
                    });
                    STATUS.store(READY, Ordering::Release);
                    log::info!("Veritas overlay: composition surface ready");
                    if let Some(placement) = *LAST.lock().unwrap() {
                        apply_now(placement);
                    }
                    Ok(())
                })();
                if let Err(e) = finished {
                    let _ = DestroyWindow(hwnd);
                    fail(e);
                }
                Ok(())
            },
        ));
        env3.CreateCoreWebView2CompositionController(hwnd, &handler)
    }
}

unsafe fn setup_webview(
    app: &AppHandle,
    env: &ICoreWebView2Environment,
    composition: &ICoreWebView2CompositionController,
    controller: &ICoreWebView2Controller,
) -> Result<ICoreWebView2> {
    unsafe {
        controller
            .cast::<ICoreWebView2Controller2>()?
            .SetDefaultBackgroundColor(COREWEBVIEW2_COLOR { R: 0, G: 0, B: 0, A: 0 })?;
        if let Ok(c3) = controller.cast::<ICoreWebView2Controller3>() {
            c3.SetShouldDetectMonitorScaleChanges(false)?;
        }
        controller.SetIsVisible(false)?;
        let webview = controller.CoreWebView2()?;
        let settings = webview.Settings()?;
        settings.SetAreDefaultContextMenusEnabled(false)?;
        settings.SetIsStatusBarEnabled(false)?;
        settings.SetIsZoomControlEnabled(false)?;
        settings.SetAreDevToolsEnabled(cfg!(debug_assertions))?;

        let mut token = 0;
        composition.add_CursorChanged(
            &CursorChangedEventHandler::create(Box::new(|_, _| Ok(()))),
            &mut token,
        )?;

        let origin = origin(app);
        let allowed = origin.clone();
        webview.add_NavigationStarting(
            &NavigationStartingEventHandler::create(Box::new(move |_, args| {
                if let Some(args) = args {
                    let mut uri = PWSTR::null();
                    args.Uri(&mut uri)?;
                    if !take_pwstr(uri).starts_with(&allowed) {
                        args.SetCancel(true)?;
                    }
                }
                Ok(())
            })),
            &mut token,
        )?;
        webview.add_NewWindowRequested(
            &NewWindowRequestedEventHandler::create(Box::new(|_, args| {
                if let Some(args) = args {
                    args.SetHandled(true)?;
                }
                Ok(())
            })),
            &mut token,
        )?;

        if !tauri::is_dev() {
            webview.AddWebResourceRequestedFilter(
                &HSTRING::from(format!("{ASSET_ORIGIN}/*")),
                COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
            )?;
            let assets = app.clone();
            let env = env.clone();
            webview.add_WebResourceRequested(
                &WebResourceRequestedEventHandler::create(Box::new(move |_, args| {
                    let Some(args) = args else { return Ok(()) };
                    let mut uri = PWSTR::null();
                    args.Request()?.Uri(&mut uri)?;
                    let response = serve(&assets, &env, &take_pwstr(uri))?;
                    args.SetResponse(&response)
                })),
                &mut token,
            )?;
        }

        let messages = app.clone();
        webview.add_WebMessageReceived(
            &WebMessageReceivedEventHandler::create(Box::new(move |_, args| {
                let Some(args) = args else { return Ok(()) };
                let mut text = PWSTR::null();
                args.WebMessageAsJson(&mut text)?;
                if let Ok(message) = serde_json::from_str::<Value>(&take_pwstr(text)) {
                    answer(&messages, message);
                }
                Ok(())
            })),
            &mut token,
        )?;

        webview.AddScriptToExecuteOnDocumentCreated(
            &HSTRING::from(BRIDGE),
            &AddScriptToExecuteOnDocumentCreatedCompletedHandler::create(Box::new(|_, _| Ok(()))),
        )?;
        webview.Navigate(&HSTRING::from(format!("{origin}{ROUTE}")))?;
        Ok(webview)
    }
}

fn origin(app: &AppHandle) -> String {
    match app.config().build.dev_url.as_ref() {
        Some(url) if tauri::is_dev() => url.as_str().trim_end_matches('/').to_string(),
        _ => ASSET_ORIGIN.into(),
    }
}

/// Release build: the bundle comes from the app's embedded assets.
unsafe fn serve(
    app: &AppHandle,
    env: &ICoreWebView2Environment,
    uri: &str,
) -> Result<ICoreWebView2WebResourceResponse> {
    let path = uri
        .strip_prefix(ASSET_ORIGIN)
        .unwrap_or("/")
        .split(['?', '#'])
        .next()
        .unwrap_or("/");
    let path = if path == "/" { "/index.html" } else { path };
    unsafe {
        match app.asset_resolver().get(path.to_string()) {
            Some(asset) => {
                let stream = SHCreateMemStream(Some(&asset.bytes));
                let headers = HSTRING::from(format!("Content-Type: {}", asset.mime_type));
                env.CreateWebResourceResponse(stream.as_ref(), 200, w!("OK"), &headers)
            }
            None => env.CreateWebResourceResponse(None, 404, w!("Not Found"), w!("")),
        }
    }
}

/// Bridge calls from the page. Only the overlay's own commands exist here.
fn answer(app: &AppHandle, message: Value) {
    if message["kind"] != "invoke" {
        return;
    }
    let args = &message["args"];
    let result: std::result::Result<Value, String> = match message["cmd"].as_str() {
        Some("get_veritas_overlay") => {
            serde_json::to_value(app.state::<Host>().state()).map_err(|e| e.to_string())
        }
        Some("set_veritas_overlay_settings") => serde_json::from_value(args["settings"].clone())
            .map_err(|e| e.to_string())
            .and_then(|settings| host::set_veritas_overlay_settings(app.clone(), settings))
            .and_then(|state| serde_json::to_value(state).map_err(|e| e.to_string())),
        Some("veritas_overlay_action") => host::action(app, args["action"].as_str().unwrap_or(""))
            .and_then(|state| serde_json::to_value(state).map_err(|e| e.to_string())),
        other => Err(format!("Команда недоступна в оверлее: {other:?}")),
    };
    let reply = match result {
        Ok(value) => json!({ "kind": "reply", "id": message["id"], "value": value }),
        Err(error) => json!({ "kind": "reply", "id": message["id"], "error": error }),
    };
    post(&reply.to_string());
}

fn post(message: &str) {
    SURFACE.with(|s| {
        if let Some(surface) = s.borrow().as_ref() {
            unsafe {
                let _ = surface.webview.PostWebMessageAsJson(&HSTRING::from(message));
            }
        }
    });
}

fn apply_now(p: Placement) {
    SURFACE.with(|s| {
        let Some(surface) = s.borrow().as_ref().map(|s| (s.hwnd, s.controller.clone())) else {
            return;
        };
        let (hwnd, controller) = surface;
        unsafe {
            let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            let wanted = if p.click_through {
                style | WS_EX_TRANSPARENT.0 as isize
            } else {
                style & !(WS_EX_TRANSPARENT.0 as isize)
            };
            if wanted != style {
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, wanted);
            }
            if !p.visible || p.width == 0 || p.height == 0 {
                let _ = controller.SetIsVisible(false);
                let _ = ShowWindow(hwnd, SW_HIDE);
                return;
            }
            let _ = SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                p.x,
                p.y,
                p.width as i32,
                p.height as i32,
                SWP_NOACTIVATE | SWP_SHOWWINDOW | SWP_FRAMECHANGED,
            );
            let _ = controller.SetBounds(RECT {
                left: 0,
                top: 0,
                right: p.width as i32,
                bottom: p.height as i32,
            });
            if let Ok(c3) = controller.cast::<ICoreWebView2Controller3>() {
                let _ = c3.SetRasterizationScale(p.dpi.max(96) as f64 / 96.0);
            }
            let _ = controller.SetIsVisible(true);
        }
    });
}

fn composition() -> Option<ICoreWebView2CompositionController> {
    SURFACE
        .try_with(|s| s.try_borrow().ok()?.as_ref().map(|s| s.composition.clone()))
        .ok()
        .flatten()
}

/// Mouse input is forwarded by hand in composition mode; it only arrives here
/// when click-through is off.
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match message {
            WM_MOUSEACTIVATE => return LRESULT(MA_NOACTIVATE as isize),
            WM_SETCURSOR if (lparam.0 & 0xffff) as u32 == HTCLIENT => {
                if let Some(composition) = composition() {
                    let mut cursor = HCURSOR::default();
                    if composition.Cursor(&mut cursor).is_ok() && !cursor.is_invalid() {
                        SetCursor(Some(cursor));
                        return LRESULT(1);
                    }
                }
            }
            WM_MOUSEMOVE | WM_LBUTTONDOWN | WM_LBUTTONUP | WM_LBUTTONDBLCLK | WM_RBUTTONDOWN
            | WM_RBUTTONUP | WM_RBUTTONDBLCLK | WM_MBUTTONDOWN | WM_MBUTTONUP | WM_MBUTTONDBLCLK
            | WM_MOUSEWHEEL | WM_MOUSEHWHEEL | WM_MOUSELEAVE => {
                if let Some(composition) = composition() {
                    if message == WM_MOUSEMOVE && !TRACKING_LEAVE.get() {
                        let mut track = TRACKMOUSEEVENT {
                            cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                            dwFlags: TME_LEAVE,
                            hwndTrack: hwnd,
                            dwHoverTime: 0,
                        };
                        TRACKING_LEAVE.set(TrackMouseEvent(&mut track).is_ok());
                    }
                    if message == WM_MOUSELEAVE {
                        TRACKING_LEAVE.set(false);
                    }
                    let wheel = matches!(message, WM_MOUSEWHEEL | WM_MOUSEHWHEEL);
                    let mut point = POINT {
                        x: (lparam.0 & 0xffff) as i16 as i32,
                        y: ((lparam.0 >> 16) & 0xffff) as i16 as i32,
                    };
                    if wheel {
                        let _ = ScreenToClient(hwnd, &mut point);
                    }
                    let data = if wheel {
                        ((wparam.0 >> 16) & 0xffff) as i16 as i32 as u32
                    } else {
                        0
                    };
                    // the event kinds are the WM_* values themselves
                    let _ = composition.SendMouseInput(
                        COREWEBVIEW2_MOUSE_EVENT_KIND(message as i32),
                        COREWEBVIEW2_MOUSE_EVENT_VIRTUAL_KEYS((wparam.0 & 0xffff) as i32),
                        data,
                        if message == WM_MOUSELEAVE { POINT::default() } else { point },
                    );
                    return LRESULT(0);
                }
            }
            _ => {}
        }
        DefWindowProcW(hwnd, message, wparam, lparam)
    }
}
