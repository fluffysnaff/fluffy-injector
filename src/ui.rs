use crate::app::InjectorApp;
use crate::models::{Dll, Toast, ToastLevel};
use eframe::egui::{
    self, Color32, CursorIcon, Frame, Id, Margin, Pos2, Rect, RichText, Sense, Stroke, TextEdit,
    TextureHandle, UiBuilder, Vec2, Visuals,
};
use std::path::Path;
use std::process::Command;

const SURFACE: Color32 = Color32::from_rgb(28, 28, 28);
const RAISED: Color32 = Color32::from_rgb(36, 36, 36);
const RULE: Color32 = Color32::from_rgb(58, 58, 58);
const BG: Color32 = Color32::from_rgb(16, 16, 16);
const PANEL_MIN: f32 = 140.0;

pub(crate) fn show(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    if !ui.ctx().input(|i| i.focused) {
        egui::Popup::close_all(ui.ctx());
    }
    ui.painter().rect_filled(ui.max_rect(), 0.0, SURFACE);
    status_bar(ui, app);
    let changed = split_body(ui, app);
    toasts(ui.ctx(), app);
    changed
}

pub(crate) fn apply_theme(ctx: &egui::Context) {
    let mut v = Visuals::dark();
    v.window_fill = SURFACE;
    v.panel_fill = SURFACE;
    v.extreme_bg_color = BG;
    v.text_edit_bg_color.replace(BG);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, RULE);
    v.widgets.inactive.bg_fill = Color32::from_rgb(45, 45, 45);
    v.widgets.hovered.bg_fill = RULE;
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::from_gray(140));
    v.widgets.active.bg_fill = Color32::from_rgb(68, 68, 68);
    v.widgets.active.fg_stroke = Stroke::new(1.0, Color32::from_gray(180));
    v.selection.bg_fill = Color32::from_rgb(50, 90, 150);
    ctx.set_visuals(v);
    ctx.all_styles_mut(|s| {
        let mut scroll = egui::style::ScrollStyle::solid();
        scroll.bar_width = 10.0;
        s.spacing.scroll = scroll;
    });
}

fn scroll<R>(ui: &mut egui::Ui, id: &'static str, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
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

fn split_body(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    let full = ui.available_rect_before_wrap();
    ui.allocate_rect(full, Sense::hover());
    let max = (full.width() - PANEL_MIN).max(PANEL_MIN);
    let left_w = (full.width() * app.config.split_ratio).clamp(PANEL_MIN, max);
    let left = Rect::from_min_size(full.min, Vec2::new(left_w, full.height()));
    let right = Rect::from_min_max(Pos2::new(full.min.x + left_w, full.min.y), full.max);
    let mut changed = pane(ui, left, |ui| process_panel(ui, app));
    changed |= pane(ui, right, |ui| dll_panel(ui, app));
    ui.painter()
        .vline(left.max.x, full.y_range(), Stroke::new(1.0, RULE));
    changed | split_drag(ui, app, full, left_w)
}

fn pane(ui: &mut egui::Ui, rect: Rect, add: impl FnOnce(&mut egui::Ui) -> bool) -> bool {
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(rect.shrink(12.0))
            .layout(egui::Layout::top_down_justified(egui::Align::Min)),
        |ui| {
            ui.set_clip_rect(rect);
            ui.set_min_size(ui.available_size());
            add(ui)
        },
    )
    .inner
}

fn split_drag(ui: &mut egui::Ui, app: &mut InjectorApp, full: Rect, left_w: f32) -> bool {
    let handle = Rect::from_center_size(
        Pos2::new(full.min.x + left_w, full.center().y),
        Vec2::new(8.0, full.height()),
    );
    let r = ui.interact(handle, Id::new("split_handle"), Sense::drag());
    if r.hovered() || r.dragged() {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
    }
    let Some(pos) = r.dragged().then(|| r.interact_pointer_pos()).flatten() else {
        return false;
    };
    let max = (full.width() - PANEL_MIN).max(PANEL_MIN);
    app.config.split_ratio = (pos.x - full.min.x).clamp(PANEL_MIN, max) / full.width().max(1.0);
    true
}

fn status_bar(ui: &mut egui::Ui, app: &InjectorApp) {
    egui::Panel::top("status_bar")
        .exact_size(44.0)
        .show_separator_line(false)
        .frame(
            Frame::NONE
                .fill(RAISED)
                .stroke(Stroke::new(1.0, RULE))
                .inner_margin(Margin::symmetric(14, 0)),
        )
        .show(ui, |ui| status_contents(ui, app));
}

fn status_contents(ui: &mut egui::Ui, app: &InjectorApp) {
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.label(RichText::new(process_status(app)).size(14.0).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let n = app.selected_dlls().count();
            ui.label(
                RichText::new(format!("DLLs selected: {n}"))
                    .size(14.0)
                    .color(Color32::from_rgb(140, 190, 255)),
            );
        });
    });
}

fn process_status(app: &InjectorApp) -> String {
    match app.selected_process_info() {
        Some(p) => format!("{}  ·  PID {}", p.name, p.pid),
        None => app
            .config
            .last_selected_app
            .as_ref()
            .map_or_else(|| "No process selected".into(), |n| format!("Waiting for {n}")),
    }
}

fn toasts(ctx: &egui::Context, app: &mut InjectorApp) {
    app.toasts.retain(Toast::is_alive);
    if app.toasts.is_empty() {
        return;
    }
    let rect = ctx.content_rect();
    egui::Area::new("toasts".into())
        .fixed_pos(Pos2::new(rect.center().x - 110.0, rect.top() + 12.0))
        .order(egui::Order::Foreground)
        .interactable(false)
        .show(ctx, |ui| {
            ui.set_width(220.0);
            for t in &app.toasts {
                toast(ui, t);
                ui.add_space(6.0);
            }
        });
}

fn toast(ui: &mut egui::Ui, t: &Toast) {
    let (prefix, color) = match t.level {
        ToastLevel::Info => ("Info", Color32::from_gray(200)),
        ToastLevel::Success => ("OK", Color32::from_rgb(120, 220, 140)),
        ToastLevel::Warning => ("Warn", Color32::from_rgb(240, 200, 80)),
        ToastLevel::Error => ("Error", Color32::from_rgb(240, 100, 100)),
    };
    Frame::popup(ui.style())
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new(format!("{prefix}: {}", t.message)).color(color));
        });
}

fn process_panel(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    ui.label(RichText::new("Processes").size(15.0).strong());
    ui.add_space(8.0);
    if ui
        .add(
            TextEdit::singleline(&mut app.process_search)
                .hint_text("Filter by name…")
                .desired_width(f32::INFINITY)
                .background_color(BG),
        )
        .changed()
    {
        app.sync_process_search_lower();
    }
    ui.add_space(10.0);
    scroll(ui, "process_list", |ui| process_list(ui, app))
}

fn process_list(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    if app.is_loading_processes && app.processes.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(24.0);
            ui.add(egui::Spinner::new());
        });
        let action = space_menu(ui, |ui| blocked_menu(ui, app));
        return apply(app, ui.ctx(), action);
    }
    let (mut sel, mut menu) = (None, None);
    process_rows(ui, app, &mut sel, &mut menu);
    let empty = space_menu(ui, |ui| blocked_menu(ui, app));
    apply(app, ui.ctx(), menu.or(sel)) | apply(app, ui.ctx(), empty)
}

fn process_rows(
    ui: &mut egui::Ui,
    app: &InjectorApp,
    sel: &mut Option<Action>,
    menu: &mut Option<Action>,
) {
    let search = app.process_search_lower.as_str();
    for p in &app.processes {
        if app.config.is_blocked(&p.name) || !matches_search(&p.name, search) {
            continue;
        }
        let fav = app.config.is_favorite(&p.name);
        let r = process_row(ui, p, app.selected_process == Some(p.pid), fav, app);
        if r.clicked() {
            *sel = Some(Action::Select(p.pid));
        }
        process_menu(&r, p.pid, fav, menu);
    }
}

fn process_menu(r: &egui::Response, pid: u32, fav: bool, menu: &mut Option<Action>) {
    menu_on(r, |ui| {
        if item(ui, true, if fav { "Unfavorite" } else { "Favorite" }) {
            *menu = Some(Action::Fav(pid));
        }
        if item(ui, true, "Block from list") {
            *menu = Some(Action::Block(pid));
        }
    });
}

fn matches_search(name: &str, q: &str) -> bool {
    q.is_empty() || name.as_bytes().windows(q.len()).any(|w| w.eq_ignore_ascii_case(q.as_bytes()))
}

fn blocked_menu(ui: &mut egui::Ui, app: &InjectorApp) -> Option<Action> {
    if app.config.blocked.is_empty() {
        ui.add_enabled(false, egui::Button::new("Blocked (empty)"));
        return None;
    }
    let mut unblock = None;
    ui.menu_button("Blocked", |ui| {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        egui::ScrollArea::vertical()
            .max_height(220.0)
            .show(ui, |ui| blocked_entries(ui, &app.config.blocked, &mut unblock));
    });
    unblock
}

fn blocked_entries(ui: &mut egui::Ui, blocked: &[String], out: &mut Option<Action>) {
    for (i, name) in blocked.iter().enumerate() {
        if item(ui, true, name) {
            *out = Some(Action::Unblock(i));
        }
    }
}

fn process_row(
    ui: &mut egui::Ui,
    p: &crate::models::ProcessInfo,
    selected: bool,
    fav: bool,
    app: &InjectorApp,
) -> egui::Response {
    let (rect, r) = row(ui);
    row_bg(ui, rect, selected, r.hovered() || r.context_menu_opened());
    in_row(ui, rect, |ui| {
        icon(ui, app.icon_cache.get(&p.pid));
        ui.add_space(6.0);
        if fav {
            ui.label(RichText::new("★").strong());
            ui.add_space(4.0);
        }
        ui.add(egui::Label::new(&p.name).truncate().selectable(false).sense(Sense::hover()));
        ui.add_space(4.0);
        ui.label(RichText::new(format!("({})", p.pid)).color(Color32::from_gray(160)));
    });
    r
}

fn dll_panel(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
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
        .show(ui, |ui| changed |= dll_footer(ui, app));
    ui.label(RichText::new("DLLs").size(15.0).strong());
    ui.add_space(8.0);
    changed | scroll(ui, "dll_list", |ui| dll_list(ui, app))
}

fn dll_footer(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    let y = ui.cursor().min.y + 0.5;
    ui.painter()
        .hline(ui.max_rect().x_range(), y, Stroke::new(1.0, RULE));
    ui.add_space(11.0);
    dll_settings(ui, app) | {
        ui.add_space(10.0);
        dll_actions(ui, app)
    }
}

fn dll_settings(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    ui.horizontal(|ui| {
        let a = ui
            .checkbox(&mut app.config.copy_dll_on_inject, "Copy on inject")
            .on_hover_text("Keeps the original DLL free for rebuilding.")
            .changed();
        ui.add_space(16.0);
        a || ui
            .add_enabled(
                app.config.copy_dll_on_inject,
                egui::Checkbox::new(&mut app.config.randomize_dll_name, "Random name"),
            )
            .on_hover_text("Gives the copied DLL a random name.")
            .changed()
    })
    .inner
}

fn dll_actions(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    let n = app.selected_dlls().count();
    let can = app.selected_process.is_some() && n > 0 && !app.is_injecting;
    let gap = ui.spacing().item_spacing.x;
    let size = Vec2::new(((ui.available_width() - gap * 2.0) / 3.0).floor().clamp(49.0, 250.0), 24.0);
    let mut changed = false;
    ui.horizontal(|ui| {
        if btn(ui, true, "Add DLL", size) {
            changed |= apply(app, ui.ctx(), Some(Action::AddDll));
        }
        if btn(ui, can, if app.is_injecting { "Injecting…" } else { "Inject" }, size) {
            app.start_injection(ui.ctx());
        }
        if btn(ui, n > 0, "Remove", size) {
            let before = app.config.dlls.len();
            app.config.dlls.retain(|d| !d.selected);
            app.add_toast(ToastLevel::Info, format!("Removed {} DLL(s).", before - app.config.dlls.len()));
            changed = true;
        }
    });
    changed
}

fn dll_list(ui: &mut egui::Ui, app: &mut InjectorApp) -> bool {
    let mut menu = None;
    let mut changed = false;
    if app.config.dlls.is_empty() {
        ui.colored_label(Color32::DARK_GRAY, "No DLLs added yet.");
    } else {
        let tex = app.default_dll_texture.id();
        let can = app.selected_process.is_some() && !app.is_injecting;
        for (i, dll) in app.config.dlls.iter_mut().enumerate() {
            changed |= dll_row(ui, dll, tex, i, can, &mut menu);
        }
    }
    let add = space_menu(ui, |ui| item(ui, true, "Add DLL").then_some(Action::AddDll));
    apply(app, ui.ctx(), menu) | changed | apply(app, ui.ctx(), add)
}

fn dll_row(
    ui: &mut egui::Ui,
    dll: &mut Dll,
    tex: egui::TextureId,
    i: usize,
    can: bool,
    menu: &mut Option<Action>,
) -> bool {
    let (rect, r) = row(ui);
    let mut changed = false;
    let check = in_row(ui, rect, |ui| {
        ui.add(egui::Image::new((tex, Vec2::splat(16.0))).sense(Sense::hover()));
        ui.add_space(6.0);
        let name = Path::new(&dll.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&dll.path);
        let c = ui.checkbox(&mut dll.selected, name);
        changed = c.changed();
        c
    });
    let r = r.union(check);
    row_bg(ui, rect, false, r.hovered() || r.context_menu_opened());
    dll_menu(&r, &dll.path, i, can, menu);
    changed
}

fn dll_menu(r: &egui::Response, path: &str, i: usize, can: bool, menu: &mut Option<Action>) {
    menu_on(r, |ui| {
        if item(ui, true, "Open file location") {
            *menu = Some(Action::Open(path.into()));
        }
        if item(ui, can, "Inject") {
            *menu = Some(Action::Inject(path.into()));
        }
        if item(ui, true, "Delete") {
            *menu = Some(Action::Delete(i));
        }
    });
}

enum Action {
    Select(u32),
    Fav(u32),
    Block(u32),
    Unblock(usize),
    AddDll,
    Open(String),
    Inject(String),
    Delete(usize),
}

fn apply(app: &mut InjectorApp, ctx: &egui::Context, action: Option<Action>) -> bool {
    match action {
        Some(Action::Select(pid)) => select_process(app, pid),
        Some(Action::Fav(pid)) => with_name(app, pid, |app, name| {
            app.config.toggle_favorite(name);
            app.order_processes_by_favorite();
        }),
        Some(Action::Block(pid)) => with_name(app, pid, |app, name| {
            if app.selected_process == Some(pid) {
                app.selected_process = None;
            }
            app.config.block_process(name);
        }),
        Some(Action::Unblock(i)) => {
            app.config.unblock_at(i);
            true
        }
        Some(Action::AddDll) => add_dll(app),
        Some(Action::Open(path)) => {
            if let Err(e) = open_location(&path) {
                app.add_toast(ToastLevel::Error, e);
            }
            false
        }
        Some(Action::Inject(path)) => {
            app.start_injection_of(ctx, vec![path]);
            false
        }
        Some(Action::Delete(i)) if i < app.config.dlls.len() => {
            app.config.dlls.remove(i);
            app.add_toast(ToastLevel::Info, "Removed 1 DLL.");
            true
        }
        _ => false,
    }
}

fn select_process(app: &mut InjectorApp, pid: u32) -> bool {
    if app.selected_process == Some(pid) {
        return false;
    }
    let Some(name) = app.processes.iter().find(|p| p.pid == pid).map(|p| p.name.clone()) else {
        return false;
    };
    app.selected_process = Some(pid);
    app.config.last_selected_app = Some(name);
    true
}

fn with_name(app: &mut InjectorApp, pid: u32, f: impl FnOnce(&mut InjectorApp, &str)) -> bool {
    let Some(name) = app.processes.iter().find(|p| p.pid == pid).map(|p| p.name.clone()) else {
        return false;
    };
    f(app, &name);
    true
}

fn add_dll(app: &mut InjectorApp) -> bool {
    let Some(path) = crate::core::dll_selector::select_dll() else {
        return false;
    };
    if app.config.dlls.iter().any(|d| d.path == path) {
        app.add_toast(ToastLevel::Warning, "DLL is already in the list.");
        return false;
    }
    app.config.dlls.push(Dll { path, selected: false });
    true
}

fn open_location(path: &str) -> Result<(), String> {
    if !Path::new(path).exists() {
        return Err("DLL file not found.".into());
    }
    Command::new("explorer")
        .arg(format!("/select,{path}"))
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to open location: {e}"))
}

fn menu_on(r: &egui::Response, add: impl FnOnce(&mut egui::Ui)) {
    r.context_menu(|ui| {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        add(ui);
    });
}

fn space_menu(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> Option<Action>) -> Option<Action> {
    let mut out = None;
    menu_on(&fill(ui), |ui| out = add(ui));
    out
}

fn fill(ui: &mut egui::Ui) -> egui::Response {
    ui.allocate_exact_size(
        Vec2::new(ui.available_width(), ui.available_height().max(28.0)),
        Sense::click(),
    )
    .1
}

fn item(ui: &mut egui::Ui, enabled: bool, label: &str) -> bool {
    let clicked = ui.add_enabled(enabled, egui::Button::new(label)).clicked();
    if clicked {
        ui.close();
    }
    clicked
}

fn btn(ui: &mut egui::Ui, enabled: bool, label: &str, size: Vec2) -> bool {
    ui.add_enabled(enabled, egui::Button::new(label).min_size(size)).clicked()
}

fn row(ui: &mut egui::Ui) -> (Rect, egui::Response) {
    let h = ui.spacing().interact_size.y.max(22.0);
    ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::click())
}

fn in_row<R>(ui: &mut egui::Ui, rect: Rect, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
        ui.set_clip_rect(ui.clip_rect().intersect(rect));
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), add)
            .inner
    })
    .inner
}

fn row_bg(ui: &egui::Ui, rect: Rect, selected: bool, hot: bool) {
    let fill = selected
        .then(|| ui.visuals().selection.bg_fill)
        .or_else(|| hot.then_some(Color32::from_rgba_unmultiplied(255, 255, 255, 10)));
    if let Some(c) = fill {
        ui.painter().rect_filled(rect, 4.0, c);
    }
}

fn icon(ui: &mut egui::Ui, texture: Option<&TextureHandle>) {
    match texture {
        Some(t) => {
            ui.add(egui::Image::new((t.id(), Vec2::splat(16.0))).sense(Sense::hover()));
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
