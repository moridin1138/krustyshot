//! Tray / menu-bar icon and its menu.

use std::str::FromStr;

use rustshot_core::Config;
use rustshot_i18n::tr;
use tray_icon::menu::{accelerator::Accelerator, Menu, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub const CAPTURE_REGION: &str = "capture_region";
pub const CAPTURE_LAST_REGION: &str = "capture_last_region";
pub const CAPTURE_WINDOW: &str = "capture_window";
pub const CAPTURE_FULLSCREEN: &str = "capture_fullscreen";
pub const CAPTURE_SCROLLING: &str = "capture_scrolling";
pub const RECORD_REGION: &str = "record_region";
pub const RECORD_WINDOW: &str = "record_window";
pub const RECORD_FULLSCREEN: &str = "record_fullscreen";
pub const RECORD_STOP: &str = "record_stop";
pub const OPEN_CLIPBOARD: &str = "open_clipboard";
pub const OPEN_FILE: &str = "open_file";
pub const OPEN_FOLDER: &str = "open_folder";
pub const PREFERENCES: &str = "preferences";
pub const ABOUT: &str = "about";
pub const QUIT: &str = "quit";
pub const UPDATE: &str = "update";

pub struct Tray {
    _icon: TrayIcon,
}

fn item(id: &str, text: &str, hotkey: Option<&str>) -> MenuItem {
    let accel = hotkey.and_then(|h| Accelerator::from_str(h).ok());
    MenuItem::with_id(MenuId::new(id), text, true, accel)
}

impl Tray {
    /// `recording`: show "Stop recording" and a red icon. `update`: a newer
    /// version to offer at the top of the menu.
    pub fn new(config: &Config, recording: bool, update: Option<&str>) -> anyhow::Result<Self> {
        let menu = Menu::new();
        if let Some(version) = update {
            menu.append_items(&[&item(UPDATE, &tr!("menu-update-available", version = version), None), &PredefinedMenuItem::separator()])?;
        }
        let hk = &config.hotkeys;
        menu.append_items(&[
            &item(CAPTURE_REGION, &tr!("capture-region"), Some(&hk.region)),
            &item(CAPTURE_LAST_REGION, &tr!("capture-last-region"), Some(&hk.last_region)),
            &item(CAPTURE_WINDOW, &tr!("capture-window"), Some(&hk.window)),
            &item(CAPTURE_FULLSCREEN, &tr!("capture-fullscreen"), Some(&hk.fullscreen)),
            &item(CAPTURE_SCROLLING, &tr!("capture-scrolling"), Some(&hk.scroll)),
            &PredefinedMenuItem::separator(),
        ])?;
        if recording {
            menu.append(&item(RECORD_STOP, &tr!("record-stop"), Some(&hk.record)))?;
        } else {
            menu.append_items(&[
                &item(RECORD_REGION, &tr!("record-region"), Some(&hk.record)),
                &item(RECORD_WINDOW, &tr!("record-window"), None),
                &item(RECORD_FULLSCREEN, &tr!("record-fullscreen"), None),
            ])?;
        }
        menu.append_items(&[
            &PredefinedMenuItem::separator(),
            &item(OPEN_CLIPBOARD, &tr!("menu-open-clipboard"), None),
            &item(OPEN_FILE, &tr!("menu-open-file"), None),
            &item(OPEN_FOLDER, &tr!("menu-open-folder"), None),
            &PredefinedMenuItem::separator(),
            &item(PREFERENCES, &tr!("menu-settings"), None),
            &item(ABOUT, &tr!("menu-about"), None),
            &PredefinedMenuItem::separator(),
            &item(QUIT, &tr!("menu-quit"), None),
        ])?;

        let builder = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Krustyshot")
            // With a click action set, left click captures and right click opens the menu.
            .with_menu_on_left_click(config.tray_click == rustshot_core::TrayClick::ShowMenu);
        #[cfg(target_os = "macos")]
        let builder = if recording {
            // Not a template icon, so macOS keeps it red.
            builder.with_icon(tray_icon_image(true))
        } else {
            builder.with_icon_templated(tray_icon_image(false))
        };
        #[cfg(not(target_os = "macos"))]
        let builder = builder.with_icon(tray_icon_image(recording));
        let icon = builder.build()?;
        Ok(Self { _icon: icon })
    }
}

/// A simple "selection corners + crosshair" glyph, drawn in code so the
/// binary needs no image assets. Black on transparent so macOS can tint it.
fn tray_icon_image(recording: bool) -> Icon {
    const S: usize = 32;
    // macOS tints template icons itself; elsewhere use a colour visible on light and dark bars.
    const INK: [u8; 4] = if cfg!(target_os = "macos") { [0, 0, 0, 255] } else { [0, 150, 230, 255] };
    const RED: [u8; 4] = [230, 40, 40, 255];
    let ink = if recording { RED } else { INK };
    let mut rgba = vec![0u8; S * S * 4];
    let mut set = |x: usize, y: usize| {
        if x < S && y < S {
            let i = (y * S + x) * 4;
            rgba[i..i + 4].copy_from_slice(&ink);
        }
    };
    let (lo, hi, arm, t) = (3usize, 28usize, 8usize, 3usize);
    for k in 0..arm {
        for w in 0..t {
            // Corner brackets.
            for (cx, cy, dx, dy) in [(lo, lo, 1i32, 1i32), (hi, lo, -1, 1), (lo, hi, 1, -1), (hi, hi, -1, -1)] {
                let x = (cx as i32 + dx * k as i32) as usize;
                let y = (cy as i32 + dy * w as i32) as usize;
                set(x, y);
                let x = (cx as i32 + dx * w as i32) as usize;
                let y = (cy as i32 + dy * k as i32) as usize;
                set(x, y);
            }
        }
    }
    // Centre crosshair.
    for k in 11..=20 {
        for w in 15..=16 {
            set(k, w);
            set(w, k);
        }
    }
    Icon::from_rgba(rgba, S as u32, S as u32).expect("valid icon")
}
