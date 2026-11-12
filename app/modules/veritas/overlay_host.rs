//! Damage widgets: one popup per widget, each with WebView2 in composition mode.
//!
//! Every widget page is the same Vite bundle (`#/overlay/veritas/<id>`),
//! rendered by a WebView2 composition controller into a DirectComposition
//! visual, so the popup has real per-pixel transparency, never takes focus and
//! lets clicks through to the game. All widgets share the main window's WebView2
//! environment (one browser process). Tauri IPC isn't available there: a small
//! bridge script serves the overlay's invoke/listen over `postMessage`.
//!
//! In arrange mode the whole popup is a drag handle (`HTCAPTION`); where it is
//! dropped is saved as a fraction of the game's client area.
//!
//! All COM objects live on the main thread; other threads only queue work.
//! If the environment can't be obtained, `status` reports `Failed` and the
//! caller falls back to a plain Tauri window.
use super::host::{self, Host};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
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

/// Where and how a widget should be, in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Placement {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    /// CSS pixel -> physical pixel ratio, in percent (DPI × widget scale).
    pub zoom_percent: u32,
    pub click_through: bool,
    pub draggable: bool,
}
static LAST: Mutex<Option<HashMap<String, Placement>>> = Mutex::new(None);

struct Surface {
    id: String,
    hwnd: HWND,
    controller: ICoreWebView2Controller,
    composition: ICoreWebView2CompositionController,
    webview: ICoreWebView2,
    draggable: bool,
    _device: IDCompositionDesktopDevice,
    _target: IDCompositionTarget,
    _visual: IDCompositionVisual2,
}
thread_local! {
    static APP: RefCell<Option<AppHandle>> = const { RefCell::new(None) };
    static ENV: RefCell<Option<ICoreWebView2Environment>> = const { RefCell::new(None) };
    static SURFACES: RefCell<Vec<Surface>> = const { RefCell::new(Vec::new()) };
    static CREATING_IDS: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    /// Popup being dragged right now: don't move it from under the cursor.
    static MOVING: Cell<isize> = const { Cell::new(0) };
    static TRACKING_LEAVE: Cell<isize> = const { Cell::new(0) };
}

const CLASS: PCWSTR = w!("NousVeritasOverlay");
const ROUTE: &str = "/index.html#/overlay/veritas/";
/// Origin the release build serves its bundle from (as Tauri does on Windows).
const ASSET_ORIGIN: &str = "http://tauri.localhost";

/// Minimal `__TAURI_INTERNALS__` so the overlay page (invoke/listen) works over
/// WebView2 messages; `nousOverlay.size` reports the measured content.
const BRIDGE: &str = r#"(() => {
  if (window.__TAURI_INTERNALS__) return
  const webview = window.chrome.webview
  const callbacks = new Map(), pending = new Map(), listeners = new Map()
  let next = 1
  window.isTauri = true
  window.nousOverlay = { size: (width, height) => webview.postMessage({ kind: 'size', width, height }) }
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

/// Current state; the first call starts obtaining the WebView2 environment.
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

/// Moves/shows/hides one widget, creating it on first show. Cheap when
/// nothing changed.
pub fn apply(app: &AppHandle, id: &str, placement: Placement) {
    {
        let mut last = LAST.lock().unwrap();
        let last = last.get_or_insert_with(HashMap::new);
        if last.get(id) == Some(&placement) {
            return;
        }
        last.insert(id.to_string(), placement);
    }
    if STATUS.load(Ordering::Acquire) == READY {
        let id = id.to_string();
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || apply_now(&handle, &id, placement));
    }
}

/// Hides every widget.
pub fn hide(app: &AppHandle) {
    let shown: Vec<(String, Placement)> = LAST
        .lock()
        .unwrap()
        .iter()
        .flatten()
        .filter(|(_, p)| p.visible)
        .map(|(id, p)| (id.clone(), *p))
        .collect();
    for (id, p) in shown {
        apply(app, &id, Placement { visible: false, ..p });
    }
}

/// Delivers a Tauri-style event to every widget page.
pub fn emit(app: &AppHandle, event: &str, payload: &impl serde::Serialize) {
    if STATUS.load(Ordering::Acquire) != READY {
        return;
    }
    let Ok(message) =
        serde_json::to_string(&json!({ "kind": "event", "event": event, "payload": payload }))
    else {
        return;
    };
    let _ = app.run_on_main_thread(move || {
        SURFACES.with(|s| {
            for surface in s.borrow().iter() {
                unsafe {
                    let _ = surface.webview.PostWebMessageAsJson(&HSTRING::from(&message));
                }
            }
        })
    });
}

/// Window facts for the replay harness (main thread only).
pub fn inspect() -> Option<Value> {
    let rows = SURFACES
        .try_with(|s| {
            s.borrow()
                .iter()
                .map(|surface| unsafe {
                    let style = GetWindowLongPtrW(surface.hwnd, GWL_EXSTYLE) as u32;
                    let mut rect = RECT::default();
                    let _ = GetWindowRect(surface.hwnd, &mut rect);
                    json!({
                        "widget": surface.id,
                        "visible": IsWindowVisible(surface.hwnd).as_bool(),
                        "clickThrough": style & WS_EX_TRANSPARENT.0 != 0,
                        "noActivate": style & WS_EX_NOACTIVATE.0 != 0,
                        "topmost": style & WS_EX_TOPMOST.0 != 0,
                        "x": rect.left, "y": rect.top,
                        "width": rect.right - rect.left, "height": rect.bottom - rect.top,
                    })
                })
                .collect::<Vec<_>>()
        })
        .ok()?;
    (!rows.is_empty()).then(|| json!({ "surface": "composition", "widgets": rows }))
}

fn start(app: &AppHandle) {
    let Some(main) = app.get_webview_window("main") else {
        log::warn!("Veritas overlay: no main webview to share, falling back");
        STATUS.store(FAILED, Ordering::Release);
        return;
    };
    let handle = app.clone();
    let queued = main.with_webview(move |webview| {
        let env = unsafe {
            webview
                .controller()
                .CoreWebView2()
                .and_then(|w| w.cast::<ICoreWebView2_2>())
                .and_then(|w| w.Environment())
        };
        match env {
            Ok(env) => {
                APP.with(|a| *a.borrow_mut() = Some(handle.clone()));
                ENV.with(|e| *e.borrow_mut() = Some(env));
                STATUS.store(READY, Ordering::Release);
                log::info!("Veritas overlay: composition widgets ready");
                let pending: Vec<(String, Placement)> = LAST
                    .lock()
                    .unwrap()
                    .iter()
                    .flatten()
                    .map(|(id, p)| (id.clone(), *p))
                    .collect();
                for (id, p) in pending {
                    apply_now(&handle, &id, p);
                }
            }
            Err(e) => {
                log::warn!("Veritas overlay: no WebView2 environment ({e}), falling back");
                STATUS.store(FAILED, Ordering::Release);
            }
        }
    });
    if let Err(e) = queued {
        log::warn!("Veritas overlay: {e}");
        STATUS.store(FAILED, Ordering::Release);
    }
}

/// Main thread. Popup + DirectComposition now, WebView2 asynchronously.
unsafe fn create(app: &AppHandle, id: &str) -> Result<()> {
    let Some(env) = ENV.with(|e| e.borrow().clone()) else {
        return Err(Error::from(E_UNEXPECTED));
    };
    unsafe {
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
        // everything the page leaves transparent shows the game through;
        // layered + transparent: clicks pass to the window below
        let hwnd = CreateWindowExW(
            WS_EX_NOREDIRECTIONBITMAP
                | WS_EX_TOPMOST
                | WS_EX_TOOLWINDOW
                | WS_EX_NOACTIVATE
                | WS_EX_LAYERED
                | WS_EX_TRANSPARENT,
            CLASS,
            &HSTRING::from(format!("Nous — {id}")),
            WS_POPUP,
            0,
            0,
            1,
            1,
            None,
            None,
            Some(instance.into()),
            None,
        )?;
        SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA)?;

        let device: IDCompositionDesktopDevice = DCompositionCreateDevice2(None)?;
        let target = device.CreateTargetForHwnd(hwnd, true)?;
        let visual = device.CreateVisual()?;
        target.SetRoot(&visual)?;
        device.Commit()?;

        let app = app.clone();
        let id = id.to_string();
        let handler = CreateCoreWebView2CompositionControllerCompletedHandler::create(Box::new(
            move |result, composition| {
                CREATING_IDS.with(|c| c.borrow_mut().remove(&id));
                let finished = (|| -> Result<()> {
                    result?;
                    let composition = composition.ok_or_else(|| Error::from(E_POINTER))?;
                    composition.SetRootVisualTarget(&visual)?;
                    device.Commit()?;
                    let controller: ICoreWebView2Controller = composition.cast()?;
                    let webview = setup_webview(&app, &id, &env, &composition, &controller)?;
                    SURFACES.with(|s| {
                        s.borrow_mut().push(Surface {
                            id: id.clone(),
                            hwnd,
                            controller,
                            composition,
                            webview,
                            draggable: false,
                            _device: device.clone(),
                            _target: target.clone(),
                            _visual: visual.clone(),
                        })
                    });
                    let placement = LAST
                        .lock()
                        .unwrap()
                        .as_ref()
                        .and_then(|l| l.get(&id).copied());
                    if let Some(placement) = placement {
                        apply_now(&app, &id, placement);
                    }
                    Ok(())
                })();
                if let Err(e) = finished {
                    log::warn!("Veritas overlay widget {id}: {e}");
                    let _ = DestroyWindow(hwnd);
                }
                Ok(())
            },
        ));
        env3.CreateCoreWebView2CompositionController(hwnd, &handler)
    }
}

unsafe fn setup_webview(
    app: &AppHandle,
    id: &str,
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
                    if !same_origin(&take_pwstr(uri), &allowed) {
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
        let widget = id.to_string();
        webview.add_WebMessageReceived(
            &WebMessageReceivedEventHandler::create(Box::new(move |webview, args| {
                let (Some(webview), Some(args)) = (webview, args) else {
                    return Ok(());
                };
                let mut text = PWSTR::null();
                args.WebMessageAsJson(&mut text)?;
                if let Ok(message) = serde_json::from_str::<Value>(&take_pwstr(text)) {
                    answer(&messages, &widget, &webview, message);
                }
                Ok(())
            })),
            &mut token,
        )?;

        webview.AddScriptToExecuteOnDocumentCreated(
            &HSTRING::from(BRIDGE),
            &AddScriptToExecuteOnDocumentCreatedCompletedHandler::create(Box::new(|_, _| Ok(()))),
        )?;
        webview.Navigate(&HSTRING::from(format!("{origin}{ROUTE}{id}")))?;
        Ok(webview)
    }
}

/// True if `uri` is a page of `origin`. A plain prefix check also let through
/// `http://tauri.localhost.example.com` and `http://tauri.localhost@example.com`,
/// pages that would get the overlay's bridge to the application.
fn same_origin(uri: &str, origin: &str) -> bool {
    uri.strip_prefix(origin)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(['/', '?', '#']))
}

#[cfg(test)]
mod tests {
    use super::same_origin;

    #[test]
    fn only_pages_of_the_application_may_load_in_the_overlay() {
        let origin = "http://tauri.localhost";
        for ok in [origin, "http://tauri.localhost/", "http://tauri.localhost/index.html#/overlay/veritas", "http://tauri.localhost?x=1"] {
            assert!(same_origin(ok, origin), "{ok}");
        }
        for foreign in [
            "http://tauri.localhost.example.com/",
            "http://tauri.localhost@example.com/",
            "http://tauri.localhost:8080/",
            "http://tauri.localhostx/",
            "https://tauri.localhost/",
            "http://example.com/http://tauri.localhost/",
        ] {
            assert!(!same_origin(foreign, origin), "{foreign}");
        }
        assert!(same_origin("http://localhost:1420/index.html", "http://localhost:1420"));
        assert!(!same_origin("http://localhost:14200/", "http://localhost:1420"));
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

/// Messages from a widget page: its size, or one of the overlay's commands.
fn answer(app: &AppHandle, widget: &str, webview: &ICoreWebView2, message: Value) {
    match message["kind"].as_str() {
        Some("size") => {
            if let (Some(w), Some(h)) = (message["width"].as_f64(), message["height"].as_f64()) {
                host::set_widget_size(app, widget, w, h);
            }
            return;
        }
        Some("invoke") => {}
        _ => return,
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
    unsafe {
        let _ = webview.PostWebMessageAsJson(&HSTRING::from(reply.to_string()));
    }
}

fn apply_now(app: &AppHandle, id: &str, p: Placement) {
    let surface = SURFACES.with(|s| {
        let mut s = s.borrow_mut();
        let surface = s.iter_mut().find(|s| s.id == id)?;
        surface.draggable = p.draggable;
        Some((surface.hwnd, surface.controller.clone()))
    });
    let Some((hwnd, controller)) = surface else {
        // created on first show; its placement is applied once it exists
        let start = p.visible && CREATING_IDS.with(|c| c.borrow_mut().insert(id.to_string()));
        if start {
            if let Err(e) = unsafe { create(app, id) } {
                CREATING_IDS.with(|c| c.borrow_mut().remove(id));
                log::warn!("Veritas overlay widget {id}: {e}");
            }
        }
        return;
    };
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
        if MOVING.get() != hwnd.0 as isize {
            let _ = SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                p.x,
                p.y,
                p.width as i32,
                p.height as i32,
                SWP_NOACTIVATE | SWP_SHOWWINDOW | SWP_FRAMECHANGED,
            );
        }
        let _ = controller.SetBounds(RECT {
            left: 0,
            top: 0,
            right: p.width as i32,
            bottom: p.height as i32,
        });
        if let Ok(c3) = controller.cast::<ICoreWebView2Controller3>() {
            let _ = c3.SetRasterizationScale(f64::from(p.zoom_percent.max(50)) / 100.0);
        }
        let _ = controller.SetIsVisible(true);
    }
}

/// (widget id, composition controller, draggable) for a popup.
fn surface_of(hwnd: HWND) -> Option<(String, ICoreWebView2CompositionController, bool)> {
    SURFACES
        .try_with(|s| {
            let s = s.try_borrow().ok()?;
            let surface = s.iter().find(|s| s.hwnd == hwnd)?;
            Some((surface.id.clone(), surface.composition.clone(), surface.draggable))
        })
        .ok()
        .flatten()
}

/// Mouse input is forwarded by hand in composition mode; it only arrives here
/// when click-through is off. In arrange mode the popup is dragged instead.
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        let Some((id, composition, draggable)) = surface_of(hwnd) else {
            return DefWindowProcW(hwnd, message, wparam, lparam);
        };
        match message {
            WM_MOUSEACTIVATE => return LRESULT(MA_NOACTIVATE as isize),
            WM_NCHITTEST if draggable => return LRESULT(HTCAPTION as isize),
            WM_SETCURSOR if draggable => {
                SetCursor(LoadCursorW(None, IDC_SIZEALL).ok());
                return LRESULT(1);
            }
            WM_ENTERSIZEMOVE => MOVING.set(hwnd.0 as isize),
            WM_EXITSIZEMOVE => {
                MOVING.set(0);
                let mut rect = RECT::default();
                if GetWindowRect(hwnd, &mut rect).is_ok() {
                    if let Some(app) = APP.with(|a| a.borrow().clone()) {
                        host::move_widget(&app, &id, rect.left, rect.top);
                    }
                }
            }
            WM_SETCURSOR if (lparam.0 & 0xffff) as u32 == HTCLIENT => {
                let mut cursor = HCURSOR::default();
                if composition.Cursor(&mut cursor).is_ok() && !cursor.is_invalid() {
                    SetCursor(Some(cursor));
                    return LRESULT(1);
                }
            }
            WM_MOUSEMOVE | WM_LBUTTONDOWN | WM_LBUTTONUP | WM_LBUTTONDBLCLK | WM_RBUTTONDOWN
            | WM_RBUTTONUP | WM_RBUTTONDBLCLK | WM_MBUTTONDOWN | WM_MBUTTONUP | WM_MBUTTONDBLCLK
            | WM_MOUSEWHEEL | WM_MOUSEHWHEEL | WM_MOUSELEAVE => {
                if message == WM_MOUSEMOVE && TRACKING_LEAVE.get() != hwnd.0 as isize {
                    let mut track = TRACKMOUSEEVENT {
                        cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    if TrackMouseEvent(&mut track).is_ok() {
                        TRACKING_LEAVE.set(hwnd.0 as isize);
                    }
                }
                if message == WM_MOUSELEAVE {
                    TRACKING_LEAVE.set(0);
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
            _ => {}
        }
        DefWindowProcW(hwnd, message, wparam, lparam)
    }
}
