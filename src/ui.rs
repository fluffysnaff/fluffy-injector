mod dll_panel;
mod process_panel;

use crate::app::InjectorApp;
use crate::models::toast::{Toast, ToastLevel};
use eframe::egui::{
    self, Color32, CursorIcon, Frame, Id, Margin, Pos2, Rect, RichText, Sense, Stroke, UiBuilder,
    Vec2, Visuals,
};

pub(crate) const SURFACE: Color32 = Color32::from_rgb(28, 28, 28);
pub(crate) const SURFACE_RAISED: Color32 = Color32::from_rgb(36, 36, 36);
pub(crate) const RULE: Color32 = Color32::from_rgb(58, 58, 58);
const PANEL_MIN: f32 = 140.0;
const HANDLE_HIT: f32 = 8.0;

pub(crate) fn show(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    apply_theme(ui.ctx());
    ui.painter().rect_filled(ui.max_rect(), 0.0, SURFACE);

    let mut changed = false;
    draw_status_bar(ui, app);
    changed |= draw_split_body(ui, app);
    draw_toasts(ui.ctx(), app);
    changed
}

fn draw_split_body(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    let mut changed = false;
    let full = ui.available_rect_before_wrap();
    ui.allocate_rect(full, Sense::hover());

    let left_width = resolved_left_width(full.width(), app.config.split_ratio);
    let left_rect = Rect::from_min_size(full.min, Vec2::new(left_width, full.height()));
    let right_rect = Rect::from_min_max(Pos2::new(full.min.x + left_width, full.min.y), full.max);

    changed |= show_process_side(ui, app, left_rect);
    changed |= show_dll_side(ui, app, right_rect);
    ui.painter()
        .vline(left_rect.max.x, full.y_range(), Stroke::new(1.0, RULE));
    // Handle last so it wins pointer hits over the side panes.
    changed |= drag_split_handle(ui, app, full, left_width);
    changed
}

fn show_process_side(ui: &mut egui::Ui, app: &mut InjectorApp, rect: Rect) -> bool {
    let mut changed = false;
    with_side_pane(ui, rect, |ui| changed |= process_panel::show(ui, app));
    changed
}

fn show_dll_side(ui: &mut egui::Ui, app: &mut InjectorApp, rect: Rect) -> bool {
    let mut changed = false;
    with_side_pane(ui, rect, |ui| changed |= dll_panel::show(ui, app));
    changed
}

fn with_side_pane(ui: &mut egui::Ui, rect: Rect, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down_justified(egui::Align::Min)),
        |ui| {
            ui.set_clip_rect(rect);
            fill_panel(ui);
            Frame::NONE
                .inner_margin(Margin::same(12))
                .show(ui, |ui| {
                    // Claim the full pane so ScrollArea width follows the edge, not content.
                    ui.set_min_size(ui.available_size());
                    add_contents(ui);
                });
        },
    );
}

fn resolved_left_width(total: f32, ratio: f32) -> f32 {
    let max = (total - PANEL_MIN).max(PANEL_MIN);
    (total * ratio).clamp(PANEL_MIN, max)
}

fn drag_split_handle(ui: &mut egui::Ui, app: &mut InjectorApp, full: Rect, left_width: f32) -> bool {
    let handle = Rect::from_center_size(
        Pos2::new(full.min.x + left_width, full.center().y),
        Vec2::new(HANDLE_HIT, full.height()),
    );
    let response = ui.interact(handle, Id::new("split_handle"), Sense::drag());
    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
    }
    if !response.dragged() {
        return false;
    }
    let Some(pointer) = response.interact_pointer_pos() else {
        return false;
    };
    let max = (full.width() - PANEL_MIN).max(PANEL_MIN);
    let width = (pointer.x - full.min.x).clamp(PANEL_MIN, max);
    app.config.split_ratio = width / full.width().max(1.0);
    true
}

pub(crate) fn fill_panel(ui: &mut egui::Ui) {
    ui.painter().rect_filled(ui.max_rect(), 0.0, SURFACE);
}

pub(crate) fn rule_separator(ui: &mut egui::Ui) {
    let stroke = Stroke::new(1.0, RULE);
    let y = ui.cursor().min.y + 0.5;
    let x_range = ui.max_rect().x_range();
    ui.painter().hline(x_range, y, stroke);
    ui.add_space(1.0);
}

fn apply_theme(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();
    visuals.window_fill = SURFACE;
    visuals.panel_fill = SURFACE;
    visuals.extreme_bg_color = Color32::from_rgb(16, 16, 16);
    visuals.text_edit_bg_color = Some(Color32::from_rgb(16, 16, 16));
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, RULE);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(45, 45, 45);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(58, 58, 58);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::from_gray(140));
    visuals.widgets.active.bg_fill = Color32::from_rgb(68, 68, 68);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::from_gray(180));
    visuals.selection.bg_fill = Color32::from_rgb(50, 90, 150);
    ctx.set_visuals(visuals);
    ctx.all_styles_mut(|style| {
        let mut scroll = egui::style::ScrollStyle::solid();
        scroll.bar_width = 10.0;
        style.spacing.scroll = scroll;
    });
}

fn draw_status_bar(ui: &mut egui::Ui, app: &InjectorApp) {
    let frame = Frame::NONE
        .fill(SURFACE_RAISED)
        .stroke(Stroke::new(1.0, RULE))
        .inner_margin(Margin::symmetric(14, 0));
    egui::Panel::top("status_bar")
        .exact_size(44.0)
        .show_separator_line(false)
        .frame(frame)
        .show(ui, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                draw_status_contents(ui, app);
            });
        });
}

fn draw_status_contents(ui: &mut egui::Ui, app: &InjectorApp) {
    ui.label(RichText::new(process_status(app)).size(14.0).strong());
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let dll_count = app.selected_dlls().count();
        ui.label(
            RichText::new(format!("DLLs selected: {dll_count}"))
                .size(14.0)
                .color(Color32::from_rgb(140, 190, 255)),
        );
    });
}

fn process_status(app: &InjectorApp) -> String {
    match app.selected_process_info() {
        Some(process) => format!("{}  ·  PID {}", process.name, process.pid),
        None => app
            .config
            .last_selected_app
            .as_ref()
            .map(|name| format!("Waiting for {name}"))
            .unwrap_or_else(|| "No process selected".to_owned()),
    }
}

fn draw_toasts(ctx: &egui::Context, app: &mut InjectorApp) {
    app.toasts.retain(|toast| toast.is_alive());
    egui::Area::new("toasts".into())
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-12.0, -12.0))
        .show(ctx, |ui| {
            for toast in &app.toasts {
                draw_toast(ui, toast);
                ui.add_space(6.0);
            }
        });
}

fn draw_toast(ui: &mut egui::Ui, toast: &Toast) {
    let (prefix, color) = match toast.level {
        ToastLevel::Info => ("Info", Color32::from_gray(200)),
        ToastLevel::Success => ("OK", Color32::from_rgb(120, 220, 140)),
        ToastLevel::Warning => ("Warn", Color32::from_rgb(240, 200, 80)),
        ToastLevel::Error => ("Error", Color32::from_rgb(240, 100, 100)),
    };
    Frame::popup(ui.style())
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_min_width(220.0);
            ui.label(RichText::new(format!("{prefix}: {}", toast.message)).color(color));
        });
}
