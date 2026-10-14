//! Interface fonts: each OS's own UI font where we can find it, at
//! comfortable sizes. Text typed into screenshots keeps the bundled font
//! (see `rustshot_editor::ANNOTATION_FAMILY`).

use std::sync::Arc;

use egui::{FontData, FontFamily, FontId, TextStyle};

/// Candidate UI font files, best first: (path, index within a collection).
fn candidates() -> &'static [(&'static str, u32)] {
    if cfg!(target_os = "macos") {
        &[
            ("/System/Library/Fonts/SFNS.ttf", 0),
            ("/System/Library/Fonts/HelveticaNeue.ttc", 0),
            ("/System/Library/Fonts/Helvetica.ttc", 0),
        ]
    } else if cfg!(windows) {
        &[(r"C:\Windows\Fonts\segoeui.ttf", 0), (r"C:\Windows\Fonts\arial.ttf", 0)]
    } else {
        &[
            ("/usr/share/fonts/cantarell/Cantarell-VF.otf", 0),
            ("/usr/share/fonts/opentype/cantarell/Cantarell-VF.otf", 0),
            ("/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf", 0),
            ("/usr/share/fonts/noto/NotoSans-Regular.ttf", 0),
            ("/usr/share/fonts/truetype/ubuntu/Ubuntu-R.ttf", 0),
            ("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", 0),
            ("/usr/share/fonts/TTF/DejaVuSans.ttf", 0),
        ]
    }
}

fn system_ui_font() -> Option<FontData> {
    candidates().iter().find_map(|(path, index)| {
        let bytes = std::fs::read(path).ok()?;
        log::debug!("interface font: {path}");
        let mut data = FontData::from_owned(bytes);
        data.index = *index;
        Some(data)
    })
}

pub fn install(ctx: &egui::Context) {
    let mut fonts = rustshot_editor::font_definitions();
    if let Some(data) = system_ui_font() {
        fonts.font_data.insert("system-ui".into(), Arc::new(data));
        if let Some(prop) = fonts.families.get_mut(&FontFamily::Proportional) {
            // The system font first; the bundled fonts stay as fallbacks for
            // anything it lacks (and Phosphor for icons).
            prop.insert(0, "system-ui".into());
        }
    }
    ctx.set_fonts(fonts);

    // Slightly larger than egui's defaults; the small size in particular.
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (TextStyle::Small, FontId::proportional(11.5)),
            (TextStyle::Body, FontId::proportional(14.0)),
            (TextStyle::Button, FontId::proportional(14.0)),
            (TextStyle::Heading, FontId::proportional(21.0)),
            (TextStyle::Monospace, FontId::monospace(13.0)),
        ]
        .into();
    });
    // Secondary ("weak") text with more contrast than egui's default.
    ctx.style_mut_of(egui::Theme::Light, |s| s.visuals.weak_text_color = Some(egui::Color32::from_gray(105)));
    ctx.style_mut_of(egui::Theme::Dark, |s| s.visuals.weak_text_color = Some(egui::Color32::from_gray(165)));
}
