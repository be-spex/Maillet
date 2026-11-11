//! Windows: the Maillet window (popup at the tray icon), settings, about and
//! the OTP code popup.

use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};

pub const MAIN: &str = "main";
const MAIN_SIZE: (f64, f64) = (420.0, 720.0);
/// Gap between window and tray icon / screen edge.
const MARGIN: i32 = 8;

/// Where the tray icon was last clicked and when the window was last hidden by losing focus.
#[derive(Default)]
pub struct MainWindowState {
    last_click: Mutex<Option<(i32, i32)>>,
    hidden_on_blur: Mutex<Option<Instant>>,
    shown_at: Mutex<Option<Instant>>,
}

pub fn create(app: &tauri::App) -> tauri::Result<()> {
    let main = WebviewWindowBuilder::new(app, MAIN, WebviewUrl::App("app.html".into()))
        .title("Maillet")
        .inner_size(MAIN_SIZE.0, MAIN_SIZE.1)
        .min_inner_size(320.0, 400.0)
        .decorations(false)
        .skip_taskbar(true)
        .visible(false)
        .build()?;
    let handle = app.handle().clone();
    main.on_window_event(move |event| match event {
        // Popup behaviour: clicking elsewhere hides the window.
        WindowEvent::Focused(false) => {
            let state = handle.state::<MainWindowState>();
            let just_shown = state.shown_at.lock().unwrap().is_some_and(|t| t.elapsed() < Duration::from_millis(400));
            if !just_shown {
                *state.hidden_on_blur.lock().unwrap() = Some(Instant::now());
                if let Some(w) = handle.get_webview_window(MAIN) {
                    let _ = w.hide();
                }
            }
        }
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            if let Some(w) = handle.get_webview_window(MAIN) {
                let _ = w.hide();
            }
        }
        _ => {}
    });

    let settings = WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("index.html".into()))
        .title("Maillet – Einstellungen")
        .inner_size(820.0, 760.0)
        .visible(false)
        .build()?;
    hide_on_close(&settings);

    let about = WebviewWindowBuilder::new(app, "about", WebviewUrl::App("about.html".into()))
        .title("Über Maillet")
        .inner_size(360.0, 340.0)
        .resizable(false)
        .visible(false)
        .build()?;
    hide_on_close(&about);

    WebviewWindowBuilder::new(app, "code", WebviewUrl::App("code.html".into()))
        .title("Maillet – Code")
        .inner_size(crate::features::otp::popup::POPUP_WIDTH, 250.0)
        .resizable(false)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .build()?;
    Ok(())
}

fn hide_on_close(window: &WebviewWindow) {
    let w = window.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = w.hide();
        }
    });
}

pub fn show(app: &AppHandle, label: &str) {
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// Tray click: show the window at the icon, or hide it if it is open.
pub fn toggle_main(app: &AppHandle, at: Option<(i32, i32)>) {
    let Some(w) = app.get_webview_window(MAIN) else { return };
    let state = app.state::<MainWindowState>();
    // Clicking the icon first takes the focus away, which already hid the window.
    let just_hidden = state.hidden_on_blur.lock().unwrap().is_some_and(|t| t.elapsed() < Duration::from_millis(400));
    if w.is_visible().unwrap_or(false) {
        let _ = w.hide();
    } else if !just_hidden {
        show_main(app, at);
    }
}

pub fn show_main(app: &AppHandle, at: Option<(i32, i32)>) {
    let Some(w) = app.get_webview_window(MAIN) else { return };
    let state = app.state::<MainWindowState>();
    if let Some(p) = at {
        *state.last_click.lock().unwrap() = Some(p);
    }
    let anchor = *state.last_click.lock().unwrap();
    *state.shown_at.lock().unwrap() = Some(Instant::now());
    let _ = w.show();
    // Position after show(): GTK ignores moves of hidden windows.
    place(&w, anchor);
    let _ = w.set_focus();
    activate_x11(&w);
}

/// Window managers refuse focus to windows of background programs, and a
/// tray click does not count as user action for our window. Without focus
/// the popup would not close when clicking elsewhere. Pagers and task bars
/// may activate windows (`_NET_ACTIVE_WINDOW`, source 2) – we do the same.
/// On Wayland the GDK window is no X11 window and nothing happens.
#[cfg(target_os = "linux")]
fn activate_x11(w: &WebviewWindow) {
    let window = w.clone();
    let _ = w.run_on_main_thread(move || {
        use gtk::prelude::{Cast, WidgetExt};
        let Ok(gtk_window) = window.gtk_window() else { return };
        let Some(gdk_window) = gtk_window.window() else { return };
        let Ok(x11_window) = gdk_window.downcast::<gdkx11::X11Window>() else { return };
        send_active_window(x11_window.xid());
    });
}

#[cfg(not(target_os = "linux"))]
fn activate_x11(_w: &WebviewWindow) {}

#[cfg(target_os = "linux")]
fn send_active_window(xid: u64) {
    use x11_dl::xlib;
    let Ok(x) = xlib::Xlib::open() else { return };
    // SAFETY: plain Xlib calls on a display opened and closed here.
    unsafe {
        let display = (x.XOpenDisplay)(std::ptr::null());
        if display.is_null() {
            return;
        }
        let root = (x.XDefaultRootWindow)(display);
        let atom = (x.XInternAtom)(display, c"_NET_ACTIVE_WINDOW".as_ptr(), xlib::False);
        let mut event: xlib::XEvent = std::mem::zeroed();
        event.client_message = xlib::XClientMessageEvent {
            type_: xlib::ClientMessage,
            serial: 0,
            send_event: xlib::True,
            display,
            window: xid as xlib::Window,
            message_type: atom,
            format: 32,
            data: xlib::ClientMessageData::from([2, xlib::CurrentTime as i64, 0, 0, 0]),
        };
        (x.XSendEvent)(display, root, xlib::False, xlib::SubstructureRedirectMask | xlib::SubstructureNotifyMask, &mut event);
        (x.XFlush)(display);
        (x.XCloseDisplay)(display);
    }
}

/// Next to the tray icon, inside the work area; above the icon if the panel is at the bottom.
/// Without a known icon position: top right corner. (Wayland compositors may ignore this.)
fn place(w: &WebviewWindow, anchor: Option<(i32, i32)>) {
    let Ok(size) = w.outer_size() else { return };
    let (ww, wh) = (size.width as i32, size.height as i32);
    let monitor = match anchor {
        Some((x, y)) => w.monitor_from_point(x as f64, y as f64).ok().flatten(),
        None => None,
    }
    .or_else(|| w.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else { return };
    let area = monitor.work_area();
    let (ax, ay, aw, ah) = (area.position.x, area.position.y, area.size.width as i32, area.size.height as i32);
    let (x, y) = match anchor {
        Some((cx, cy)) => {
            let x = cx - ww / 2;
            let y = if cy > ay + ah / 2 { cy - wh - MARGIN } else { cy + MARGIN };
            (x, y)
        }
        None => (ax + aw - ww - MARGIN, ay + MARGIN),
    };
    let x = x.clamp(ax + MARGIN, (ax + aw - ww - MARGIN).max(ax));
    let y = y.clamp(ay + MARGIN, (ay + ah - wh - MARGIN).max(ay));
    let _ = w.set_position(PhysicalPosition::new(x, y));
}
