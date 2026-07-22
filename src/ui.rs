use crate::app::InjectorApp;
use crate::models::{Dll, Toast, ToastLevel};
use eframe::egui::{
    self, Color32, CursorIcon, Frame, Id, Margin, Pos2, Rect, RichText, Sense, Stroke, TextEdit,
    TextureHandle, UiBuilder, Vec2, Visuals,
};
use std::path::Path;

const SURFACE: Color32 = Color32::from_rgb(28, 28, 28);
const SURFACE_RAISED: Color32 = Color32::from_rgb(36, 36, 36);
const RULE: Color32 = Color32::from_rgb(58, 58, 58);
const PANEL_MIN: f32 = 140.0;
const HANDLE_HIT: f32 = 8.0;
const INSET: f32 = 12.0;

pub(crate) fn show(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    ui.painter().rect_filled(ui.max_rect(), 0.0, SURFACE);
    draw_status_bar(ui, app);
    let changed = draw_split_body(ui, app);
    draw_toasts(ui.ctx(), app);
    changed
}

pub(crate) fn apply_theme(ctx: &egui::Context) {
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

fn filled_scroll<R>(ui: &mut egui::Ui, id: &'static str, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let size = ui.available_size();
    egui::ScrollArea::vertical()
        .id_salt(id)
        .auto_shrink([false, false])
        .max_width(size.x)
        .max_height(size.y)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            add(ui)
        })
        .inner
}

fn rule_separator(ui: &mut egui::Ui) {
    let y = ui.cursor().min.y + 0.5;
    ui.painter()
        .hline(ui.max_rect().x_range(), y, Stroke::new(1.0, RULE));
    ui.add_space(1.0);
}

fn draw_split_body(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    let full = ui.available_rect_before_wrap();
    ui.allocate_rect(full, Sense::hover());

    let left_w = resolved_left_width(full.width(), app.config.split_ratio);
    let left = Rect::from_min_size(full.min, Vec2::new(left_w, full.height()));
    let right = Rect::from_min_max(Pos2::new(full.min.x + left_w, full.min.y), full.max);

    let mut changed = side_pane(ui, left, |ui| draw_process_panel(ui, app));
    changed |= side_pane(ui, right, |ui| draw_dll_panel(ui, app));
    ui.painter()
        .vline(left.max.x, full.y_range(), Stroke::new(1.0, RULE));
    changed |= drag_split_handle(ui, app, full, left_w);
    changed
}

fn side_pane(ui: &mut egui::Ui, rect: Rect, add: impl FnOnce(&mut egui::Ui) -> bool) -> bool {
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(rect.shrink(INSET))
            .layout(egui::Layout::top_down_justified(egui::Align::Min)),
        |ui| {
            ui.set_clip_rect(rect);
            ui.set_min_size(ui.available_size());
            add(ui)
        },
    )
    .inner
}

fn resolved_left_width(total: f32, ratio: f32) -> f32 {
    let max = (total - PANEL_MIN).max(PANEL_MIN);
    (total * ratio).clamp(PANEL_MIN, max)
}

fn drag_split_handle(ui: &mut egui::Ui, app: &mut InjectorApp, full: Rect, left_w: f32) -> bool {
    let handle = Rect::from_center_size(
        Pos2::new(full.min.x + left_w, full.center().y),
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
        let count = app.selected_dlls().count();
        ui.label(
            RichText::new(format!("DLLs selected: {count}"))
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
    app.toasts.retain(Toast::is_alive);
    if app.toasts.is_empty() {
        return;
    }

    const WIDTH: f32 = 220.0;
    let screen = ctx.content_rect();
    let pos = Pos2::new(screen.center().x - WIDTH * 0.5, screen.top() + 12.0);

    egui::Area::new("toasts".into())
        .fixed_pos(pos)
        .order(egui::Order::Foreground)
        .interactable(false)
        .show(ctx, |ui| {
            ui.set_min_width(WIDTH);
            ui.set_max_width(WIDTH);
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
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new(format!("{prefix}: {}", toast.message)).color(color));
        });
}

fn draw_process_panel(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    ui.label(RichText::new("Processes").size(15.0).strong());
    ui.add_space(8.0);
    let search = ui.add(
        TextEdit::singleline(&mut app.process_search)
            .hint_text("Filter by name…")
            .desired_width(f32::INFINITY)
            .background_color(Color32::from_rgb(16, 16, 16)),
    );
    if search.changed() {
        app.sync_process_search_lower();
    }
    ui.add_space(10.0);
    filled_scroll(ui, "process_list", |ui| draw_process_list(ui, app))
}

fn draw_process_list(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    if app.is_loading_processes && app.processes.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(24.0);
            ui.add(egui::Spinner::new());
        });
        return empty_process_space_menu(ui, app);
    }

    let mut selection = None;
    let mut menu_action = None;
    draw_visible_process_rows(ui, app, &mut selection, &mut menu_action);
    let empty_changed = empty_process_space_menu(ui, app);
    apply_process_menu_action(app, menu_action) | apply_process_selection(app, selection) | empty_changed
}

fn draw_visible_process_rows(
    ui: &mut egui::Ui,
    app: &InjectorApp,
    selection: &mut Option<u32>,
    menu_action: &mut Option<ProcessMenuAction>,
) {
    let search = app.process_search_lower.as_str();
    for process in &app.processes {
        if app.config.is_blocked(&process.name) {
            continue;
        }
        if !search.is_empty() && !ascii_contains_ignore_case(&process.name, search) {
            continue;
        }
        let favorite = app.config.is_favorite(&process.name);
        let selected = app.selected_process == Some(process.pid);
        let response = draw_process_row(
            ui,
            &process.name,
            process.pid,
            app.icon_cache.get(&process.pid),
            selected,
            favorite,
        );
        if response.clicked() {
            *selection = Some(process.pid);
        }
        process_row_menu(&response, process.pid, favorite, menu_action);
    }
}

fn empty_process_space_menu(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    let height = ui.available_height().max(28.0);
    let (_, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::click());
    let mut unblock = None;
    response.context_menu(|ui| {
        blocked_context_menu(ui, &app.config.blocked, &mut unblock);
    });
    let Some(index) = unblock else {
        return false;
    };
    app.config.unblock_at(index);
    true
}

fn blocked_context_menu(ui: &mut egui::Ui, blocked: &[String], unblock: &mut Option<usize>) {
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
    if blocked.is_empty() {
        ui.add_enabled(false, egui::Button::new("Blocked (empty)"));
        return;
    }
    ui.menu_button("Blocked", |ui| {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        egui::ScrollArea::vertical()
            .max_height(220.0)
            .show(ui, |ui| {
                for (index, name) in blocked.iter().enumerate() {
                    if ui.button(name).clicked() {
                        *unblock = Some(index);
                        ui.close();
                    }
                }
            });
    });
}

fn ascii_contains_ignore_case(haystack: &str, needle_lower: &str) -> bool {
    if needle_lower.is_empty() {
        return true;
    }
    haystack
        .as_bytes()
        .windows(needle_lower.len())
        .any(|window| window.eq_ignore_ascii_case(needle_lower.as_bytes()))
}

fn process_row_menu(
    response: &egui::Response,
    pid: u32,
    favorite: bool,
    menu_action: &mut Option<ProcessMenuAction>,
) {
    response.context_menu(|ui| {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        let favorite_label = if favorite { "Unfavorite" } else { "Favorite" };
        if ui.button(favorite_label).clicked() {
            *menu_action = Some(ProcessMenuAction::ToggleFavorite(pid));
            ui.close();
        }
        if ui.button("Block from list").clicked() {
            *menu_action = Some(ProcessMenuAction::Block(pid));
            ui.close();
        }
    });
}

enum ProcessMenuAction {
    ToggleFavorite(u32),
    Block(u32),
}

fn process_name_by_pid(app: &InjectorApp, pid: u32) -> Option<String> {
    app.processes
        .iter()
        .find(|process| process.pid == pid)
        .map(|process| process.name.clone())
}

fn apply_process_menu_action(app: &mut InjectorApp, action: Option<ProcessMenuAction>) -> bool {
    let Some(action) = action else {
        return false;
    };
    let (pid, block) = match action {
        ProcessMenuAction::ToggleFavorite(pid) => (pid, false),
        ProcessMenuAction::Block(pid) => (pid, true),
    };
    let Some(name) = process_name_by_pid(app, pid) else {
        return false;
    };
    if block {
        if app.selected_process == Some(pid) {
            app.selected_process = None;
        }
        app.config.block_process(&name);
    } else {
        app.config.toggle_favorite(&name);
        app.order_processes_by_favorite();
    }
    true
}

fn apply_process_selection(app: &mut InjectorApp, selection: Option<u32>) -> bool {
    let Some(pid) = selection else {
        return false;
    };
    if app.selected_process == Some(pid) {
        return false;
    }
    let Some(name) = process_name_by_pid(app, pid) else {
        return false;
    };
    app.selected_process = Some(pid);
    app.config.last_selected_app = Some(name);
    true
}

fn draw_process_row(
    ui: &mut egui::Ui,
    name: &str,
    pid: u32,
    texture: Option<&TextureHandle>,
    selected: bool,
    favorite: bool,
) -> egui::Response {
    let height = ui.spacing().interact_size.y.max(22.0);
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::click());
    paint_row_bg(ui, rect, selected, response.hovered() || response.context_menu_opened());
    ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
        ui.set_clip_rect(ui.clip_rect().intersect(rect));
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            paint_process_row_contents(ui, name, pid, texture, favorite);
        });
    });
    response
}

fn paint_process_row_contents(
    ui: &mut egui::Ui,
    name: &str,
    pid: u32,
    texture: Option<&TextureHandle>,
    favorite: bool,
) {
    row_icon(ui, texture);
    ui.add_space(6.0);
    if favorite {
        ui.label(RichText::new("★").strong());
        ui.add_space(4.0);
    }
    ui.add(
        egui::Label::new(name)
            .truncate()
            .selectable(false)
            .sense(Sense::hover()),
    );
    ui.add_space(4.0);
    let mut pid_buf = [0u8; 14];
    ui.label(RichText::new(pid_in_parens(pid, &mut pid_buf)).color(Color32::from_gray(160)));
}

fn pid_in_parens(pid: u32, buf: &mut [u8; 14]) -> &str {
    let mut digits = [0u8; 10];
    let mut value = pid;
    let mut count = 0;
    loop {
        digits[count] = b'0' + (value % 10) as u8;
        count += 1;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    buf[0] = b'(';
    for index in 0..count {
        buf[1 + index] = digits[count - 1 - index];
    }
    buf[1 + count] = b')';
    std::str::from_utf8(&buf[..2 + count]).unwrap_or("()")
}

fn paint_row_bg(ui: &egui::Ui, rect: Rect, selected: bool, hovered: bool) {
    let fill = if selected {
        Some(ui.visuals().selection.bg_fill)
    } else if hovered {
        Some(Color32::from_rgba_unmultiplied(255, 255, 255, 10))
    } else {
        None
    };
    if let Some(fill) = fill {
        ui.painter().rect_filled(rect, 4.0, fill);
    }
}

fn row_icon(ui: &mut egui::Ui, texture: Option<&TextureHandle>) {
    match texture {
        Some(texture) => {
            ui.add(egui::Image::new((texture.id(), Vec2::splat(16.0))).sense(Sense::hover()));
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

fn draw_dll_panel(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    let mut changed = false;
    egui::Panel::bottom("dll_footer")
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::NONE.fill(SURFACE).inner_margin(Margin {
            left: 0,
            right: 0,
            top: 10,
            bottom: 4,
        }))
        .show(ui, |ui| changed |= draw_dll_footer(ui, app));
    ui.label(RichText::new("DLLs").size(15.0).strong());
    ui.add_space(8.0);
    changed |= filled_scroll(ui, "dll_list", |ui| {
        draw_dll_list(ui, &mut app.config.dlls, app.default_dll_texture.id())
    });
    changed
}

fn draw_dll_footer(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    rule_separator(ui);
    ui.add_space(10.0);
    let settings = draw_dll_settings(ui, app);
    ui.add_space(10.0);
    settings | draw_dll_actions(ui, app)
}

fn draw_dll_settings(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    ui.horizontal(|ui| {
        let copy = ui
            .checkbox(&mut app.config.copy_dll_on_inject, "Copy on inject")
            .on_hover_text("Keeps the original DLL free for rebuilding.")
            .changed();
        ui.add_space(16.0);
        let random = ui
            .add_enabled(
                app.config.copy_dll_on_inject,
                egui::Checkbox::new(&mut app.config.randomize_dll_name, "Random name"),
            )
            .on_hover_text("Gives the copied DLL a random name.")
            .changed();
        copy || random
    })
    .inner
}

fn draw_dll_actions(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    let selected = app.selected_dlls().count();
    let can_inject = app.selected_process.is_some() && selected > 0 && !app.is_injecting;
    let inject = if app.is_injecting {
        "Injecting…"
    } else {
        "Inject"
    };
    let size = Vec2::new(action_width(ui), 24.0);
    let mut changed = false;
    ui.horizontal(|ui| {
        if action_button(ui, true, "Add DLL", size) {
            changed |= add_dll(app);
        }
        if action_button(ui, can_inject, inject, size) {
            app.start_injection(ui.ctx());
        }
        if action_button(ui, selected > 0, "Remove", size) {
            let n = remove_selected_dlls(app);
            app.add_toast(ToastLevel::Info, format!("Removed {n} DLL(s)."));
            changed = true;
        }
    });
    changed
}

fn action_width(ui: &egui::Ui) -> f32 {
    let gap = ui.spacing().item_spacing.x;
    ((ui.available_width() - gap * 2.0) / 3.0)
        .floor()
        .clamp(49.0, 250.0)
}

fn action_button(ui: &mut egui::Ui, enabled: bool, label: &str, size: Vec2) -> bool {
    ui.add_enabled(enabled, egui::Button::new(label).min_size(size))
        .clicked()
}

fn draw_dll_list(ui: &mut egui::Ui, dlls: &mut [Dll], texture: egui::TextureId) -> bool {
    if dlls.is_empty() {
        ui.colored_label(Color32::DARK_GRAY, "No DLLs added yet.");
        return false;
    }
    let mut changed = false;
    for dll in dlls {
        changed |= draw_dll_row(ui, dll, texture);
    }
    changed
}

fn draw_dll_row(ui: &mut egui::Ui, dll: &mut Dll, texture: egui::TextureId) -> bool {
    ui.horizontal(|ui| {
        ui.add(egui::Image::new((texture, Vec2::splat(16.0))));
        ui.checkbox(&mut dll.selected, dll_label(&dll.path)).changed()
    })
    .inner
}

fn dll_label(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_owned())
}

fn add_dll(app: &mut InjectorApp) -> bool {
    let Some(path) = crate::core::dll_selector::select_dll() else {
        return false;
    };
    if app.config.dlls.iter().any(|dll| dll.path == path) {
        app.add_toast(ToastLevel::Warning, "DLL is already in the list.");
        return false;
    }
    app.config.dlls.push(Dll {
        path,
        selected: false,
    });
    true
}

fn remove_selected_dlls(app: &mut InjectorApp) -> usize {
    let before = app.config.dlls.len();
    app.config.dlls.retain(|dll| !dll.selected);
    before - app.config.dlls.len()
}
