//! Tray / menu-bar icon and its menu.

use std::str::FromStr;

use rustshot_core::Config;
use tray_icon::menu::{accelerator::Accelerator, Menu, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub const CAPTURE_REGION: &str = "capture_region";
pub const CAPTURE_LAST_REGION: &str = "capture_last_region";
pub const CAPTURE_WINDOW: &str = "capture_window";
pub const CAPTURE_FULLSCREEN: &str = "capture_fullscreen";
pub const OPEN_CLIPBOARD: &str = "open_clipboard";
pub const OPEN_FILE: &str = "open_file";
pub const OPEN_FOLDER: &str = "open_folder";
pub const PREFERENCES: &str = "preferences";
pub const ABOUT: &str = "about";
pub const QUIT: &str = "quit";

pub struct Tray {
    _icon: TrayIcon,
}

fn item(id: &str, text: &str, hotkey: Option<&str>) -> MenuItem {
    let accel = hotkey.and_then(|h| Accelerator::from_str(h).ok());
    MenuItem::with_id(MenuId::new(id), text, true, accel)
}

impl Tray {
    pub fn new(config: &Config) -> anyhow::Result<Self> {
        let menu = Menu::new();
        let hk = &config.hotkeys;
        menu.append_items(&[
            &item(CAPTURE_REGION, "Capture region", Some(&hk.region)),
            &item(CAPTURE_LAST_REGION, "Capture last region", Some(&hk.last_region)),
            &item(CAPTURE_WINDOW, "Capture window", Some(&hk.window)),
            &item(CAPTURE_FULLSCREEN, "Capture full screen", Some(&hk.fullscreen)),
            &PredefinedMenuItem::separator(),
            &item(OPEN_CLIPBOARD, "Open image from clipboard", None),
            &item(OPEN_FILE, "Open image from file…", None),
            &item(OPEN_FOLDER, "Open output folder", None),
            &PredefinedMenuItem::separator(),
            &item(PREFERENCES, "Preferences…", None),
            &item(ABOUT, "About Rustshot", None),
            &PredefinedMenuItem::separator(),
            &item(QUIT, "Quit Rustshot", None),
        ])?;

        let builder = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Rustshot")
            // With a click action set, left click captures and right click opens the menu.
            .with_menu_on_left_click(config.tray_click == rustshot_core::TrayClick::ShowMenu);
        #[cfg(target_os = "macos")]
        let builder = builder.with_icon_templated(tray_icon_image());
        #[cfg(not(target_os = "macos"))]
        let builder = builder.with_icon(tray_icon_image());
        let icon = builder.build()?;
        Ok(Self { _icon: icon })
    }
}

/// A simple "selection corners + crosshair" glyph, drawn in code so the
/// binary needs no image assets. Black on transparent so macOS can tint it.
fn tray_icon_image() -> Icon {
    const S: usize = 32;
    // macOS tints template icons itself; elsewhere use a colour visible on light and dark bars.
    const INK: [u8; 4] = if cfg!(target_os = "macos") { [0, 0, 0, 255] } else { [0, 150, 230, 255] };
    let mut rgba = vec![0u8; S * S * 4];
    let mut set = |x: usize, y: usize| {
        if x < S && y < S {
            let i = (y * S + x) * 4;
            rgba[i..i + 4].copy_from_slice(&INK);
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
