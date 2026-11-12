//! Tray icon via StatusNotifierItem (ksni). Unlike libappindicator, this
//! reports left clicks, so a click can open the window. The number of unread
//! mails is drawn into the icon, which works on every desktop. A dot in the
//! bottom right corner marks open tasks, independent of the count.

use ksni::menu::StandardItem;
use ksni::{Icon, MenuItem, ToolTip, Tray, TrayMethods};
use tauri::{AppHandle, Manager};

use super::windows;
use crate::base::i18n::t;

pub struct MailletTray {
    app: AppHandle,
    count: usize,
    /// Open tasks: 0 none, 1 open, 2 at least one in progress.
    tasks: u8,
    tooltip: String,
    icons: Vec<(i32, Vec<u8>)>,
}

pub type TrayHandle = ksni::Handle<MailletTray>;

impl MailletTray {
    fn new(app: AppHandle) -> Self {
        let icons = [&include_bytes!("../../icons/32x32.png")[..], &include_bytes!("../../icons/128x128.png")[..]]
            .iter()
            .filter_map(|bytes| tauri::image::Image::from_bytes(bytes).ok())
            .map(|img| {
                let argb = rgba_to_argb(img.rgba());
                if img.width() == 128 { (64, downscale_half(&argb, 128)) } else { (img.width() as i32, argb) }
            })
            .collect();
        Self { app, count: 0, tasks: 0, tooltip: "Maillet".into(), icons }
    }
}

/// Starts the tray; `None` if there is no D-Bus session at all.
/// A missing or not yet started tray host is no error: Cinnamon's
/// xapp-sn-watcher shows icons even while reporting no registered host, and
/// at login Maillet may start before the panel. The icon appears once a host
/// picks it up.
pub async fn spawn(app: AppHandle) -> Option<TrayHandle> {
    match MailletTray::new(app).assume_sni_available(true).spawn().await {
        Ok(handle) => Some(handle),
        Err(e) => {
            tracing::warn!("No tray available ({e}) – install a StatusNotifierItem/AppIndicator host");
            None
        }
    }
}

pub async fn update(handle: &TrayHandle, count: usize, tooltip: String) {
    handle
        .update(move |t: &mut MailletTray| {
            t.count = count;
            t.tooltip = tooltip;
        })
        .await;
}

/// Open tasks changed (`level` as in `MailletTray::tasks`); the tooltip is refreshed along.
pub async fn update_tasks(handle: &TrayHandle, level: u8, tooltip: String) {
    handle
        .update(move |t: &mut MailletTray| {
            t.tasks = level;
            t.tooltip = tooltip;
        })
        .await;
}

impl Tray for MailletTray {
    fn id(&self) -> String {
        "maillet".into()
    }

    fn title(&self) -> String {
        "Maillet".into()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        self.icons
            .iter()
            .map(|(size, data)| {
                let mut data = data.clone();
                if self.count > 0 {
                    draw_badge(&mut data, *size, self.count);
                }
                draw_task_dot(&mut data, *size, self.tasks);
                Icon { width: *size, height: *size, data }
            })
            .collect()
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip { title: self.tooltip.clone(), description: t("tray.click", &[]), ..Default::default() }
    }

    fn watcher_online(&self) {
        tracing::info!("Tray host available – icon registered");
    }

    fn watcher_offline(&self, reason: ksni::OfflineReason) -> bool {
        tracing::warn!("Tray host not available yet ({reason:?}) – waiting for it");
        true
    }

    fn activate(&mut self, x: i32, y: i32) {
        windows::toggle_main(&self.app, Some((x, y)));
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let open = if self.count > 0 { t("tray.open_n", &[("n", &self.count.to_string())]) } else { t("tray.open", &[]) };
        vec![
            StandardItem { label: open, activate: Box::new(|t: &mut Self| windows::show_main(&t.app, None)), ..Default::default() }.into(),
            StandardItem { label: t("tray.check", &[]), activate: Box::new(|t: &mut Self| super::check_now_sync(&t.app)), ..Default::default() }.into(),
            MenuItem::Separator,
            StandardItem { label: t("tray.settings", &[]), activate: Box::new(|t: &mut Self| windows::show(&t.app, "settings")), ..Default::default() }.into(),
            StandardItem { label: t("tray.about", &[]), activate: Box::new(|t: &mut Self| windows::show(&t.app, "about")), ..Default::default() }.into(),
            MenuItem::Separator,
            StandardItem { label: t("tray.quit", &[]), activate: Box::new(|t: &mut Self| t.app.exit(0)), ..Default::default() }.into(),
        ]
    }
}

/// Pixels as ARGB32 in network byte order, as StatusNotifierItem expects.
fn rgba_to_argb(rgba: &[u8]) -> Vec<u8> {
    rgba.as_chunks::<4>().0.iter().flat_map(|p| [p[3], p[0], p[1], p[2]]).collect()
}

/// Halves the size by averaging 2×2 blocks.
fn downscale_half(argb: &[u8], size: usize) -> Vec<u8> {
    let half = size / 2;
    let mut out = vec![0u8; half * half * 4];
    for y in 0..half {
        for x in 0..half {
            for c in 0..4 {
                let sum: u32 = [(0, 0), (1, 0), (0, 1), (1, 1)]
                    .iter()
                    .map(|(dx, dy)| argb[((y * 2 + dy) * size + x * 2 + dx) * 4 + c] as u32)
                    .sum();
                out[(y * half + x) * 4 + c] = (sum / 4) as u8;
            }
        }
    }
    out
}

/// 3×5 pixel digits, one row per entry, bits from the left.
const DIGITS: [[u8; 5]; 10] = [
    [7, 5, 5, 5, 7], [2, 6, 2, 2, 7], [7, 1, 7, 4, 7], [7, 1, 7, 1, 7], [5, 5, 7, 1, 1],
    [7, 4, 7, 1, 7], [7, 4, 7, 5, 7], [7, 1, 1, 1, 1], [7, 5, 7, 5, 7], [7, 5, 7, 1, 7],
];

/// Red pill with the count in the top right corner. 32 px base, scaled for larger icons.
fn draw_badge(data: &mut [u8], size: i32, count: usize) {
    let s = (size / 32).max(1);
    let text = if count > 99 { "99".to_string() } else { count.to_string() };
    let h = 16 * s;
    let w = ((text.len() as i32 * 8 + 6) * s).max(h);
    let (x0, r) = (size - w, h / 2);
    let mut set = |x: i32, y: i32, px: [u8; 4]| {
        if (0..size).contains(&x) && (0..size).contains(&y) {
            let i = ((y * size + x) * 4) as usize;
            data[i..i + 4].copy_from_slice(&px);
        }
    };
    for y in 0..h {
        for x in x0..size {
            // Rounded ends: inside if within the end circles or the straight middle.
            let cx = (x - x0).clamp(r, w - r) + x0;
            if (x - cx).pow(2) + (y - r).pow(2) <= r * r {
                set(x, y, [255, 229, 72, 77]);
            }
        }
    }
    // Center the digits (each 6 px wide plus 2 px gap) in the pill.
    let mut cx = x0 + (w - (text.len() as i32 * 8 - 2) * s) / 2;
    for ch in text.chars() {
        let rows = DIGITS[ch.to_digit(10).unwrap_or(0) as usize];
        for (row, bits) in rows.iter().enumerate() {
            for col in 0..3 {
                if bits & (4 >> col) != 0 {
                    for dy in 0..2 * s {
                        for dx in 0..2 * s {
                            set(cx + col * 2 * s + dx, 3 * s + row as i32 * 2 * s + dy, [255, 255, 255, 255]);
                        }
                    }
                }
            }
        }
        cx += 8 * s;
    }
}

/// Dot in the bottom right corner for open tasks: violet = open, orange = at
/// least one in progress (the colours of the task board). White ring, so it
/// reads on light and dark panels. Nothing for `level` 0.
fn draw_task_dot(data: &mut [u8], size: i32, level: u8) {
    let color = match level {
        1 => [255, 109, 93, 252],
        2 => [255, 242, 162, 12],
        _ => return,
    };
    let s = (size / 32).max(1);
    let (outer, inner) = (6 * s, 4 * s + s / 2);
    let (cx, cy) = (size - outer, size - outer);
    for y in cy - outer..=cy + outer {
        for x in cx - outer..=cx + outer {
            let d = (x - cx).pow(2) + (y - cy).pow(2);
            if (0..size).contains(&x) && (0..size).contains(&y) && d <= outer * outer {
                let i = ((y * size + x) * 4) as usize;
                data[i..i + 4].copy_from_slice(if d <= inner * inner { &color } else { &[255, 255, 255, 255] });
            }
        }
    }
}

/// Fallback when there is no tray: show the window on start.
pub fn no_tray_fallback(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(windows::MAIN) {
        let _ = w.show();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_badge_stays_inside_and_draws_digits() {
        for (size, count) in [(32, 3), (32, 42), (64, 7), (32, 1000)] {
            let mut data = vec![0u8; (size * size * 4) as usize];
            draw_badge(&mut data, size, count);
            let px = data.as_chunks::<4>().0;
            let white = px.iter().filter(|p| **p == [255, 255, 255, 255]).count();
            let red = px.iter().filter(|p| **p == [255, 229, 72, 77]).count();
            assert!(white > 0 && red > white, "size {size}, count {count}");
            // Bottom-left corner untouched.
            let i = (((size - 1) * size) * 4) as usize;
            assert_eq!(&data[i..i + 4], &[0, 0, 0, 0]);
        }
    }

    #[test]
    fn test_task_dot_colours_and_bounds() {
        for size in [32, 64] {
            let mut none = vec![0u8; (size * size * 4) as usize];
            draw_task_dot(&mut none, size, 0);
            assert!(none.iter().all(|b| *b == 0), "no tasks, no dot");
            for (level, color) in [(1u8, [255, 109, 93, 252]), (2, [255, 242, 162, 12])] {
                let mut data = vec![0u8; (size * size * 4) as usize];
                draw_task_dot(&mut data, size, level);
                let px = data.as_chunks::<4>().0;
                assert!(px.iter().filter(|p| **p == color).count() > 10, "size {size} level {level}");
                // Top-left corner untouched; the dot touches the bottom right corner area.
                assert_eq!(&data[0..4], &[0, 0, 0, 0]);
                let last = ((size * size - 1) * 4) as usize;
                assert_eq!(&data[last..last + 4], &[0, 0, 0, 0], "corner pixel stays free");
            }
        }
    }

    #[test]
    fn test_argb_and_downscale() {
        assert_eq!(rgba_to_argb(&[1, 2, 3, 4]), vec![4, 1, 2, 3]);
        let src: Vec<u8> = (0..16).flat_map(|i| [255, i * 10, 0, 0]).collect();
        let out = downscale_half(&src, 4);
        assert_eq!(out.len(), 16);
        assert_eq!(out[1], ((10 + 40 + 50) / 4) as u8); // pixels 0, 1, 4, 5
    }
}
