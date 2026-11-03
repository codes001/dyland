use std::sync::Mutex;
use std::time::Duration;

use gdk::prelude::*;
use gtk::prelude::*;
use tauri::Manager;

use crate::logging;

static LAST_PLACE: Mutex<Option<(i32, i32, i32, i32)>> = Mutex::new(None);

pub const OVERLAY_WIDTH: i32 = 440;
pub const OVERLAY_HEIGHT_COMPACT: i32 = 96;
pub const OVERLAY_HEIGHT_TALL: i32 = 760;

static MODE: Mutex<Option<(bool, bool)>> = Mutex::new(None);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PxRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(serde::Deserialize)]
pub struct InputRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub fn install_shutdown(app: &tauri::AppHandle) {
    for signal in [libc::SIGTERM, libc::SIGINT] {
        let app = app.clone();
        glib::unix_signal_add(signal, move || {
            logging::info(&format!("received signal {signal}, shutting down"));
            app.exit(0);
            glib::ControlFlow::Break
        });
    }
}

pub fn attach(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "main window missing"))?;
    let gtk_window = window.gtk_window()?;
    gtk_window.set_type_hint(gdk::WindowTypeHint::Utility);
    apply_hints(&gtk_window);
    if gtk_window.window().is_none() {
        gtk_window.realize();
    }
    gtk_window.resize(OVERLAY_WIDTH, OVERLAY_HEIGHT_COMPACT);
    place(&gtk_window);
    set_shape(&gtk_window, &default_rects(false));
    watch_monitors(&gtk_window);
    for delay_ms in [300_u64, 1000, 2500] {
        let gtk_window = gtk_window.clone();
        glib::timeout_add_local(Duration::from_millis(delay_ms), move || {
            apply_hints(&gtk_window);
            place(&gtk_window);
            glib::ControlFlow::Break
        });
    }
    logging::info("overlay window attached");
    Ok(())
}

pub fn set_mode(window: &tauri::WebviewWindow, visible: bool, tall: bool) -> Result<(), String> {
    {
        let mut mode = MODE.lock().map_err(|err| err.to_string())?;
        if *mode == Some((visible, tall)) {
            return Ok(());
        }
        *mode = Some((visible, tall));
    }
    let gtk_window = window.gtk_window().map_err(|err| err.to_string())?;
    apply_hints(&gtk_window);
    if !visible {
        set_shape(&gtk_window, &[]);
        gtk_window.hide();
        logging::info("overlay hidden");
        return Ok(());
    }
    let height = if tall {
        OVERLAY_HEIGHT_TALL
    } else {
        OVERLAY_HEIGHT_COMPACT
    };
    gtk_window.resize(OVERLAY_WIDTH, height);
    place(&gtk_window);
    set_shape(&gtk_window, &default_rects(tall));
    gtk_window.show();
    let _ = window.show();
    logging::info(&format!(
        "overlay mode visible={visible} tall={tall} size={OVERLAY_WIDTH}x{height}"
    ));
    Ok(())
}

pub fn set_input_regions(
    window: &tauri::WebviewWindow,
    regions: &[InputRect],
) -> Result<(), String> {
    let gtk_window = window.gtk_window().map_err(|err| err.to_string())?;
    let allocation = gtk_window.allocation();
    let window_w = allocation.width().max(OVERLAY_WIDTH);
    let window_h = allocation.height().max(OVERLAY_HEIGHT_COMPACT);
    let rects: Vec<PxRect> = regions
        .iter()
        .filter_map(|region| {
            css_rect_to_window(
                region.x,
                region.y,
                region.width,
                region.height,
                window_w,
                window_h,
            )
        })
        .take(16)
        .collect();
    set_shape(&gtk_window, &rects);
    Ok(())
}

pub fn raise_existing(window: &tauri::WebviewWindow) {
    let Ok(gtk_window) = window.gtk_window() else {
        return;
    };
    apply_hints(&gtk_window);
    let tall = MODE
        .lock()
        .ok()
        .and_then(|mode| *mode)
        .map(|(_, tall)| tall)
        .unwrap_or(false);
    let height = if tall {
        OVERLAY_HEIGHT_TALL
    } else {
        OVERLAY_HEIGHT_COMPACT
    };
    gtk_window.resize(OVERLAY_WIDTH, height);
    place(&gtk_window);
    gtk_window.deiconify();
    gtk_window.show();
    let _ = window.show();
    let _ = window.unminimize();
    logging::info("brought the running island forward");
}

fn apply_hints(window: &gtk::ApplicationWindow) {
    window.set_decorated(false);
    window.set_resizable(false);
    window.set_skip_taskbar_hint(true);
    window.set_skip_pager_hint(true);
    window.set_keep_above(true);
    window.set_accept_focus(false);
    window.set_focus_on_map(false);
    window.stick();
    window.set_role("dyland-overlay");
    window.set_title("dyland");
}

fn place(window: &gtk::ApplicationWindow) {
    let (monitor, source) =
        match gdk::Display::default().and_then(|display| select_monitor(&display)) {
            Some(found) => found,
            None => {
                logging::error("no monitor available; overlay left at the compositor default");
                write_health(window, "degraded", "none");
                return;
            }
        };
    let geometry = monitor.geometry();
    let x = overlay_origin(geometry.x(), geometry.width(), OVERLAY_WIDTH);
    let y = geometry.y();
    let key = (x, y, geometry.width(), geometry.height());
    let changed = LAST_PLACE
        .lock()
        .map(|mut last| {
            let changed = *last != Some(key);
            *last = Some(key);
            changed
        })
        .unwrap_or(true);
    window.move_(x, y);
    write_health(window, "ok", source);
    if changed {
        logging::info(&format!(
            "overlay placed at {x},{y} on {source} monitor {}x{} scale {}",
            geometry.width(),
            geometry.height(),
            monitor.scale_factor()
        ));
    }
}

fn watch_monitors(window: &gtk::ApplicationWindow) {
    let Some(display) = gdk::Display::default() else {
        return;
    };
    let added = window.clone();
    display.connect_monitor_added(move |_, _| place(&added));
    let removed = window.clone();
    display.connect_monitor_removed(move |_, _| place(&removed));
    for index in 0..display.n_monitors() {
        if let Some(monitor) = display.monitor(index) {
            let window = window.clone();
            monitor.connect_invalidate(move |_| place(&window));
        }
    }
}

fn select_monitor(display: &gdk::Display) -> Option<(gdk::Monitor, &'static str)> {
    if let Some(primary) = display.primary_monitor() {
        return Some((primary, "gdk-primary"));
    }
    let mut best: Option<gdk::Monitor> = None;
    let mut best_key = i32::MAX;
    for index in 0..display.n_monitors() {
        let Some(monitor) = display.monitor(index) else {
            continue;
        };
        let geometry = monitor.geometry();
        let key = geometry.x().saturating_abs() + geometry.y().saturating_abs();
        if key < best_key {
            best_key = key;
            best = Some(monitor);
        }
    }
    best.map(|monitor| (monitor, "origin"))
}

fn default_rects(tall: bool) -> Vec<PxRect> {
    if tall {
        vec![PxRect {
            x: 0,
            y: 0,
            width: OVERLAY_WIDTH,
            height: OVERLAY_HEIGHT_TALL,
        }]
    } else {
        vec![PxRect {
            x: 147,
            y: 6,
            width: 146,
            height: 52,
        }]
    }
}

fn set_shape(window: &gtk::ApplicationWindow, rects: &[PxRect]) {
    let Some(gdk_window) = window.window() else {
        return;
    };
    let shape = gtk::cairo::Region::create();
    for rect in rects {
        let cairo_rect = gtk::cairo::RectangleInt::new(rect.x, rect.y, rect.width, rect.height);
        if shape.union_rectangle(&cairo_rect).is_err() {
            logging::error("failed to union an input rectangle");
            return;
        }
    }
    gdk_window.input_shape_combine_region(&shape, 0, 0);
}

fn write_health(window: &gtk::ApplicationWindow, status: &str, monitor: &str) {
    let allocation = window.allocation();
    logging::write_health(&serde_json::json!({
        "status": status,
        "pid": std::process::id(),
        "version": env!("CARGO_PKG_VERSION"),
        "monitor": monitor,
        "units": "logical-px",
        "overlay": {
            "width": allocation.width(),
            "height": allocation.height(),
        }
    }));
}

pub fn overlay_origin(monitor_x: i32, monitor_width: i32, overlay_width: i32) -> i32 {
    monitor_x + monitor_width.saturating_sub(overlay_width) / 2
}

pub fn css_rect_to_window(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    window_w: i32,
    window_h: i32,
) -> Option<PxRect> {
    if !x.is_finite() || !y.is_finite() || !width.is_finite() || !height.is_finite() {
        return None;
    }
    if width < 1.0 || height < 1.0 || window_w <= 0 || window_h <= 0 {
        return None;
    }
    let x0 = (x - 2.0).floor() as i32;
    let y0 = (y - 2.0).floor() as i32;
    let x1 = (x + width + 2.0).ceil() as i32;
    let y1 = (y + height + 2.0).ceil() as i32;
    let x0 = x0.clamp(0, window_w);
    let y0 = y0.clamp(0, window_h);
    let x1 = x1.clamp(0, window_w);
    let y1 = y1.clamp(0, window_h);
    let rect_w = x1 - x0;
    let rect_h = y1 - y0;
    if rect_w < 1 || rect_h < 1 {
        None
    } else {
        Some(PxRect {
            x: x0,
            y: y0,
            width: rect_w,
            height: rect_h,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centers_on_the_primary_monitor() {
        assert_eq!(overlay_origin(0, 1920, 440), 740);
        assert_eq!(overlay_origin(-1920, 1920, 440), -1180);
        assert_eq!(overlay_origin(0, 400, 440), 0);
    }

    #[test]
    fn clamps_hit_rect_to_the_window() {
        let rect = css_rect_to_window(10.0, 8.0, 126.0, 40.0, 440, 96).unwrap();
        assert_eq!(
            rect,
            PxRect {
                x: 8,
                y: 6,
                width: 130,
                height: 44
            }
        );
        assert!(css_rect_to_window(500.0, 10.0, 20.0, 20.0, 440, 96).is_none());
        assert!(css_rect_to_window(0.0, 0.0, f64::NAN, 10.0, 440, 96).is_none());
    }
}
