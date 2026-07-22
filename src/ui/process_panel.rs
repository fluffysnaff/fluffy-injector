use crate::app::InjectorApp;
use crate::models::process::ProcessInfo;
use eframe::egui::{self, Color32, RichText, Sense, TextEdit, TextureHandle, UiBuilder, Vec2};

pub(super) fn show(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    ui.label(RichText::new("Processes").size(15.0).strong());
    ui.add_space(8.0);
    ui.add(
        TextEdit::singleline(&mut app.process_search)
            .hint_text("Filter by name…")
            .desired_width(f32::INFINITY)
            .background_color(Color32::from_rgb(16, 16, 16)),
    );
    ui.add_space(10.0);
    let list_size = ui.available_size();
    egui::ScrollArea::vertical()
        .id_salt("process_list")
        .auto_shrink([false, false])
        .max_width(list_size.x)
        .max_height(list_size.y)
        .scroll_bar_visibility(egui::containers::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
        .show(ui, |ui| {
            // Inner width already excludes a solid scrollbar — fill that, don't reclaim the bar.
            ui.set_min_width(ui.available_width());
            draw_process_list(ui, app)
        })
        .inner
}

fn draw_process_list(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    if app.is_loading_processes && app.processes.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(24.0);
            ui.add(egui::Spinner::new());
        });
        return false;
    }

    let search = app.process_search.to_lowercase();
    let mut selection = None;
    for process in &app.processes {
        if !search.is_empty() && !process.name.to_lowercase().contains(&search) {
            continue;
        }
        let selected = app.selected_process == Some(process.pid);
        if draw_process_row(ui, process, app.icon_cache.get(&process.pid), selected) {
            selection = Some((process.pid, process.name.clone()));
        }
    }
    let Some((pid, name)) = selection else {
        return false;
    };
    app.selected_process = Some(pid);
    app.config.last_selected_app = Some(name);
    true
}

fn draw_process_row(
    ui: &mut egui::Ui,
    process: &ProcessInfo,
    texture: Option<&TextureHandle>,
    selected: bool,
) -> bool {
    let height = ui.spacing().interact_size.y.max(22.0);
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::click());
    if selected {
        ui.painter()
            .rect_filled(rect, 4.0, ui.visuals().selection.bg_fill);
    } else if response.hovered() {
        ui.painter().rect_filled(
            rect,
            4.0,
            Color32::from_rgba_unmultiplied(255, 255, 255, 10),
        );
    }
    ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
        // Keep ScrollArea's clip — do not replace it with the row rect or
        // scrolled rows paint up through the header.
        ui.set_clip_rect(ui.clip_rect().intersect(rect));
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            draw_process_icon(ui, texture);
            ui.add_space(6.0);
            // Hover-only so the row's click sense receives the press.
            ui.add(
                egui::Label::new(format!("{}  ({})", process.name, process.pid))
                    .truncate()
                    .selectable(false)
                    .sense(Sense::hover()),
            );
        });
    });
    response.clicked()
}

fn draw_process_icon(ui: &mut egui::Ui, texture: Option<&TextureHandle>) {
    match texture {
        Some(texture) => {
            ui.add(
                egui::Image::new((texture.id(), Vec2::splat(16.0))).sense(Sense::hover()),
            );
        }
        None => {
            ui.add(
                egui::Label::new(RichText::new("·").color(Color32::DARK_GRAY))
                    .selectable(false)
                    .sense(Sense::hover()),
            );
        }
    }
}
