//! Agent Moshpit: a small desktop office for coding agents.
//!
//! Each agent is a real program (Claude Code, Codex, and whatever else is in the
//! table) running in a terminal inside this window. This file is the desktop
//! side only: the window, the tray icon, notifications and the commands the
//! window may call. The work is in `engine`.

mod editor;
mod engine;
mod harness;
mod link;
mod mcp;
mod model;
mod office;
mod proctree;
mod pty;
mod status;
mod storage;
#[cfg(windows)]
mod toast;
mod update;
mod voice;

/// Run the lightweight MCP subprocess without opening a desktop or another office.
pub fn run_mcp_stdio() -> Result<(), String> { mcp::run_stdio() }

use engine::{Handle, Shell};
use model::{NewAgent, Phase, Snapshot};
use office::SavedDesk;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::image::Image;
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder, Window, WindowEvent};

const WINDOW: &str = "main";
const TRAY: &str = "main";
const TRAY_ICON: &[u8] = include_bytes!("../icons/tray.png");
const TRAY_ICON_ATTENTION: &[u8] = include_bytes!("../icons/tray-attention.png");

/// Whether there is a tray icon to come back from. Without one (some Linux
/// desktops), closing the window has to quit, or the app could never be reached again.
static HAS_TRAY: AtomicBool = AtomicBool::new(false);

/// The real `Shell`: the window, the tray and the notification centre.
struct Desktop {
    app: AppHandle,
    desks: storage::DeskStore,
    /// The folder each desk's last screen is kept in between runs.
    screens: PathBuf,
    /// (need you, working, done) as last shown on the tray, to skip no-op updates.
    shown: Mutex<Option<(usize, usize, usize)>>,
}

/// The office's own address (see `link.rs`), and whether Windows knows to open it here.
#[cfg_attr(not(windows), allow(dead_code))]
struct Address {
    scheme: String,
    known: bool,
}

/// A desk a click on a notification asked for, until the window takes it.
struct Opening(Mutex<Option<String>>);

#[derive(Default)]
struct NewAgentRequest(AtomicBool);

/// Bring the office to the front, with this desk's terminal open when one is named.
fn open_desk(app: &AppHandle, desk: Option<String>) {
    if let Some(id) = desk {
        *app.state::<Opening>().0.lock().unwrap() = Some(id);
    }
    open_window(app);
    // A window that is already there takes the desk now; a new one asks as it starts.
    let _ = app.emit("office:open-desk", ());
}

/// Say something in the system's notification centre. A click opens the desk it is about,
/// or the office when it is about none.
fn notify(app: &AppHandle, title: &str, body: &str, desk: Option<&str>) {
    #[cfg(windows)]
    {
        let address = app.state::<Address>();
        let opens = address.known.then(|| desk.map_or_else(|| link::office(&address.scheme), |id| link::desk(&address.scheme, id)));
        let app_id = toast::app_id(&app.config().identifier, instance_name().is_some());
        let (handle, wanted) = (app.clone(), desk.map(str::to_string));
        if toast::show(&app_id, &address.scheme, desk.unwrap_or("office"), title, body, opens.as_deref(), move || open_desk(&handle, wanted.clone())).is_ok() {
            return;
        }
    }
    use tauri_plugin_notification::NotificationExt;
    let _ = desk;
    let _ = app.notification().builder().title(title).body(body).show();
}

impl Shell for Desktop {
    fn snapshot(&self, snapshot: &Snapshot) {
        let _ = self.app.emit("office:snapshot", snapshot);
        let count = |phase: Phase| snapshot.agents.iter().filter(|a| a.phase == phase).count();
        let counts = (count(Phase::NeedsYou), count(Phase::Working), count(Phase::Done));
        let mut shown = self.shown.lock().unwrap();
        if *shown == Some(counts) {
            return;
        }
        let needed_before = shown.map_or(0, |(need, _, _)| need);
        *shown = Some(counts);
        if let Some(window) = self.app.get_webview_window(WINDOW) {
            let _ = window.set_title(&window_title(counts.0));
            // Someone new needs you while the window is behind: its taskbar button
            // flashes, as a terminal's does for a bell.
            if counts.0 > needed_before && !window.is_focused().unwrap_or(false) {
                let _ = window.request_user_attention(Some(tauri::UserAttentionType::Informational));
            }
        }
        let Some(tray) = self.app.tray_by_id(TRAY) else {
            return;
        };
        let (need, working, done) = counts;
        let mut parts: Vec<String> = Vec::new();
        if need > 0 {
            parts.push(format!("{need} need{} you", if need == 1 { "s" } else { "" }));
        }
        if working > 0 {
            parts.push(format!("{working} working"));
        }
        if done > 0 {
            parts.push(format!("{done} done"));
        }
        let name = titled(instance_name().as_deref(), 0);
        let tooltip = if parts.is_empty() { name } else { format!("{name}: {}", parts.join(", ")) };
        let _ = tray.set_tooltip(Some(tooltip));
        let bytes = if need > 0 { TRAY_ICON_ATTENTION } else { TRAY_ICON };
        if let Ok(icon) = Image::from_bytes(bytes) {
            let _ = tray.set_icon(Some(icon));
        }
    }

    fn notify(&self, title: &str, body: &str, desk: &str) {
        notify(&self.app, title, body, Some(desk));
    }

    fn save(&self, revision: u64, desks: &[SavedDesk]) {
        if let Err(error) = self.desks.save(revision, desks) {
            if self.desks.blocked() {
                return;
            } // The more useful startup recovery message is already retained.
            let problem = harness::Problem { text: format!("Changes could not be saved: {error}. Check that the office's data folder is writable before quitting."), file: self.desks.path.to_string_lossy().into_owned(), line: None };
            let problems = self.app.state::<StartupProblems>();
            let mut known = problems.0.lock().unwrap();
            if !known.iter().any(|p| p.text == problem.text && p.file == problem.file) {
                known.retain(|p| p.file != problem.file);
                known.push(problem.clone());
                let _ = self.app.emit("office:storage-problem", problem);
            }
        }
    }

    fn save_screens(&self, screens: &[(String, Vec<u8>)]) {
        if self.desks.preserve_screens() {
            return;
        }
        let _ = std::fs::create_dir_all(&self.screens);
        // A screen kept for a desk that has gone goes with it.
        if let Ok(entries) = std::fs::read_dir(&self.screens) {
            for entry in entries.flatten() {
                let id = entry.path().file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                if !screens.iter().any(|(kept, _)| *kept == id) {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
        for (id, printed) in screens {
            let _ = std::fs::write(self.screens.join(format!("{id}.screen")), printed);
        }
    }
}

/// The screens kept for these desks when the office last quit.
fn kept_screens(dir: &Path, desks: &[SavedDesk]) -> Vec<(String, Vec<u8>)> {
    desks.iter().filter_map(|d| std::fs::read(dir.join(format!("{}.screen", d.id))).ok().map(|printed| (d.id.clone(), printed))).collect()
}

/// The window's title, which the taskbar shows: how many need you, when anyone does.
/// A named office says its name, so a test window is never taken for the real one.
fn window_title(need: usize) -> String {
    titled(instance_name().as_deref(), need)
}

fn titled(instance: Option<&str>, need: usize) -> String {
    let name = instance.map_or_else(|| "Agent Moshpit".to_string(), |name| format!("Agent Moshpit ({name})"));
    match need {
        0 => name,
        1 => format!("{name} (1 needs you)"),
        n => format!("{name} ({n} need you)"),
    }
}

/// Where the office keeps its files: the desks, the window's place, pasted pictures.
struct DataDir(PathBuf);

/// What the user chose that the core has to know, kept beside the desks (`settings.json`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    /// Closing the window quits the office, as in most apps, instead of leaving it in the tray.
    close_quits: bool,
    voice: voice::Settings,
}

struct Chosen {
    path: PathBuf,
    now: Mutex<Settings>,
}

/// What was wrong as the office started, said once the window asks.
struct StartupProblems(Mutex<Vec<harness::Problem>>);

/// Write beside the file and rename, so a crash never leaves half a file.
fn write_whole(path: &Path, text: &str) {
    let tmp = path.with_extension("tmp");
    if std::fs::write(&tmp, text).is_ok() {
        let _ = std::fs::rename(&tmp, path);
    }
}

fn load_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

// ── where the window was ───────────────────────────────────────────────────

/// Size in logical pixels, position in physical ones (positions are only
/// comparable across monitors of different scale in physical pixels).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct Placement {
    width: f64,
    height: f64,
    x: i32,
    y: i32,
}

/// The window is destroyed on close, so its place is remembered here and on disk.
struct WindowMemory {
    path: PathBuf,
    last: Mutex<Option<Placement>>,
}

impl WindowMemory {
    fn note(&self, window: &Window) {
        if window.is_minimized().unwrap_or(false) || window.is_maximized().unwrap_or(false) {
            return;
        }
        let (Ok(size), Ok(position), Ok(scale)) = (window.inner_size(), window.outer_position(), window.scale_factor()) else {
            return;
        };
        if size.width == 0 || size.height == 0 {
            return;
        }
        let placement = Placement { width: f64::from(size.width) / scale, height: f64::from(size.height) / scale, x: position.x, y: position.y };
        *self.last.lock().unwrap() = Some(placement);
    }

    fn save(&self) {
        if let Some(placement) = *self.last.lock().unwrap() {
            if let Ok(text) = serde_json::to_string(&placement) {
                write_whole(&self.path, &text);
            }
        }
    }
}

/// The saved position, in logical pixels, if it still lands on a connected monitor.
fn on_screen(app: &AppHandle, placement: &Placement) -> Option<(f64, f64)> {
    let monitors = app.available_monitors().ok()?;
    monitors.iter().find_map(|monitor| {
        let (origin, size) = (monitor.position(), monitor.size());
        // The title bar must be reachable: require the top-left corner, with a margin, to be on this monitor.
        let inside_x = placement.x + 80 >= origin.x && placement.x + 80 <= origin.x + size.width as i32;
        let inside_y = placement.y >= origin.y && placement.y + 40 <= origin.y + size.height as i32;
        (inside_x && inside_y).then(|| {
            let scale = monitor.scale_factor();
            (f64::from(placement.x) / scale, f64::from(placement.y) / scale)
        })
    })
}

/// Show the office, making the window if it does not exist.
///
/// Closing the window destroys it and its webview, which is what gives the
/// memory back; the engine, the terminals and the tray keep running without it.
fn open_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(WINDOW) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    // Built on its own thread: on Windows, creating a webview from inside an
    // event handler (a tray click, a second launch) deadlocks the event loop.
    let app = app.clone();
    std::thread::spawn(move || {
        if app.get_webview_window(WINDOW).is_some() {
            return;
        }
        let remembered = *app.state::<WindowMemory>().last.lock().unwrap();
        let (width, height) = remembered.map_or((1280.0, 800.0), |p| (p.width.max(340.0), p.height.max(420.0)));
        let need = app.state::<Handle>().snapshot().agents.iter().filter(|a| a.phase == Phase::NeedsYou).count();
        let mut builder = WebviewWindowBuilder::new(&app, WINDOW, WebviewUrl::App("index.html".into()))
            .title(window_title(need))
            .inner_size(width, height)
            .min_inner_size(340.0, 420.0)
            // The window shows this app and nothing else. A link in a terminal is opened
            // in the browser, never followed here; this is the second lock.
            .on_navigation(|url| is_own_page(url.scheme(), url.host_str()));
        // WebView2 150 ignores environment browser flags in elevated hosts,
        // including hosted Windows CI. Pass the isolated debug fixture's port
        // through its API; release builds never enable this test hook.
        #[cfg(all(windows, debug_assertions))]
        if instance_name().is_some() && std::env::var_os("MOSHPIT_DATA_DIR").is_some() {
            if let Some(port) = std::env::var("MOSHPIT_TEST_DEBUG_PORT").ok().and_then(|s| s.parse::<u16>().ok()).filter(|p| *p != 0) {
                builder = builder.additional_browser_args(&format!("--remote-debugging-port={port}"));
            }
        }
        if let Some((x, y)) = remembered.as_ref().and_then(|p| on_screen(&app, p)) {
            builder = builder.position(x, y);
        }
        if let Ok(window) = builder.build() {
            app.state::<voice::Voice>().window(true);
            allow_clipboard(&window);
            quiet_browser(&window);
            app.state::<Handle>().attention(true, true);
        }
    });
}

/// Where the bundled page lives, as WebView2 names its origin.
#[cfg_attr(not(windows), allow(dead_code))]
const OWN_ORIGINS: &[&str] = &["http://tauri.localhost", "https://tauri.localhost"];

/// Whether an address WebView2 asks about is this app's own page.
#[cfg_attr(not(windows), allow(dead_code))]
fn own_address(uri: &str) -> bool {
    let Some((scheme, rest)) = uri.split_once("://") else {
        return false;
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = host.rsplit_once(':').filter(|(_, port)| port.chars().all(|c| c.is_ascii_digit())).map_or(host, |(name, _)| name);
    is_own_page(&scheme.to_ascii_lowercase(), Some(&host.to_ascii_lowercase()))
}

/// The terminal's right-click Paste reads the clipboard, which WebView2 asks the
/// user about unless told otherwise. This app's own page may read it without
/// asking, the way a terminal application does; no other page ever is shown here.
#[cfg(windows)]
fn allow_clipboard(window: &WebviewWindow) {
    let _ = window.with_webview(|webview| unsafe {
        use webview2_com::Microsoft::Web::WebView2::Win32::{ICoreWebView2Profile4, ICoreWebView2_13, COREWEBVIEW2_PERMISSION_KIND, COREWEBVIEW2_PERMISSION_KIND_CLIPBOARD_READ, COREWEBVIEW2_PERMISSION_STATE_ALLOW};
        use webview2_com::{PermissionRequestedEventHandler, SetPermissionStateCompletedHandler};
        use windows_core::{Interface, HSTRING, PWSTR};
        let Ok(core) = webview.controller().CoreWebView2() else {
            return;
        };
        // Said up front, so the page knows it may (and `navigator.permissions` says so)...
        if let Ok(profile) = core.cast::<ICoreWebView2_13>().and_then(|core| core.Profile()).and_then(|p| p.cast::<ICoreWebView2Profile4>()) {
            for origin in OWN_ORIGINS {
                let done = SetPermissionStateCompletedHandler::create(Box::new(|_| Ok(())));
                let _ = profile.SetPermissionState(COREWEBVIEW2_PERMISSION_KIND_CLIPBOARD_READ, &HSTRING::from(*origin), COREWEBVIEW2_PERMISSION_STATE_ALLOW, &done);
            }
        }
        // ...and answered when asked, in case the page is somewhere the list does not name.
        let handler = PermissionRequestedEventHandler::create(Box::new(|_, args| {
            let Some(args) = args else { return Ok(()) };
            let mut kind = COREWEBVIEW2_PERMISSION_KIND::default();
            args.PermissionKind(&mut kind)?;
            let mut uri = PWSTR::null();
            args.Uri(&mut uri)?;
            if kind == COREWEBVIEW2_PERMISSION_KIND_CLIPBOARD_READ && own_address(&webview2_com::take_pwstr(uri)) {
                args.SetState(COREWEBVIEW2_PERMISSION_STATE_ALLOW)?;
            }
            Ok(())
        }));
        let mut token = 0i64;
        let _ = core.add_PermissionRequested(&handler, &mut token);
    });
}

#[cfg(not(windows))]
fn allow_clipboard(_window: &WebviewWindow) {}

/// The window is an app, not a browser: F5 must not reload it, Ctrl+P print it or
/// Ctrl+F search it. WebView2 acts on those keys itself even when the page says
/// no, so they are turned off where it reads them, and the keys still reach the
/// page (and so a terminal's program). `MOSHPIT_DEVTOOLS=1` leaves them on, for
/// working on the window itself.
#[cfg(windows)]
fn quiet_browser(window: &WebviewWindow) {
    if std::env::var_os("MOSHPIT_DEVTOOLS").is_some() {
        return;
    }
    let _ = window.with_webview(|webview| unsafe {
        use webview2_com::AcceleratorKeyPressedEventHandler;
        use webview2_com::Microsoft::Web::WebView2::Win32::{ICoreWebView2AcceleratorKeyPressedEventArgs2, ICoreWebView2Settings3};
        use windows_core::Interface;
        let controller = webview.controller();
        if let Ok(settings) = controller.CoreWebView2().and_then(|core| core.Settings()) {
            if let Ok(settings) = settings.cast::<ICoreWebView2Settings3>() {
                let _ = settings.SetAreBrowserAcceleratorKeysEnabled(false);
            }
        }
        // The same, key by key, in case the setting only counts from the next page.
        let handler = AcceleratorKeyPressedEventHandler::create(Box::new(|_, args| {
            if let Some(args) = args.and_then(|a| a.cast::<ICoreWebView2AcceleratorKeyPressedEventArgs2>().ok()) {
                let _ = args.SetIsBrowserAcceleratorKeyEnabled(false);
            }
            Ok(())
        }));
        let mut token = 0i64;
        let _ = controller.add_AcceleratorKeyPressed(&handler, &mut token);
    });
}

#[cfg(not(windows))]
fn quiet_browser(_window: &WebviewWindow) {}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open the office", true, None::<&str>)?;
    let new = MenuItem::with_id(app, "new", "New agent\u{2026}", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Agent Moshpit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &new, &PredefinedMenuItem::separator(app)?, &quit])?;
    TrayIconBuilder::with_id(TRAY)
        .icon(Image::from_bytes(TRAY_ICON)?)
        .tooltip("Agent Moshpit")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => open_window(app),
            "new" => {
                app.state::<NewAgentRequest>().0.store(true, Ordering::SeqCst);
                open_window(app);
                let _ = app.emit("office:new-agent", ());
            }
            "quit" => request_quit(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                open_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

/// Quit, after asking when agents would be stopped by it.
fn request_quit(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Some(message) = app.state::<Handle>().quit_words() {
            use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
            let (tx, rx) = tokio::sync::oneshot::channel();
            app.dialog()
                .message(message)
                .title("Quit Agent Moshpit?")
                .kind(MessageDialogKind::Warning)
                .buttons(MessageDialogButtons::OkCancelCustom("Quit and stop them".into(), "Keep running".into()))
                .show(move |quit| {
                    let _ = tx.send(quit);
                });
            if !rx.await.unwrap_or(false) {
                return;
            }
        }
        quit_now(&app);
    });
}

/// Set once quitting has begun, so the window closing on the way out says nothing.
static QUITTING: AtomicBool = AtomicBool::new(false);

/// The first time the window is closed, say that the office is still running and
/// where it went. Closing a window is quitting in most apps; here it is not.
fn say_still_running(app: &AppHandle) {
    let told = app.state::<DataDir>().0.join("told-about-tray");
    if told.exists() {
        return;
    }
    let running = app.state::<Handle>().snapshot().agents.iter().filter(|a| a.running).count();
    let body = match running {
        0 => "It keeps watching from the tray. To quit, use Quit in the tray icon's menu, or ⋯ → Quit (Ctrl+Shift+Q) in the window.".to_string(),
        1 => "1 agent keeps working, and the office watches from the tray. To quit, use Quit in the tray icon's menu, or ⋯ → Quit (Ctrl+Shift+Q).".to_string(),
        n => format!("{n} agents keep working, and the office watches from the tray. To quit, use Quit in the tray icon's menu, or ⋯ → Quit (Ctrl+Shift+Q)."),
    };
    notify(app, "Agent Moshpit is still running", &body, None);
    let _ = std::fs::write(told, "");
}

/// What has to be done before the office goes away, for good or to come back as a
/// newer version: the window's place is kept, every program is ended, and the desks
/// and their screens are saved.
pub(crate) fn put_away(app: &AppHandle) {
    QUITTING.store(true, Ordering::Relaxed);
    app.state::<voice::Voice>().shutdown();
    app.state::<WindowMemory>().save();
    app.state::<Handle>().shutdown();
}

fn quit_now(app: &AppHandle) {
    put_away(app);
    app.exit(0);
}

// ── commands the window may call ───────────────────────────────────────────

#[tauri::command]
fn snapshot(handle: State<'_, Handle>) -> Snapshot {
    handle.snapshot()
}

/// Seat a new agent and start its program in a terminal of the size given.
#[tauri::command]
async fn new_agent(handle: State<'_, Handle>, spec: NewAgent, cols: u16, rows: u16) -> Result<String, String> {
    handle.new_agent(spec, cols, rows)
}

/// Install a program in a terminal of its own. Returns the terminal's id.
#[tauri::command]
async fn install(handle: State<'_, Handle>, harness: String, cols: u16, rows: u16) -> Result<String, String> {
    handle.run_job(&harness, engine::JobKind::Install, cols, rows)
}

/// Update a program in a terminal of its own. Returns the terminal's id.
#[tauri::command]
async fn update(handle: State<'_, Handle>, harness: String, cols: u16, rows: u16) -> Result<String, String> {
    handle.run_job(&harness, engine::JobKind::Update, cols, rows)
}

#[tauri::command]
fn forget_job(handle: State<'_, Handle>, job: String) {
    handle.forget_job(&job);
}

/// Make sure a desk's program is running before its terminal is shown.
#[tauri::command]
async fn wake(handle: State<'_, Handle>, agent: String, cols: u16, rows: u16) -> Result<(), String> {
    handle.wake(&agent, cols, rows)
}

/// Follow a desk's terminal: first the screen as it stands, then every byte after.
/// Returns the number to stop following with.
#[tauri::command]
fn term_attach(handle: State<'_, Handle>, agent: String, on_data: Channel<InvokeResponseBody>) -> Result<u64, String> {
    handle.attach(&agent, Box::new(move |bytes: &[u8]| on_data.send(InvokeResponseBody::Raw(bytes.to_vec())).is_ok()))
}

#[tauri::command]
fn term_detach(handle: State<'_, Handle>, voice: State<'_, voice::Voice>, agent: String, token: u64) {
    voice.cancel_agent(&agent);
    handle.detach(&agent, token);
}

#[tauri::command]
fn term_write(handle: State<'_, Handle>, agent: String, data: String) -> Result<(), String> {
    handle.write(&agent, data.as_bytes())
}

#[tauri::command]
fn term_resize(handle: State<'_, Handle>, agent: String, cols: u16, rows: u16) {
    handle.resize(&agent, cols, rows);
}

#[tauri::command]
fn stop(handle: State<'_, Handle>, voice: State<'_, voice::Voice>, agent: String) {
    voice.cancel_agent(&agent);
    handle.stop(&agent);
}

/// End a desk's program and start it again, in a terminal of the size given.
#[tauri::command]
async fn restart(handle: State<'_, Handle>, voice: State<'_, voice::Voice>, agent: String, cols: u16, rows: u16) -> Result<(), String> {
    voice.cancel_agent(&agent);
    handle.restart(&agent, cols, rows)
}

/// Show a desk's folder in the file manager.
#[tauri::command]
fn show_folder(app: AppHandle, handle: State<'_, Handle>, agent: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let folder = handle.folder(&agent).ok_or("That folder is not there any more.")?;
    app.opener().open_path(folder.to_string_lossy(), None::<&str>).map_err(|e| e.to_string())
}

/// The editors on this computer that a file path can be opened in.
#[tauri::command]
async fn editors() -> Vec<editor::EditorView> {
    editor::found()
}

/// Open a file a program printed, at its line, in the editor asked for (or the first
/// one on this computer); or show it in its folder when there is none. A path that is
/// not whole is read from the desk's folder. Only a file that exists is opened, and
/// only ever in the editor: nothing a program names is run. Says which it did.
#[tauri::command]
async fn open_file(app: AppHandle, handle: State<'_, Handle>, agent: Option<String>, path: String, line: Option<u32>, editor: Option<String>) -> Result<String, String> {
    use tauri_plugin_opener::OpenerExt;
    let asked = PathBuf::from(path.trim());
    let full = if asked.is_absolute() {
        asked
    } else {
        let desk = agent.as_deref().ok_or("That path is not a whole one.")?;
        handle.folder(desk).ok_or("That desk's folder is not there.")?.join(asked)
    };
    let full = full.canonicalize().map_err(|_| format!("{} is not there.", full.display()))?;
    if !full.is_file() {
        return Err(format!("{} is not a file.", full.display()));
    }
    if let Some(editor) = editor::pick(editor.as_deref()) {
        if editor.open(&full, line).is_ok() {
            return Ok("editor".into());
        }
    }
    app.opener().reveal_item_in_dir(editor::shown(&full)).map_err(|e| e.to_string())?;
    Ok("folder".into())
}

/// Open a desk's folder in the editor asked for, or the first one on this computer.
#[tauri::command]
async fn open_folder(handle: State<'_, Handle>, agent: String, editor: Option<String>) -> Result<(), String> {
    let folder = handle.folder(&agent).ok_or("That folder is not there any more.")?;
    let editor = editor::pick(editor.as_deref()).ok_or("No editor was found on this computer.")?;
    editor.open(&folder, None).map_err(|e| format!("{} could not be started: {e}", editor.name()))
}

/// Pasted pictures are kept this long, then cleared away when the office starts.
const PASTED_KEPT: std::time::Duration = std::time::Duration::from_secs(7 * 24 * 60 * 60);

/// A picture pasted into a terminal, written to a file so its path can be pasted
/// instead: Claude Code and Codex both take a picture by its path, and every other
/// program at least sees where it is.
#[tauri::command]
async fn save_pasted_image(dir: State<'_, DataDir>, request: tauri::ipc::Request<'_>) -> Result<String, String> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err("No picture came with the paste.".into());
    };
    let kind = picture_kind(bytes).ok_or("That is not a picture the office knows how to keep.")?;
    if bytes.len() > 40 * 1024 * 1024 {
        return Err("That picture is too big to paste.".into());
    }
    let folder = dir.0.join("pasted");
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let path = folder.join(format!("pasted-{}.{kind}", model::now_ms()));
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

/// The kind of picture, from its first bytes.
fn picture_kind(bytes: &[u8]) -> Option<&'static str> {
    match bytes {
        [0x89, b'P', b'N', b'G', ..] => Some("png"),
        [0xff, 0xd8, 0xff, ..] => Some("jpg"),
        [b'G', b'I', b'F', b'8', ..] => Some("gif"),
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => Some("webp"),
        [b'B', b'M', ..] => Some("bmp"),
        _ => None,
    }
}

/// Clear away pictures pasted long ago.
fn clear_old_pastes(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir.join("pasted")) else {
        return;
    };
    for entry in entries.flatten() {
        let old = entry.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_some_and(|age| age > PASTED_KEPT);
        if old {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// Whatever was wrong as the office started, for the window to say.
#[tauri::command]
fn startup_problems(problems: State<'_, StartupProblems>) -> Vec<harness::Problem> {
    problems.0.lock().unwrap().clone()
}

#[tauri::command]
fn settings(chosen: State<'_, Chosen>) -> Settings {
    chosen.now.lock().unwrap().clone()
}

/// Closing the window quits the office from now on, or leaves it in the tray again.
#[tauri::command]
fn set_close_quits(chosen: State<'_, Chosen>, on: bool) {
    let mut now = chosen.now.lock().unwrap();
    now.close_quits = on;
    if let Ok(text) = serde_json::to_string_pretty(&*now) {
        write_whole(&chosen.path, &text);
    }
}

/// The desk a click on a notification asked for, once: the window opens its terminal.
#[tauri::command]
fn take_new_agent(request: State<'_, NewAgentRequest>) -> bool {
    request.0.swap(false, Ordering::SeqCst)
}

#[tauri::command]
fn take_opening(opening: State<'_, Opening>) -> Option<String> {
    opening.0.lock().unwrap().take()
}

#[tauri::command]
fn dismiss(handle: State<'_, Handle>, voice: State<'_, voice::Voice>, agent: String) {
    voice.cancel_agent(&agent);
    handle.dismiss(&agent);
}

#[tauri::command]
fn rename(handle: State<'_, Handle>, agent: String, title: String) {
    handle.rename(&agent, &title);
}

/// Which desks have their terminal on screen right now.
#[tauri::command]
fn watch(handle: State<'_, Handle>, voice: State<'_, voice::Voice>, agents: Vec<String>) {
    voice.watch(&agents);
    handle.watch(agents);
}

#[tauri::command]
fn open_page(app: AppHandle, url: String) {
    open_web_page(&app, &url);
}

/// Whether an address is this app's own page: the bundled window, or the
/// development server while developing.
fn is_own_page(scheme: &str, host: Option<&str>) -> bool {
    match (scheme, host) {
        // The bundled page, as macOS and Linux name it, then as Windows does.
        ("tauri", Some("localhost")) | ("http" | "https", Some("tauri.localhost")) => true,
        ("http", Some("localhost")) => cfg!(debug_assertions),
        // What a webview shows before its first page.
        ("about", _) => true,
        _ => false,
    }
}

/// Whether a web address may be handed to the system's browser: an https page,
/// or a plain http one on this computer (a development server an agent started).
/// The window applies the same rule before it makes a link in a terminal
/// clickable (`webAddress` in `src/lib/links.ts`); this is the check that counts.
fn openable(url: &str) -> bool {
    if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return false;
    }
    let (rest, local_only) = match (url.strip_prefix("https://"), url.strip_prefix("http://")) {
        (Some(rest), _) => (rest, false),
        (_, Some(rest)) => (rest, true),
        _ => return false,
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    // "https://bank.example@evil.example" goes to evil.example: a name before an @ is refused.
    if host.is_empty() || host.contains('@') {
        return false;
    }
    if !local_only {
        return true;
    }
    let name = host.strip_prefix("[::1]").or_else(|| host.strip_prefix("localhost")).or_else(|| host.strip_prefix("127.0.0.1"));
    name.is_some_and(|port| port.is_empty() || (port.starts_with(':') && port[1..].chars().all(|c| c.is_ascii_digit())))
}

/// Hand a web address to the system's browser, if it is one that may be opened.
fn open_web_page(app: &AppHandle, url: &str) {
    use tauri_plugin_opener::OpenerExt;
    if openable(url) {
        let _ = app.opener().open_url(url, None::<&str>);
    }
}

#[tauri::command]
fn quit(app: AppHandle) {
    request_quit(&app);
}

/// The newer version of the office that is out, when one is.
#[tauri::command]
fn newer_version(latest: State<'_, update::Latest>) -> Option<update::Newer> {
    latest.newer.lock().unwrap().clone()
}

/// Put the newer version in place and start the office again, asking first when that
/// would end someone's work. An error is words for the user, and nothing was changed.
/// `Ok(false)` is the user saying not now.
#[tauri::command]
async fn update_now(app: AppHandle) -> Result<bool, String> {
    if let Some(busy) = app.state::<Handle>().quit_words() {
        use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
        let (tx, rx) = tokio::sync::oneshot::channel();
        app.dialog()
            .message(format!("The update restarts the office. {busy}"))
            .title("Update Agent Moshpit?")
            .kind(MessageDialogKind::Warning)
            .buttons(MessageDialogButtons::OkCancelCustom("Update and restart".into(), "Not now".into()))
            .show(move |go| {
                let _ = tx.send(go);
            });
        if !rx.await.unwrap_or(false) {
            return Ok(false);
        }
    }
    update::install(&app).await.map(|()| true)
}

#[tauri::command]
fn app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
fn voice_view(voice: State<'_, voice::Voice>) -> voice::View { voice.view() }

#[tauri::command]
fn voice_config(chosen: State<'_, Chosen>, voice: State<'_, voice::Voice>, settings: voice::Settings) -> Result<(), String> {
    voice.cancel();
    let mut now = chosen.now.lock().unwrap();
    let updated = Settings { voice: settings, ..now.clone() };
    let text = serde_json::to_string_pretty(&updated).map_err(|e| e.to_string())?;
    // Unlike transient UI state, a failure to save this privacy preference is surfaced.
    let tmp = chosen.path.with_extension("tmp");
    std::fs::write(&tmp, text).and_then(|()| std::fs::rename(&tmp, &chosen.path)).map_err(|e| format!("Voice settings could not be saved: {e}"))?;
    *now = updated;
    Ok(())
}

#[tauri::command]
fn voice_start(app: AppHandle, handle: State<'_, Handle>, voice: State<'_, voice::Voice>, chosen: State<'_, Chosen>, agent: String) -> Result<(), String> {
    if !app.get_webview_window(WINDOW).is_some_and(|w| w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(true)) {
        return Err("Open the office window before recording.".into());
    }
    voice.start(handle.inner().clone(), agent, chosen.now.lock().unwrap().voice.clone())
}

#[tauri::command]
fn voice_stop(voice: State<'_, voice::Voice>) -> Result<(), String> { voice.stop() }
#[tauri::command]
fn voice_cancel(voice: State<'_, voice::Voice>) { voice.cancel(); }
#[tauri::command]
fn voice_download(voice: State<'_, voice::Voice>, model: voice::Model) -> Result<(), String> { voice.download(model) }
#[tauri::command]
fn voice_cancel_download(voice: State<'_, voice::Voice>) { voice.cancel_download(); }
#[tauri::command]
fn voice_remove(voice: State<'_, voice::Voice>, model: voice::Model) -> Result<(), String> { voice.remove(model) }

#[tauri::command]
async fn pick_folder(window: WebviewWindow) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = tokio::sync::oneshot::channel();
    window.dialog().file().set_parent(&window).set_title("Where should the agent work?").pick_folder(move |folder| {
        let _ = tx.send(folder);
    });
    let folder = rx.await.map_err(|_| "The folder dialog closed unexpectedly.".to_string())?;
    Ok(folder.and_then(|f| f.into_path().ok()).map(|p| p.to_string_lossy().into_owned()))
}

/// `MOSHPIT_INSTANCE=name` (or `--instance name`, which is how Windows starts a named
/// office from its address) makes this a separate office: its own single-instance
/// lock, its own window storage, its own data folder, its own address and its own
/// window title. Tests use one so they can never hand over to, or share anything
/// with, the office the user has open.
fn instance_name() -> Option<String> {
    static NAME: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    NAME.get_or_init(|| {
        let args: Vec<String> = std::env::args().collect();
        let given = args.iter().position(|a| a == "--instance").and_then(|at| args.get(at + 1).cloned()).or_else(|| std::env::var("MOSHPIT_INSTANCE").ok())?;
        let name: String = given.chars().filter(char::is_ascii_alphanumeric).take(24).collect();
        (!name.is_empty()).then_some(name)
    })
    .clone()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut context = tauri::generate_context!();
    if let Some(name) = instance_name() {
        let identifier = format!("{}.{name}", context.config().identifier);
        context.config_mut().identifier = identifier;
    }
    let scheme = link::scheme(instance_name().as_deref());
    let handed = scheme.clone();
    let app = tauri::Builder::default()
        // Must be first: a second launch only brings the running office forward, with
        // the desk its address names (a notification clicked) open.
        .plugin(tauri_plugin_single_instance::init(move |app, args, _cwd| open_desk(app, link::asked(&args, &handed).flatten())))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            snapshot,
            new_agent,
            install,
            update,
            forget_job,
            wake,
            term_attach,
            term_detach,
            term_write,
            term_resize,
            stop,
            restart,
            show_folder,
            editors,
            open_file,
            open_folder,
            save_pasted_image,
            startup_problems,
            take_opening,
            take_new_agent,
            settings,
            set_close_quits,
            dismiss,
            rename,
            watch,
            open_page,
            quit,
            newer_version,
            update_now,
            app_version,
            pick_folder,
            voice_view, voice_config, voice_start, voice_stop, voice_cancel,
            voice_download, voice_cancel_download, voice_remove
        ])
        .on_window_event(|window, event| {
            if window.label() != WINDOW {
                return;
            }
            let handle = window.state::<Handle>();
            if matches!(event, WindowEvent::CloseRequested { .. } | WindowEvent::Destroyed) || window.is_minimized().unwrap_or(false) {
                window.state::<voice::Voice>().window(false);
                window.state::<voice::Voice>().shutdown();
            }
            if matches!(event, WindowEvent::Resized(_)) && !window.is_minimized().unwrap_or(true) { window.state::<voice::Voice>().window(true); }
            match event {
                // Chosen in the window: closing it quits, asking first if anyone is busy.
                WindowEvent::CloseRequested { api, .. } if HAS_TRAY.load(Ordering::Relaxed) && !QUITTING.load(Ordering::Relaxed) => {
                    if window.state::<Chosen>().now.lock().unwrap().close_quits {
                        api.prevent_close();
                        window.state::<voice::Voice>().window(true);
                        request_quit(window.app_handle());
                    }
                }
                WindowEvent::Focused(focused) => handle.attention(true, *focused),
                WindowEvent::Resized(_) | WindowEvent::Moved(_) => window.state::<WindowMemory>().note(window),
                WindowEvent::Destroyed => {
                    handle.attention(false, false);
                    window.state::<WindowMemory>().save();
                    if !HAS_TRAY.load(Ordering::Relaxed) {
                        quit_now(window.app_handle());
                    } else if !QUITTING.load(Ordering::Relaxed) {
                        say_still_running(window.app_handle());
                    }
                }
                _ => {}
            }
        })
        .setup(move |app| {
            // Windows is told that this program opens the office's address, which is what a
            // click on one of its notifications opens.
            let exe = std::env::current_exe().ok();
            let known = exe.as_deref().is_some_and(|exe| link::register(&scheme, exe, instance_name().as_deref()));
            // Started by a click on a notification from before: that desk is opened.
            let launched: Vec<String> = std::env::args().collect();
            app.manage(NewAgentRequest::default());
            app.manage(Opening(Mutex::new(link::asked(&launched, &scheme).flatten())));
            app.manage(Address { scheme: scheme.clone(), known });

            // Tests and tools can keep their files somewhere other than the real app data folder.
            let dir = match std::env::var_os("MOSHPIT_DATA_DIR") {
                Some(custom) => PathBuf::from(custom),
                None => app.path().app_data_dir()?,
            };
            let _ = std::fs::create_dir_all(&dir);
            let (desks, saved, storage_problem) = storage::DeskStore::load(dir.join("desks.json"));
            let chosen = dir.join("settings.json");
            app.manage(Chosen { now: Mutex::new(load_json(&chosen).unwrap_or_default()), path: chosen });
            let memory = WindowMemory { path: dir.join("window.json"), last: Mutex::new(None) };
            *memory.last.lock().unwrap() = load_json(&memory.path);
            app.manage(memory);

            let screens = dir.join("screens");
            let kept = kept_screens(&screens, &saved);
            let (table, mut problems) = harness::table_checked(&dir.join("harnesses.json"));
            if let Some(text) = storage_problem {
                problems.push(harness::Problem { text, file: desks.path.to_string_lossy().into_owned(), line: None });
            }
            let shell = Arc::new(Desktop { app: app.handle().clone(), desks, screens, shown: Mutex::new(None) });
            let handle = engine::start(shell, table, saved, kept);
            if std::env::var_os("MOSHPIT_DISABLE_MCP").is_none() {
                let path = dir.join("coordination.json");
                if let Err(text) = handle.enable_mcp(path.clone()) {
                    problems.push(harness::Problem { text, file: path.to_string_lossy().into_owned(), line: None });
                }
            }
            app.manage(handle);
            app.manage(StartupProblems(Mutex::new(problems)));
            clear_old_pastes(&dir);
            app.manage(DataDir(dir.clone()));
            app.manage(voice::Voice::new(dir.join("voice-models")));

            app.manage(update::Latest::default());
            update::watch(app.handle());

            HAS_TRAY.store(build_tray(app.handle()).is_ok(), Ordering::Relaxed);
            // `--hidden` is for starting with the computer: tray only, no window.
            let hidden = std::env::args().any(|arg| arg == "--hidden");
            if !hidden || !HAS_TRAY.load(Ordering::Relaxed) {
                open_window(app.handle());
            }
            Ok(())
        })
        .build(context)
        .expect("Agent Moshpit could not start");

    app.run(|app, event| match event {
        // Closing the last window is not quitting: the office keeps watching from the tray.
        RunEvent::ExitRequested { api, code, .. } => {
            if code.is_none() && HAS_TRAY.load(Ordering::Relaxed) {
                api.prevent_exit();
            }
        }
        RunEvent::Exit => app.state::<voice::Voice>().shutdown(),
        #[cfg(target_os = "macos")]
        RunEvent::Reopen { .. } => open_window(app),
        _ => {
            let _ = app;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{is_own_page, openable, own_address, titled};

    #[test]
    fn a_named_office_says_its_name_in_the_title() {
        assert_eq!(titled(None, 0), "Agent Moshpit");
        assert_eq!(titled(None, 2), "Agent Moshpit (2 need you)");
        assert_eq!(titled(Some("ux"), 0), "Agent Moshpit (ux)");
        assert_eq!(titled(Some("ux"), 1), "Agent Moshpit (ux) (1 needs you)");
    }

    #[test]
    fn only_the_apps_own_page_reads_the_clipboard_unasked() {
        for own in ["http://tauri.localhost/", "https://tauri.localhost/index.html", "http://tauri.localhost:80/x", "HTTP://Tauri.Localhost/"] {
            assert!(own_address(own), "{own}");
        }
        for other in ["https://example.com/", "http://tauri.localhost.evil.example/", "https://evil.example/?tauri.localhost", "file:///C:/x", "tauri.localhost", ""] {
            assert!(!own_address(other), "{other}");
        }
    }

    #[test]
    fn the_window_stays_on_its_own_page() {
        assert!(is_own_page("tauri", Some("localhost")));
        assert!(is_own_page("http", Some("tauri.localhost")));
        assert!(is_own_page("https", Some("tauri.localhost")));
        assert!(is_own_page("about", None));
        assert!(!is_own_page("https", Some("example.com")));
        assert!(!is_own_page("https", Some("localhost")));
        assert!(!is_own_page("http", Some("tauri.localhost.evil.example")));
        assert!(!is_own_page("file", None));
        assert!(!is_own_page("tauri", Some("example.com")));
    }

    #[test]
    fn only_web_pages_that_are_what_they_look_like_are_opened() {
        for good in ["https://example.com", "https://example.com/a?b=c#d", "http://localhost:5173/", "http://127.0.0.1:8080/health", "http://localhost", "http://[::1]:3000/x"] {
            assert!(openable(good), "{good}");
        }
        for bad in [
            "http://example.com",
            "http://localhost.evil.example/",
            "http://localhost:80a/",
            "http://127.0.0.1.evil.example",
            "https://bank.example@evil.example/",
            "https://",
            "https:// example.com",
            "https://example.com/\npath",
            "file:///C:/Windows/System32/calc.exe",
            "javascript:alert(1)",
            "ms-settings:",
            "example.com",
            "",
        ] {
            assert!(!openable(bad), "{bad:?}");
        }
    }
}
