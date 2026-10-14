//! The editor window UI: toolbar, properties bar, canvas and interaction.

use egui::{
    Align2, Color32, FontId, Key, Modifiers, Pos2, Rect, Sense, Shape, Stroke, StrokeKind,
    TextureHandle, TextureOptions, Ui, Vec2,
};
use image::RgbaImage;

use crate::effects::Effect;
use crate::document::{Document, Element, Handle, HighlightMode, History, Kind, ObfuscateMode, Style, SPEECH_PADDING};
use crate::export::{self, arrow_head, contrasting_text, measure_text, speech_radius, speech_tail_base, TEXT_PADDING};
use crate::raster;

/// Something the host application must do on the editor's behalf.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorRequest {
    Save,
    SaveAs,
    Copy,
    Close,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tool {
    Select,
    Rect,
    Ellipse,
    Line,
    Arrow,
    Freehand,
    Text,
    Speech,
    Counter,
    Highlight,
    Obfuscate,
    Crop,
}

impl Tool {
    const ALL: [Tool; 12] = [
        Tool::Select,
        Tool::Rect,
        Tool::Ellipse,
        Tool::Line,
        Tool::Arrow,
        Tool::Freehand,
        Tool::Text,
        Tool::Speech,
        Tool::Counter,
        Tool::Highlight,
        Tool::Obfuscate,
        Tool::Crop,
    ];

    fn kind(self) -> Option<Kind> {
        Some(match self {
            Tool::Rect => Kind::Rect,
            Tool::Ellipse => Kind::Ellipse,
            Tool::Line => Kind::Line,
            Tool::Arrow => Kind::Arrow,
            Tool::Freehand => Kind::Freehand,
            Tool::Text => Kind::Text,
            Tool::Speech => Kind::Speech,
            Tool::Counter => Kind::Counter,
            Tool::Highlight => Kind::Highlight,
            Tool::Obfuscate => Kind::Obfuscate,
            Tool::Select | Tool::Crop => return None,
        })
    }

    fn label(self) -> &'static str {
        match self {
            Tool::Select => "Select",
            Tool::Rect => "Rectangle",
            Tool::Ellipse => "Ellipse",
            Tool::Line => "Line",
            Tool::Arrow => "Arrow",
            Tool::Freehand => "Freehand",
            Tool::Text => "Text",
            Tool::Speech => "Speech bubble",
            Tool::Counter => "Step counter",
            Tool::Highlight => "Highlight",
            Tool::Obfuscate => "Obfuscate",
            Tool::Crop => "Crop",
        }
    }

    fn icon(self) -> &'static str {
        use egui_phosphor::regular as icon;
        match self {
            Tool::Select => icon::CURSOR,
            Tool::Rect => icon::SQUARE,
            Tool::Ellipse => icon::CIRCLE,
            Tool::Line => icon::LINE_SEGMENT,
            Tool::Arrow => icon::ARROW_UP_RIGHT,
            Tool::Freehand => icon::PENCIL_SIMPLE,
            Tool::Text => icon::TEXT_T,
            Tool::Speech => icon::CHAT_TEXT,
            Tool::Counter => icon::NUMBER_CIRCLE_ONE,
            Tool::Highlight => icon::HIGHLIGHTER,
            Tool::Obfuscate => icon::CHECKERBOARD,
            Tool::Crop => icon::CROP,
        }
    }

    /// Greenshot-style single-letter shortcuts.
    fn key(self) -> Key {
        match self {
            Tool::Select => Key::Escape,
            Tool::Rect => Key::R,
            Tool::Ellipse => Key::E,
            Tool::Line => Key::L,
            Tool::Arrow => Key::A,
            Tool::Freehand => Key::F,
            Tool::Text => Key::T,
            Tool::Speech => Key::S,
            Tool::Counter => Key::I,
            Tool::Highlight => Key::H,
            Tool::Obfuscate => Key::O,
            Tool::Crop => Key::C,
        }
    }
}

enum Drag {
    None,
    Create(Element),
    Move { idx: usize, before: Document, last: Pos2 },
    Resize { idx: usize, handle: Handle, before: Document },
    Crop { start: Pos2 },
}

/// Maps between image pixels and screen points.
#[derive(Clone, Copy)]
struct View {
    origin: Pos2,
    scale: f32,
}

impl View {
    fn to_screen(self, p: Pos2) -> Pos2 {
        self.origin + p.to_vec2() * self.scale
    }

    fn to_image(self, p: Pos2) -> Pos2 {
        ((p - self.origin) / self.scale).to_pos2()
    }

    fn rect(self, r: Rect) -> Rect {
        Rect::from_min_max(self.to_screen(r.min), self.to_screen(r.max))
    }
}

pub struct Editor {
    pub title: String,
    doc: Document,
    history: History,
    tool: Tool,
    style: Style,
    selected: Option<usize>,
    editing_text: Option<usize>,
    focus_text: bool,
    drag: Drag,
    crop_rect: Option<Rect>,
    /// Zoom relative to 100% (one image pixel per physical pixel). `None` = fit.
    zoom: Option<f32>,
    texture: Option<TextureHandle>,
    texture_rev: u64,
    revision: u64,
    dirty: bool,
    prop_editing: bool,
    status: String,
    /// Settings remembered by the Effects menu.
    border_width: u32,
    border_color: Color32,
    resize_percent: f32,
}

impl Editor {
    pub fn new(image: RgbaImage, title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            doc: Document::new(image),
            history: History::default(),
            tool: Tool::Rect,
            style: Style::default(),
            selected: None,
            editing_text: None,
            focus_text: false,
            drag: Drag::None,
            crop_rect: None,
            zoom: None,
            texture: None,
            texture_rev: u64::MAX,
            revision: 0,
            dirty: false,
            prop_editing: false,
            status: String::new(),
            border_width: 8,
            border_color: Color32::WHITE,
            resize_percent: 50.0,
        }
    }

    /// Place a text annotation and give it keyboard focus, as if the user had
    /// clicked with the Text tool. Used by the app's debug stress test.
    #[doc(hidden)]
    pub fn debug_start_text(&mut self, text: &str) {
        self.set_tool(Tool::Text);
        self.checkpoint();
        let mut e = Element::new(Kind::Text, Pos2::new(20.0, 20.0), self.style);
        e.text = text.to_string();
        self.doc.elements.push(e);
        self.start_text_edit(self.doc.elements.len() - 1);
    }

    /// The annotated image as it will be saved.
    pub fn flatten(&self) -> RgbaImage {
        export::flatten(&self.doc)
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
    }

    pub fn image_size(&self) -> (u32, u32) {
        self.doc.base.dimensions()
    }

    fn touched(&mut self) {
        self.revision += 1;
        self.dirty = true;
    }

    fn checkpoint(&mut self) {
        self.history.push(&self.doc);
    }

    fn undo(&mut self) {
        self.finish_text_edit();
        if self.history.undo(&mut self.doc) {
            self.selected = None;
            self.touched();
        }
    }

    fn redo(&mut self) {
        self.finish_text_edit();
        if self.history.redo(&mut self.doc) {
            self.selected = None;
            self.touched();
        }
    }

    fn delete_selected(&mut self) {
        if let Some(i) = self.selected.take()
            && i < self.doc.elements.len() {
                self.checkpoint();
                self.doc.elements.remove(i);
                self.touched();
            }
    }

    fn set_tool(&mut self, tool: Tool) {
        self.finish_text_edit();
        self.tool = tool;
        self.crop_rect = None;
        if tool != Tool::Select {
            self.selected = None;
        }
    }

    fn apply_crop(&mut self) {
        if let Some(r) = self.crop_rect.take() {
            let before = self.doc.clone();
            if self.doc.crop(r) {
                self.history.push(&before);
                self.selected = None;
                self.touched();
            }
        }
    }

    /// Add a new speech bubble (from a drag or a click) and start typing in it.
    fn add_speech(&mut self, mut e: Element) {
        let r = e.bounds();
        if r.width() < 40.0 || r.height() < 30.0 {
            e.a = r.min;
            e.b = r.min + Vec2::new(200.0, 70.0);
        }
        if e.style.fill.a() == 0 {
            e.style.fill = Color32::WHITE;
        }
        e.default_tail();
        self.checkpoint();
        self.doc.elements.push(e);
        self.start_text_edit(self.doc.elements.len() - 1);
        self.touched();
    }

    fn apply_effect(&mut self, effect: Effect) {
        self.finish_text_edit();
        self.crop_rect = None;
        self.selected = None;
        self.checkpoint();
        self.doc.apply_effect(effect);
        self.touched();
    }

    fn effects_menu(&mut self, ui: &mut Ui) {
        if ui.button("Drop shadow").clicked() {
            self.apply_effect(Effect::DropShadow { offset: 6, blur: 5.0 });
            ui.close();
        }
        if ui.button("Torn edges").clicked() {
            self.apply_effect(Effect::TornEdges { tooth: 10 });
            ui.close();
        }
        ui.horizontal(|ui| {
            if ui.button("Border").clicked() {
                self.apply_effect(Effect::Border { width: self.border_width, color: self.border_color });
                ui.close();
            }
            ui.add(egui::DragValue::new(&mut self.border_width).range(1..=200).suffix(" px"));
            egui::color_picker::color_edit_button_srgba(ui, &mut self.border_color, egui::color_picker::Alpha::Opaque);
        });
        ui.separator();
        if ui.button("Greyscale").clicked() {
            self.apply_effect(Effect::Grayscale);
            ui.close();
        }
        if ui.button("Invert colours").clicked() {
            self.apply_effect(Effect::Invert);
            ui.close();
        }
        ui.separator();
        if ui.button("Rotate left").clicked() {
            self.apply_effect(Effect::RotateCounterClockwise);
            ui.close();
        }
        if ui.button("Rotate right").clicked() {
            self.apply_effect(Effect::RotateClockwise);
            ui.close();
        }
        ui.horizontal(|ui| {
            if ui.button("Resize").clicked() {
                self.apply_effect(Effect::Resize { scale: self.resize_percent / 100.0 });
                ui.close();
            }
            ui.add(egui::DragValue::new(&mut self.resize_percent).range(5.0..=800.0).speed(1.0).suffix(" %"));
            let (w, h) = self.image_size();
            let f = self.resize_percent / 100.0;
            ui.weak(format!("{} × {}", (w as f32 * f).round(), (h as f32 * f).round()));
        });
    }

    fn start_text_edit(&mut self, idx: usize) {
        self.editing_text = Some(idx);
        self.selected = Some(idx);
        self.focus_text = true;
        if let Some(e) = self.doc.elements.get(idx) {
            self.style = e.style;
        }
    }

    fn finish_text_edit(&mut self) {
        if let Some(i) = self.editing_text.take() {
            if self.doc.elements.get(i).is_some_and(|e| e.kind == Kind::Text && e.text.trim().is_empty()) {
                self.doc.elements.remove(i);
                self.selected = None;
            }
            self.touched();
        }
    }

    /// Draw the whole editor into `ui` and return requests for the host.
    pub fn ui(&mut self, ui: &mut Ui) -> Vec<EditorRequest> {
        let mut requests = Vec::new();
        self.handle_shortcuts(ui, &mut requests);

        egui::Panel::top("rustshot_toolbar").show(ui, |ui| {
            ui.add_space(4.0);
            self.toolbar(ui, &mut requests);
            ui.add_space(2.0);
        });
        egui::Panel::top("rustshot_properties").show(ui, |ui| {
            ui.add_space(2.0);
            self.properties(ui);
            ui.add_space(2.0);
        });
        egui::Panel::bottom("rustshot_status").show(ui, |ui| self.status_bar(ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(ui.visuals().extreme_bg_color))
            .show(ui, |ui| self.canvas(ui));
        requests
    }

    fn handle_shortcuts(&mut self, ui: &mut Ui, requests: &mut Vec<EditorRequest>) {
        let typing = self.editing_text.is_some() || ui.ctx().egui_wants_keyboard_input();
        let cmd = Modifiers::COMMAND;
        let cmd_shift = Modifiers::COMMAND | Modifiers::SHIFT;
        ui.input_mut(|i| {
            if i.consume_key(cmd_shift, Key::S) {
                requests.push(EditorRequest::SaveAs);
            } else if i.consume_key(cmd, Key::S) {
                requests.push(EditorRequest::Save);
            }
            if i.consume_key(cmd, Key::W) {
                requests.push(EditorRequest::Close);
            }
        });
        if typing {
            return;
        }
        let mut copy = false;
        ui.input_mut(|i| {
            // egui turns Cmd/Ctrl+C into an Event::Copy rather than a key press.
            i.events.retain(|e| {
                if matches!(e, egui::Event::Copy) {
                    copy = true;
                    false
                } else {
                    true
                }
            });
        });
        if copy || ui.input_mut(|i| i.consume_key(cmd_shift, Key::C)) {
            requests.push(EditorRequest::Copy);
        }
        if ui.input_mut(|i| i.consume_key(cmd_shift, Key::Z) || i.consume_key(cmd, Key::Y)) {
            self.redo();
        } else if ui.input_mut(|i| i.consume_key(cmd, Key::Z)) {
            self.undo();
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Delete) || i.consume_key(Modifiers::NONE, Key::Backspace)) {
            self.delete_selected();
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) && self.crop_rect.is_some() {
            self.apply_crop();
        }
        if ui.input_mut(|i| i.consume_key(cmd, Key::Plus) || i.consume_key(cmd, Key::Equals)) {
            self.zoom = Some((self.zoom.unwrap_or(1.0) * 1.25).min(16.0));
        }
        if ui.input_mut(|i| i.consume_key(cmd, Key::Minus)) {
            self.zoom = Some((self.zoom.unwrap_or(1.0) / 1.25).max(0.05));
        }
        if ui.input_mut(|i| i.consume_key(cmd, Key::Num0)) {
            self.zoom = None;
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            if self.crop_rect.is_some() {
                self.crop_rect = None;
            } else if self.selected.is_some() {
                self.selected = None;
            } else {
                self.set_tool(Tool::Select);
            }
        }
        for tool in Tool::ALL {
            if tool != Tool::Select && ui.input_mut(|i| i.consume_key(Modifiers::NONE, tool.key())) {
                self.set_tool(tool);
            }
        }
    }

    fn toolbar(&mut self, ui: &mut Ui, requests: &mut Vec<EditorRequest>) {
        use egui_phosphor::regular as icon;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            if icon_button(ui, icon::FLOPPY_DISK, true, &format!("Save to the output folder ({})", keys("Mod+S"))).clicked() {
                requests.push(EditorRequest::Save);
            }
            if icon_button(ui, icon::FLOPPY_DISK_BACK, true, &format!("Save as… ({})", keys("Mod+Shift+S"))).clicked() {
                requests.push(EditorRequest::SaveAs);
            }
            if icon_button(ui, icon::COPY, true, &format!("Copy image to clipboard ({})", keys("Mod+C"))).clicked() {
                requests.push(EditorRequest::Copy);
            }
            ui.separator();
            if icon_button(ui, icon::ARROW_COUNTER_CLOCKWISE, self.history.can_undo(), &format!("Undo ({})", keys("Mod+Z"))).clicked() {
                self.undo();
            }
            if icon_button(ui, icon::ARROW_CLOCKWISE, self.history.can_redo(), &format!("Redo ({})", keys("Mod+Shift+Z"))).clicked() {
                self.redo();
            }
            if icon_button(ui, icon::TRASH, self.selected.is_some(), "Delete selected (Delete)").clicked() {
                self.delete_selected();
            }
            ui.menu_button(egui::RichText::new(icon::MAGIC_WAND).size(ICON_SIZE), |ui| self.effects_menu(ui))
                .response
                .on_hover_text("Effects: shadow, torn edges, border, greyscale, rotate, resize");
            ui.separator();
            for tool in Tool::ALL {
                let key = if tool == Tool::Select { "Esc".to_string() } else { format!("{:?}", tool.key()) };
                let button = egui::Button::selectable(self.tool == tool, egui::RichText::new(tool.icon()).size(ICON_SIZE))
                    .min_size(egui::vec2(30.0, 28.0));
                if ui.add(button).on_hover_text(format!("{} ({key})", tool.label())).clicked() {
                    self.set_tool(tool);
                }
            }
            if self.crop_rect.is_some() {
                ui.separator();
                if ui.button(format!("{} Apply crop", icon::CHECK)).on_hover_text("Enter").clicked() {
                    self.apply_crop();
                }
                if ui.button(format!("{} Cancel", icon::X)).on_hover_text("Esc").clicked() {
                    self.crop_rect = None;
                }
            }
        });
    }

    /// Kind whose properties are shown: the selection, else the current tool.
    fn active_kind(&self) -> Option<Kind> {
        self.selected
            .and_then(|i| self.doc.elements.get(i))
            .map(|e| e.kind)
            .or_else(|| self.tool.kind())
    }

    fn properties(&mut self, ui: &mut Ui) {
        let kind = self.active_kind();
        let before = self.style;
        ui.horizontal_wrapped(|ui| {
            let Some(kind) = kind else {
                ui.label(match self.tool {
                    Tool::Crop => "Drag to choose the area to keep, then press Enter.",
                    _ => "Click an annotation to select it; drag to move, drag corners to resize.",
                });
                return;
            };
            match kind {
                Kind::Highlight => {
                    ui.label("Style");
                    for mode in HighlightMode::ALL {
                        ui.selectable_value(&mut self.style.highlight_mode, mode, mode.label());
                    }
                    if self.style.highlight_mode == HighlightMode::Text {
                        ui.label("Colour");
                        egui::color_picker::color_edit_button_srgba(ui, &mut self.style.highlight, egui::color_picker::Alpha::Opaque);
                    }
                }
                Kind::Counter => {
                    ui.label("Colour");
                    egui::color_picker::color_edit_button_srgba(ui, &mut self.style.stroke, egui::color_picker::Alpha::Opaque);
                    ui.label("Size");
                    ui.add(egui::DragValue::new(&mut self.style.font_size).range(8.0..=120.0).speed(0.5));
                    ui.checkbox(&mut self.style.shadow, "Shadow");
                }
                Kind::Obfuscate => {
                    ui.label("Mode");
                    ui.selectable_value(&mut self.style.obfuscate, ObfuscateMode::Pixelate, "Pixelate");
                    ui.selectable_value(&mut self.style.obfuscate, ObfuscateMode::Blur, "Blur");
                    ui.label(if self.style.obfuscate == ObfuscateMode::Pixelate { "Pixel size" } else { "Blur radius" });
                    ui.add(egui::Slider::new(&mut self.style.strength, 2.0..=50.0).integer());
                }
                _ => {
                    ui.label(match kind {
                        Kind::Text => "Text colour",
                        Kind::Speech => "Line and text colour",
                        _ => "Line colour",
                    });
                    egui::color_picker::color_edit_button_srgba(ui, &mut self.style.stroke, egui::color_picker::Alpha::BlendOrAdditive);
                    if kind.has_text() {
                        ui.label("Text size");
                        ui.add(egui::DragValue::new(&mut self.style.font_size).range(6.0..=200.0).speed(0.5));
                    }
                    if kind != Kind::Text {
                        ui.label("Thickness");
                        ui.add(egui::DragValue::new(&mut self.style.width).range(0.0..=40.0).speed(0.2));
                    }
                    if kind.has_fill() {
                        ui.label(if kind == Kind::Text { "Background" } else { "Fill" });
                        egui::color_picker::color_edit_button_srgba(ui, &mut self.style.fill, egui::color_picker::Alpha::BlendOrAdditive);
                        if self.style.fill.a() > 0 && ui.small_button("No fill").clicked() {
                            self.style.fill = Color32::TRANSPARENT;
                        }
                    }
                    if kind.has_shadow() {
                        ui.checkbox(&mut self.style.shadow, "Shadow");
                    }
                }
            }
        });

        // Apply edits to the selected element, as one undo step per interaction.
        if self.style != before {
            if let Some(i) = self.selected.filter(|&i| i < self.doc.elements.len()) {
                if !self.prop_editing {
                    self.checkpoint();
                }
                self.doc.elements[i].style = self.style;
                self.touched();
            }
            self.prop_editing = true;
        } else {
            self.prop_editing = false;
        }
    }

    fn status_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let (w, h) = self.image_size();
            ui.label(format!("{w} × {h} px"));
            ui.separator();
            let zoom_label = match self.zoom {
                Some(z) => format!("{:.0}%", z * 100.0),
                None => "Fit".into(),
            };
            ui.menu_button(format!("Zoom: {zoom_label}"), |ui| {
                if ui.button(format!("Fit to window ({})", keys("Mod+0"))).clicked() {
                    self.zoom = None;
                }
                for z in [0.25, 0.5, 1.0, 2.0, 4.0] {
                    if ui.button(format!("{:.0}%", z * 100.0)).clicked() {
                        self.zoom = Some(z);
                    }
                }
            });
            if !self.status.is_empty() {
                ui.separator();
                ui.label(&self.status);
            }
        });
    }

    fn update_texture(&mut self, ui: &Ui) {
        if self.texture.is_some() && self.texture_rev == self.revision {
            return;
        }
        let img = raster::composite(&self.doc.base, &self.doc.elements);
        let color = egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw());
        match &mut self.texture {
            Some(t) => t.set(color, TextureOptions::LINEAR),
            None => self.texture = Some(ui.ctx().load_texture("rustshot-canvas", color, TextureOptions::LINEAR)),
        }
        self.texture_rev = self.revision;
    }

    fn canvas(&mut self, ui: &mut Ui) {
        self.update_texture(ui);
        // Keep text boxes, counters and speech bubbles sized to their content.
        for e in &mut self.doc.elements {
            e.fit_content();
        }

        let ppp = ui.ctx().pixels_per_point();
        let img_size = self.doc.size();
        let avail = ui.available_size();
        let fit = ((avail.x - 16.0) / img_size.x).min((avail.y - 16.0) / img_size.y).min(1.0 / ppp).max(0.01);
        let scale = self.zoom.map_or(fit, |z| z / ppp);

        // Ctrl/Cmd + scroll wheel zooms.
        let zoom_delta = ui.input(|i| if i.modifiers.command { i.zoom_delta() } else { 1.0 });
        if zoom_delta != 1.0 {
            self.zoom = Some((scale * ppp * zoom_delta).clamp(0.05, 16.0));
        }

        egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
            let size = (img_size * scale).max(ui.available_size());
            let (resp, painter) = ui.allocate_painter(size, Sense::click_and_drag());
            let offset = ((resp.rect.size() - img_size * scale) / 2.0).max(Vec2::ZERO);
            let view = View { origin: (resp.rect.min + offset).round(), scale };
            let image_rect = view.rect(Rect::from_min_size(Pos2::ZERO, img_size));

            if let Some(tex) = &self.texture {
                painter.rect_filled(image_rect.expand(1.0), 0.0, Color32::from_gray(40));
                painter.image(tex.id(), image_rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
            }

            self.interact(ui, &resp, view);

            let painter = painter.with_clip_rect(image_rect.intersect(resp.rect));
            for (i, e) in self.doc.elements.iter().enumerate() {
                let number = (e.kind == Kind::Counter).then(|| self.doc.counter_number(i));
                let editing = self.editing_text == Some(i);
                if e.kind == Kind::Text && editing {
                    continue;
                }
                paint_element(&painter, e, view, Paint { number, drafting: false, show_text: !editing });
            }
            if let Drag::Create(e) = &self.drag {
                paint_element(&painter, e, view, Paint { number: None, drafting: true, show_text: true });
            }
            let painter = painter.with_clip_rect(resp.rect);
            if let Some(e) = self.selected.and_then(|i| self.doc.elements.get(i)) {
                paint_selection(&painter, e, view);
            }
            if let Some(r) = self.crop_rect {
                paint_crop(&painter, view.rect(r), image_rect);
            }
            self.text_editor(ui, view);

            if self.tool != Tool::Select
                && resp.hovered() {
                    ui.ctx().set_cursor_icon(if self.tool == Tool::Text { egui::CursorIcon::Text } else { egui::CursorIcon::Crosshair });
                }
        });
    }

    fn interact(&mut self, ui: &Ui, resp: &egui::Response, view: View) {
        let shift = ui.input(|i| i.modifiers.shift);
        let tol = 5.0 / view.scale;

        if resp.drag_started_by(egui::PointerButton::Primary) {
            let Some(start) = ui.input(|i| i.pointer.press_origin()) else { return };
            let p = view.to_image(start);
            self.finish_text_edit();
            self.drag = match self.tool {
                Tool::Select => self.begin_select_drag(p, tol),
                Tool::Crop => {
                    self.crop_rect = None;
                    Drag::Crop { start: p }
                }
                Tool::Text | Tool::Counter => Drag::None,
                tool => {
                    let mut e = Element::new(tool.kind().unwrap(), p, self.style);
                    if e.kind == Kind::Freehand {
                        e.points.push(p);
                    }
                    Drag::Create(e)
                }
            };
        }

        if resp.dragged_by(egui::PointerButton::Primary)
            && let Some(pos) = resp.interact_pointer_pos() {
                let p = view.to_image(pos);
                match &mut self.drag {
                    Drag::Create(e) => {
                        if e.kind == Kind::Freehand {
                            if e.points.last().is_none_or(|l| l.distance(p) > 0.75) {
                                e.points.push(p);
                            }
                        } else {
                            e.b = if shift { constrain(e.kind, e.a, p) } else { p };
                        }
                    }
                    Drag::Move { idx, last, .. } => {
                        let d = p - *last;
                        *last = p;
                        if let Some(e) = self.doc.elements.get_mut(*idx) {
                            e.translate(d);
                        }
                    }
                    Drag::Resize { idx, handle, .. } => {
                        if let Some(e) = self.doc.elements.get_mut(*idx) {
                            e.set_handle(*handle, p);
                        }
                    }
                    Drag::Crop { start } => {
                        let r = Rect::from_two_pos(*start, p);
                        self.crop_rect = Some(r.intersect(Rect::from_min_size(Pos2::ZERO, self.doc.size())));
                    }
                    Drag::None => {}
                }
            }

        if resp.drag_stopped() {
            match std::mem::replace(&mut self.drag, Drag::None) {
                Drag::Create(mut e) => {
                    if let Some(last) = e.points.last() {
                        e.b = *last;
                    }
                    if e.kind == Kind::Speech {
                        self.add_speech(e);
                    } else if !e.is_degenerate() {
                        self.checkpoint();
                        self.doc.elements.push(e);
                        self.selected = Some(self.doc.elements.len() - 1);
                        self.touched();
                    }
                }
                Drag::Move { before, .. } | Drag::Resize { before, .. } => {
                    if before.elements != self.doc.elements {
                        self.history.push(&before);
                        self.touched();
                    }
                }
                Drag::Crop { .. } | Drag::None => {}
            }
        }

        if resp.clicked() {
            let Some(pos) = resp.interact_pointer_pos() else { return };
            let p = view.to_image(pos);
            self.finish_text_edit();
            match self.tool {
                Tool::Select => {
                    self.selected = self.doc.hit(p, tol);
                    if let Some(e) = self.selected.and_then(|i| self.doc.elements.get(i)) {
                        self.style = e.style;
                    }
                }
                Tool::Speech => match self.doc.hit(p, tol).filter(|&i| self.doc.elements[i].kind == Kind::Speech) {
                    Some(i) => {
                        self.checkpoint();
                        self.start_text_edit(i);
                    }
                    None => {
                        let mut e = Element::new(Kind::Speech, p, self.style);
                        e.b = p + Vec2::new(200.0, 70.0);
                        self.add_speech(e);
                    }
                },
                Tool::Counter => {
                    self.checkpoint();
                    self.doc.elements.push(Element::counter(p, self.style));
                    self.selected = Some(self.doc.elements.len() - 1);
                    self.touched();
                }
                Tool::Text => {
                    let existing = self.doc.hit(p, tol).filter(|&i| self.doc.elements[i].kind.has_text());
                    if let Some(i) = existing {
                        self.checkpoint();
                        self.start_text_edit(i);
                    } else {
                        self.checkpoint();
                        self.doc.elements.push(Element::new(Kind::Text, p, self.style));
                        self.start_text_edit(self.doc.elements.len() - 1);
                    }
                }
                _ => {}
            }
        }

        if resp.double_clicked() && self.tool == Tool::Select
            && let Some(pos) = resp.interact_pointer_pos() {
                let p = view.to_image(pos);
                if let Some(i) = self.doc.hit(p, tol).filter(|&i| self.doc.elements[i].kind.has_text()) {
                    self.checkpoint();
                    self.start_text_edit(i);
                }
            }
    }

    fn begin_select_drag(&mut self, p: Pos2, tol: f32) -> Drag {
        if let Some(idx) = self.selected.filter(|&i| i < self.doc.elements.len()) {
            let e = &self.doc.elements[idx];
            if let Some(handle) = e.handles().into_iter().find(|&h| e.handle_pos(h).distance(p) <= tol * 1.5) {
                return Drag::Resize { idx, handle, before: self.doc.clone() };
            }
        }
        match self.doc.hit(p, tol) {
            Some(idx) => {
                self.selected = Some(idx);
                self.style = self.doc.elements[idx].style;
                Drag::Move { idx, before: self.doc.clone(), last: p }
            }
            None => {
                self.selected = None;
                Drag::None
            }
        }
    }

    /// In-place text editing for the text element being edited.
    fn text_editor(&mut self, ui: &mut Ui, view: View) {
        let Some(idx) = self.editing_text else { return };
        let Some(e) = self.doc.elements.get_mut(idx) else {
            self.editing_text = None;
            return;
        };
        let size = measure_text(&e.text, e.style.font_size).max(Vec2::new(e.style.font_size * 4.0, e.style.font_size * 1.3));
        let origin = if e.kind == Kind::Speech { e.bounds().min + Vec2::splat(SPEECH_PADDING) } else { e.a };
        let rect = Rect::from_min_size(view.to_screen(origin), size * view.scale + Vec2::new(40.0, 4.0));
        if e.kind == Kind::Text && e.style.fill.a() > 0 {
            ui.painter().rect_filled(view.rect(e.bounds().expand(TEXT_PADDING)), 0.0, e.style.fill);
        }
        ui.painter().rect_stroke(rect.expand(2.0), 2.0, Stroke::new(1.0, Color32::from_rgb(0, 120, 215)), StrokeKind::Outside);
        let edit = egui::TextEdit::multiline(&mut e.text)
            .font(FontId::proportional(e.style.font_size * view.scale))
            .text_color(e.style.stroke)
            .frame(egui::Frame::NONE)
            .margin(egui::Margin::ZERO)
            .desired_width(rect.width())
            .desired_rows(1);
        let resp = ui.put(rect, edit);
        if self.focus_text {
            resp.request_focus();
            self.focus_text = false;
        } else if resp.lost_focus() {
            self.finish_text_edit();
        }
    }
}

const ICON_SIZE: f32 = 18.0;

/// Spell out a shortcut for this platform: `Mod` is Cmd on macOS, Ctrl elsewhere.
/// (egui's bundled fonts have no ⇧ or ⌥ glyphs, so symbols would show as boxes.)
fn keys(shortcut: &str) -> String {
    shortcut.replace("Mod", if cfg!(target_os = "macos") { "Cmd" } else { "Ctrl" })
}

fn icon_button(ui: &mut Ui, glyph: &str, enabled: bool, tooltip: &str) -> egui::Response {
    let button = egui::Button::new(egui::RichText::new(glyph).size(ICON_SIZE)).min_size(egui::vec2(30.0, 28.0));
    ui.add_enabled(enabled, button).on_hover_text(tooltip).on_disabled_hover_text(tooltip)
}

/// Make the Phosphor icon font available. Call once at startup.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    ctx.set_fonts(fonts);
}

/// Shift-drag: squares/circles for boxes, 45° steps for lines.
fn constrain(kind: Kind, a: Pos2, p: Pos2) -> Pos2 {
    let d = p - a;
    if kind.is_line() {
        let angle = d.y.atan2(d.x);
        let snapped = (angle / std::f32::consts::FRAC_PI_4).round() * std::f32::consts::FRAC_PI_4;
        a + Vec2::angled(snapped) * d.length()
    } else {
        let s = d.x.abs().max(d.y.abs());
        a + Vec2::new(s.copysign(d.x), s.copysign(d.y))
    }
}

/// Per-element painting options for the canvas preview.
#[derive(Clone, Copy)]
struct Paint {
    /// Step-counter number.
    number: Option<usize>,
    /// Being drawn right now (raster elements show a preview box).
    drafting: bool,
    /// False while the element's text is being edited in place.
    show_text: bool,
}

const PREVIEW_SHADOW: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 70);

fn paint_element(painter: &egui::Painter, e: &Element, view: View, opts: Paint) {
    if e.style.shadow && e.kind.has_shadow() {
        // A quick offset silhouette; the saved image gets a soft blurred one.
        let shadow_view = View { origin: view.origin + Vec2::splat(4.0 * view.scale), scale: view.scale };
        paint_shapes(painter, e, shadow_view, opts, Some(PREVIEW_SHADOW));
    }
    paint_shapes(painter, e, view, opts, None);
}

/// Draw an element, or its silhouette in `mono` if given.
fn paint_shapes(painter: &egui::Painter, e: &Element, view: View, opts: Paint, mono: Option<Color32>) {
    let s = &e.style;
    let col = |c: Color32| match mono {
        Some(m) if c.a() > 0 => m,
        _ => c,
    };
    let stroke = Stroke::new(s.width * view.scale, col(s.stroke));
    match e.kind {
        Kind::Rect => {
            painter.rect(view.rect(e.bounds()), 0.0, col(s.fill), stroke, StrokeKind::Middle);
        }
        Kind::Ellipse => {
            let r = view.rect(e.bounds());
            if s.fill.a() > 0 {
                painter.add(Shape::ellipse_filled(r.center(), r.size() / 2.0, col(s.fill)));
            }
            painter.add(Shape::ellipse_stroke(r.center(), r.size() / 2.0, stroke));
        }
        Kind::Line => {
            painter.line_segment([view.to_screen(e.a), view.to_screen(e.b)], stroke);
        }
        Kind::Arrow => {
            let (head, shaft_end) = arrow_head(e.a, e.b, s.width);
            painter.line_segment([view.to_screen(e.a), view.to_screen(shaft_end)], stroke);
            painter.add(Shape::convex_polygon(head.iter().map(|p| view.to_screen(*p)).collect(), col(s.stroke), Stroke::NONE));
        }
        Kind::Freehand => {
            painter.add(Shape::line(e.points.iter().map(|p| view.to_screen(*p)).collect(), stroke));
        }
        Kind::Text => {
            if s.fill.a() > 0 {
                painter.rect_filled(view.rect(e.bounds().expand(TEXT_PADDING)), 0.0, col(s.fill));
            }
            if opts.show_text {
                painter.text(view.to_screen(e.a), Align2::LEFT_TOP, &e.text, FontId::proportional(s.font_size * view.scale), col(s.stroke));
            }
        }
        Kind::Speech => {
            let r = e.bounds();
            let screen = view.rect(r);
            let radius = egui::CornerRadius::same((speech_radius(r) * view.scale).clamp(0.0, 255.0) as u8);
            let base = speech_tail_base(r, e.tail);
            let tail: Vec<Pos2> = [base[0], e.tail, base[1]].iter().map(|p| view.to_screen(*p)).collect();
            let has_tail = base[0] != base[1];
            if s.fill.a() > 0 {
                // Same trick as the exporter: double-width outline, then fill over it.
                let thick = Stroke::new(s.width * 2.0 * view.scale, col(s.stroke));
                if s.width > 0.0 {
                    painter.rect_stroke(screen, radius, thick, StrokeKind::Middle);
                    if has_tail {
                        painter.add(Shape::closed_line(tail.clone(), thick));
                    }
                }
                painter.rect_filled(screen, radius, col(s.fill));
                if has_tail {
                    painter.add(Shape::convex_polygon(tail, col(s.fill), Stroke::NONE));
                }
            } else {
                painter.rect_stroke(screen, radius, stroke, StrokeKind::Middle);
                if has_tail {
                    painter.add(Shape::closed_line(tail, stroke));
                }
            }
            if opts.show_text && mono.is_none() {
                let origin = view.to_screen(r.min + Vec2::splat(SPEECH_PADDING));
                painter.text(origin, Align2::LEFT_TOP, &e.text, FontId::proportional(s.font_size * view.scale), s.stroke);
            }
        }
        Kind::Counter => {
            let r = view.rect(e.bounds());
            painter.circle_filled(r.center(), r.width() / 2.0, col(s.stroke));
            if mono.is_none() {
                let label = opts.number.unwrap_or(1).to_string();
                painter.text(r.center(), Align2::CENTER_CENTER, label, FontId::proportional(s.font_size * view.scale), contrasting_text(s.stroke));
            }
        }
        Kind::Highlight | Kind::Obfuscate => {
            // The effect itself is baked into the canvas texture once the
            // element is committed; while drawing, show a preview box.
            if opts.drafting && mono.is_none() {
                let r = view.rect(e.bounds());
                let fill = if e.kind == Kind::Highlight && s.highlight_mode == HighlightMode::Text {
                    s.highlight.gamma_multiply(0.45)
                } else {
                    Color32::from_black_alpha(90)
                };
                painter.rect(r, 0.0, fill, Stroke::new(1.0, Color32::WHITE), StrokeKind::Outside);
            }
        }
    }
}

fn paint_selection(painter: &egui::Painter, e: &Element, view: View) {
    let r = view.rect(e.bounds()).expand(3.0 + e.style.width * view.scale / 2.0);
    let accent = Color32::from_rgb(0, 120, 215);
    let dashes = Shape::dashed_line(
        &[r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()],
        Stroke::new(1.0, accent),
        4.0,
        3.0,
    );
    painter.extend(dashes);
    for h in e.handles() {
        let c = view.to_screen(e.handle_pos(h));
        painter.rect(Rect::from_center_size(c, Vec2::splat(8.0)), 1.0, Color32::WHITE, Stroke::new(1.0, accent), StrokeKind::Middle);
    }
}

fn paint_crop(painter: &egui::Painter, crop: Rect, image: Rect) {
    let shade = Color32::from_black_alpha(140);
    let outer = [
        Rect::from_min_max(image.min, Pos2::new(image.max.x, crop.min.y)),
        Rect::from_min_max(Pos2::new(image.min.x, crop.max.y), image.max),
        Rect::from_min_max(Pos2::new(image.min.x, crop.min.y), Pos2::new(crop.min.x, crop.max.y)),
        Rect::from_min_max(Pos2::new(crop.max.x, crop.min.y), Pos2::new(image.max.x, crop.max.y)),
    ];
    for r in outer {
        if r.is_positive() {
            painter.rect_filled(r, 0.0, shade);
        }
    }
    painter.rect_stroke(crop, 0.0, Stroke::new(1.5, Color32::WHITE), StrokeKind::Middle);
}
