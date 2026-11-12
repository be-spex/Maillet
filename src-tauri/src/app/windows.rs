//! Windows: the Maillet window, settings, about and the OTP code popup.
//!
//! The Maillet window has two forms:
//! - **Popup** (tray click): next to the tray icon, fixed size, hides when
//!   clicking elsewhere.
//! - **Sidebar** (global shortcut): docked to the right edge, full height,
//!   stays open until closed.

use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};

pub const MAIN: &str = "main";
/// Fixed window size (logical px): enough to read a mail comfortably. It is
/// both default and minimum; features the user turns on may add to it later.
/// The size never changes with the content – no resizing, no flicker.
const WIDTH: f64 = 700.0;
const HEIGHT: f64 = 700.0;
/// Gap between window and tray icon / screen edge.
const MARGIN: i32 = 8;

#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub enum Form {
    #[default]
    Popup,
    Sidebar,
}

#[derive(Default)]
pub struct MainWindowState {
    form: Mutex<Form>,
    last_click: Mutex<Option<(i32, i32)>>,
    /// Last geometry sent to the window (x, y, physical height).
    applied: Mutex<Option<(i32, i32, i32)>>,
    hidden_on_blur: Mutex<Option<Instant>>,
    shown_at: Mutex<Option<Instant>>,
}

pub fn create(app: &tauri::App) -> tauri::Result<()> {
    let main = WebviewWindowBuilder::new(app, MAIN, WebviewUrl::App("app.html".into()))
        .title("Maillet")
        .inner_size(WIDTH, HEIGHT)
        .decorations(false)
        .skip_taskbar(true)
        .visible(false)
        .build()?;
    let handle = app.handle().clone();
    main.on_window_event(move |event| match event {
        // Popup behaviour: clicking elsewhere hides the window. The sidebar stays.
        WindowEvent::Focused(false) => {
            let state = handle.state::<MainWindowState>();
            let just_shown = state.shown_at.lock().unwrap().is_some_and(|t| t.elapsed() < Duration::from_millis(400));
            if *state.form.lock().unwrap() == Form::Popup && !just_shown {
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
        .title(crate::base::i18n::t("win.settings", &[]))
        .inner_size(820.0, 760.0)
        .visible(false)
        .build()?;
    hide_on_close(&settings);

    let about = WebviewWindowBuilder::new(app, "about", WebviewUrl::App("about.html".into()))
        .title(crate::base::i18n::t("win.about", &[]))
        .inner_size(360.0, 340.0)
        .resizable(false)
        .visible(false)
        .build()?;
    hide_on_close(&about);

    WebviewWindowBuilder::new(app, "code", WebviewUrl::App("code.html".into()))
        .title(crate::base::i18n::t("win.code", &[]))
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

fn current_form(app: &AppHandle) -> Form {
    *app.state::<MainWindowState>().form.lock().unwrap()
}

/// Tray click: popup at the icon; closes an open popup, turns the sidebar into the popup.
pub fn toggle_main(app: &AppHandle, at: Option<(i32, i32)>) {
    let Some(w) = app.get_webview_window(MAIN) else { return };
    let state = app.state::<MainWindowState>();
    // Clicking the icon first takes the focus away, which already hid the popup.
    let just_hidden = state.hidden_on_blur.lock().unwrap().is_some_and(|t| t.elapsed() < Duration::from_millis(400));
    if w.is_visible().unwrap_or(false) && current_form(app) == Form::Popup {
        let _ = w.hide();
    } else if !just_hidden {
        show_main(app, at);
    }
}

/// Global shortcut: opens the sidebar, or closes the window if it is open.
pub fn toggle_sidebar(app: &AppHandle) {
    let Some(w) = app.get_webview_window(MAIN) else { return };
    // The key grab of the shortcut takes the focus away, which may already
    // have hidden the popup a moment ago – then the window counts as open.
    let just_hidden = app.state::<MainWindowState>().hidden_on_blur.lock().unwrap().is_some_and(|t| t.elapsed() < Duration::from_millis(400));
    if w.is_visible().unwrap_or(false) {
        let _ = w.hide();
    } else if !just_hidden {
        show_form(app, Form::Sidebar, None);
    }
}

pub fn show_main(app: &AppHandle, at: Option<(i32, i32)>) {
    show_form(app, Form::Popup, at);
}

fn show_form(app: &AppHandle, form: Form, at: Option<(i32, i32)>) {
    let Some(w) = app.get_webview_window(MAIN) else { return };
    let state = app.state::<MainWindowState>();
    *state.form.lock().unwrap() = form;
    if let Some(p) = at {
        *state.last_click.lock().unwrap() = Some(p);
    }
    *state.shown_at.lock().unwrap() = Some(Instant::now());
    // Switching the form of an open window: hide it first, so it never shows
    // up with the size of the other form.
    if w.is_visible().unwrap_or(false) {
        let _ = w.hide();
    }
    // Size while hidden, then show; GTK ignores moves of hidden windows, so
    // the position is set (again) after showing.
    layout(app, &w);
    let _ = w.show();
    layout(app, &w);
    // Both forms must stay visible above other always-on-top windows (a drop-down
    // terminal such as Guake) when summoned.
    let _ = w.set_always_on_top(true);
    let _ = w.set_focus();
    activate_x11(&w);
}

/// Sets size and position for the current form.
fn layout(app: &AppHandle, w: &WebviewWindow) {
    let state = app.state::<MainWindowState>();
    let form = *state.form.lock().unwrap();
    let anchor = *state.last_click.lock().unwrap();
    // The monitor of the tray icon, else the one under the mouse, else the primary.
    let point = anchor.or_else(|| app.cursor_position().ok().map(|p| (p.x as i32, p.y as i32)));
    let monitor = point
        .and_then(|(x, y)| w.monitor_from_point(x as f64, y as f64).ok().flatten())
        .or_else(|| w.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else { return };
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let (ax, ay, aw, ah) = (area.position.x, area.position.y, area.size.width as i32, area.size.height as i32);
    let width = ((WIDTH * scale).round() as i32).min(aw);

    let (height, x, y) = match form {
        Form::Sidebar => (ah, ax + aw - width, ay),
        Form::Popup => {
            // Smaller only if the screen itself is smaller.
            let height = ((HEIGHT * scale).round() as i32).min(ah - 2 * MARGIN);
            let (x, y) = match anchor {
                // Above the icon if the panel is at the bottom, else below it.
                Some((cx, cy)) => (cx - width / 2, if cy > ay + ah / 2 { cy - height - MARGIN } else { cy + MARGIN }),
                None => (ax + aw - width - MARGIN, ay + MARGIN),
            };
            let x = x.clamp(ax + MARGIN, (ax + aw - width - MARGIN).max(ax));
            let y = y.clamp(ay + MARGIN, (ay + ah - height - MARGIN).max(ay));
            (height, x, y)
        }
    };
    // Only send what changed: every resize or move is visible as a flicker.
    // A hidden window is always set in full (it may have been moved meanwhile).
    let visible = w.is_visible().unwrap_or(false);
    let before = state.applied.lock().unwrap().replace((x, y, height));
    let (resize, moved) = match before {
        Some((bx, by, bh)) if visible => (bh != height, (bx, by) != (x, y)),
        _ => (true, true),
    };
    // (Wayland compositors may ignore positions.)
    if resize {
        let _ = w.set_size(LogicalSize::new(width as f64 / scale, height as f64 / scale));
    }
    if !moved {
        return;
    }
    let _ = w.set_position(PhysicalPosition::new(x, y));
    // The window manager keeps a window on screen: a move that arrives while
    // the window still has its old (larger) size gets clamped. Move again once
    // the resize has gone through.
    let w = w.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(80));
        let _ = w.set_position(PhysicalPosition::new(x, y));
    });
}

/// Window managers refuse focus to windows of background programs, and a
/// tray click does not count as user action for our window. Without focus
/// the popup would not close when clicking elsewhere. Pagers and task bars
/// may activate windows (`_NET_ACTIVE_WINDOW`, source 2) – we do the same.
/// On Wayland the GDK window is no X11 window and nothing happens.
#[cfg(target_os = "linux")]
fn activate_x11(w: &WebviewWindow) {
    request_activation(w);
    // On the very first show the window may not be mapped yet and the
    // request is ignored; ask again shortly after unless it worked.
    let w = w.clone();
    std::thread::spawn(move || {
        for delay in [150, 250, 400, 700] {
            std::thread::sleep(Duration::from_millis(delay));
            if w.is_visible().unwrap_or(false) && !w.is_focused().unwrap_or(false) {
                request_activation(&w);
            }
        }
    });
}

#[cfg(target_os = "linux")]
fn request_activation(w: &WebviewWindow) {
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
