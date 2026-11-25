use crate::app::InjectorApp;
use crate::models::{Dll, Notice, ProcessInfo, ToastLevel, DEFAULT_SPLIT_RATIO};
use std::borrow::Cow;
use std::path::Path;
use std::time::Instant;
use taut::{Align, Color, Icon, Key, Length, Mods, Picture, Role, Theme, Ui};
use windows::core::{w, PCWSTR};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

const SURFACE: Color = Color::rgb(0x1c1c1c);
const RAISED: Color = Color::rgb(0x242424);
const RULE: Color = Color::rgb(0x3a3a3a);
const BG: Color = Color::rgb(0x101010);
const HOVER_ROW: Color = Color::rgb(0x252525);
const PANEL_MIN: f32 = 140.0;
const PANEL_PAD: f32 = 12.0;
const SPLITTER_WIDTH: f32 = 5.0;
const ICON_SIZE: f32 = 16.0;
const ROW_PAD: f32 = 4.0;
const DLL_ICON_GAP: f32 = 6.0;
const CHECKBOX_WIDTH: f32 = 24.0; // 16 px mark and 8 px gap in taut's checkbox.
const FAVORITE: Icon =
    Icon::solid("M8 1.8l1.9 3.9 4.3.6-3.1 3 .7 4.3L8 11.6l-3.8 2 .7-4.3-3.1-3 4.3-.6z");

pub(crate) fn theme() -> Theme {
    Theme {
        bg: SURFACE,
        panel: BG,
        raised: RAISED,
        hover: RULE,
        line: RULE,
        line_strong: RULE,
        selected: Color::rgb(0x325a96),
        accent: Color::rgb(0x8cbeff),
        text_dim: Color::rgb(0xa0a0a0),
        radius: 4.0,
        ..Theme::dark()
    }
}

pub(crate) fn show(ui: &mut Ui, app: &mut InjectorApp) -> bool {
    ui.column().grow().open();
    status_bar(ui, app);
    let changed = split_body(ui, app);
    ui.close();
    changed
}

fn status_bar(ui: &mut Ui, app: &InjectorApp) {
    let t = ui.theme;
    let process = match app.selected_process_info() {
        Some(process) => format!("{}  ·  PID {}", process.name, process.pid),
        None if app.multiple_matches() => "Multiple matches, select a PID".into(),
        None => app.config.last_selected_app.as_ref().map_or_else(
            || "No process selected".into(),
            |name| format!("Waiting for {name}"),
        ),
    };
    ui.row()
        .w(Length::Grow(1.0))
        .pad((14.0, t.gap * 1.5))
        .gap(t.gap)
        .align(Align::Start, Align::Center)
        .fill(t.raised)
        .border(t.line, 1.0)
        .open();
    ui.el()
        .w(Length::Grow(1.0))
        .text(&process, t.font, t.text)
        .clip()
        .show();
    ui.text(
        &format!("DLLs selected: {}", app.selected_dlls().count()),
        t.font,
        t.accent,
    );
    ui.close();
}

fn split_body(ui: &mut Ui, app: &mut InjectorApp) -> bool {
    let size = ui.size();
    let ratio = if app.config.split_ratio.is_finite() {
        app.config.split_ratio
    } else {
        DEFAULT_SPLIT_RATIO
    };
    let max = (size.w - SPLITTER_WIDTH - PANEL_MIN).max(PANEL_MIN);
    let mut left = (size.w * ratio).clamp(PANEL_MIN, max);
    ui.row().grow().open();
    ui.column()
        .w(Length::Px(left))
        .h(Length::Grow(1.0))
        .pad(PANEL_PAD)
        .gap(8.0)
        .clip()
        .open();
    let mut changed = process_panel(ui, app);
    ui.close();
    if ui.splitter(&mut left, PANEL_MIN, max).changed {
        app.config.split_ratio = left / size.w.max(1.0);
        changed = true;
    }
    ui.column().grow().pad(PANEL_PAD).gap(8.0).clip().open();
    changed |= dll_panel(ui, app);
    ui.close();
    ui.close();
    changed
}

fn process_panel(ui: &mut Ui, app: &mut InjectorApp) -> bool {
    heading(ui, "Processes");
    if app.scan_failed {
        ui.note("Process scan failed. Showing last results.");
    }
    let search = ui.text_box(&mut app.process_search, "Filter by name…");
    let mut search_changed = search.changed;
    if ui.popup_anchor().is_none() {
        if ui.keys("Ctrl+F") {
            ui.set_focus(search.id);
            ui.select_all();
        } else if search.focused {
            if ui.key(Key::Enter, Mods::NONE) {
                ui.set_focus(0);
            } else if ui.key(Key::Escape, Mods::NONE) {
                if app.process_search.is_empty() {
                    ui.set_focus(0);
                } else {
                    app.process_search.clear();
                    search_changed = true;
                }
            }
        }
    }
    if search_changed {
        app.rebuild_process_rows();
        if !app.process_rows.is_empty() {
            let row = if app.process_search.is_empty() {
                app.selected_row()
            } else {
                None
            };
            ui.scroll_to_row(row.unwrap_or(0));
        }
    }
    if !app.process_search.is_empty()
        && app.selected_process.is_some()
        && app.selected_row().is_none()
    {
        ui.note("Selected process is hidden by filter");
        if ui.button("Clear filter").clicked {
            app.process_search.clear();
            app.rebuild_process_rows();
            if let Some(row) = app.selected_row() {
                ui.scroll_to_row(row);
            }
            ui.rerun();
        }
    }
    let list = ui.column().grow().scroll().sense().open();
    let mut action = None;
    if app.is_loading_processes && app.processes.is_empty() {
        ui.row()
            .w(Length::Grow(1.0))
            .pad((0.0, 24.0))
            .align(Align::Center, Align::Center)
            .open();
        ui.spinner();
        ui.close();
    } else if app.process_rows.is_empty() {
        ui.note("No matching processes.");
    } else {
        for row in ui.rows(app.process_rows.len(), ui.theme.control_height) {
            let (index, favorite) = app.process_rows[row];
            let process = &app.processes[index];
            let selected = app.selected_process == Some(process.pid);
            let picture = app.process_icon(&process.exe);
            action = process_row(ui, index, process, selected, favorite, picture).or(action);
        }
        ui.close();
    }
    ui.close();
    if ui.context_menu(&list) {
        if app.config.blocked.is_empty() {
            ui.set_enabled(false);
            ui.menu_item("Blocked (empty)");
            ui.set_enabled(true);
        } else if ui.submenu("Blocked") {
            for (index, name) in app.config.blocked.iter().enumerate() {
                if ui.menu_item(&name.replace('&', "&&")).clicked {
                    action = Some(ProcessAction::Unblock(index));
                }
            }
            ui.close();
        }
        ui.close();
    }
    let changed = action.is_some_and(|action| apply_process_action(app, action));
    if changed {
        ui.rerun();
    }
    changed
}

fn process_row(
    ui: &mut Ui,
    index: usize,
    process: &ProcessInfo,
    selected: bool,
    favorite: bool,
    picture: Option<Picture>,
) -> Option<ProcessAction> {
    let t = ui.theme;
    let key = process.pid;
    let response = ui.press(ui.id(key));
    let mut action = response.clicked.then_some(ProcessAction::Select(index));
    ui.row().key(key).grow().sense().focusable().open();
    let menu_open = ui.context_menu(&response);
    if menu_open {
        let label = if favorite { "Unfavorite" } else { "Favorite" };
        if ui.menu_item(label).clicked {
            action = Some(ProcessAction::Favorite(index));
        }
        if ui.menu_item("Block from list").clicked {
            action = Some(ProcessAction::Block(index));
        }
        ui.close();
    }

    ui.row()
        .grow()
        .pad((ROW_PAD, 0.0))
        .gap(5.0)
        .align(Align::Start, Align::Center)
        .fill(if selected {
            t.selected
        } else if response.hovered || menu_open {
            HOVER_ROW
        } else {
            Color::NONE
        })
        .radius(t.radius)
        .open();
    icon(ui, picture);
    if favorite {
        ui.el().icon(FAVORITE, 12.0, t.text).show();
    }
    let pid = format!("({key})");
    let name = ui
        .el()
        .text(&process.name, t.font, t.text)
        .shrink(1)
        .clip()
        .show();
    if name.rect.w < ui.text_width(&process.name, t.font) {
        ui.tooltip(&response, &process.name);
    }
    ui.text(&pid, t.font, t.text_dim);
    ui.close();
    ui.close();
    ui.tell(
        &response,
        Role::ListItem,
        Some(selected),
        &format!("{} {pid}", process.name),
    );
    action
}

enum ProcessAction {
    Select(usize),
    Favorite(usize),
    Block(usize),
    Unblock(usize),
}

fn apply_process_action(app: &mut InjectorApp, action: ProcessAction) -> bool {
    match action {
        ProcessAction::Select(index) => {
            let process = &app.processes[index];
            if app.selected_process == Some(process.pid) {
                return false;
            }
            app.selected_process = Some(process.pid);
            app.config.last_selected_app = Some(process.name.clone());
            return true;
        }
        ProcessAction::Favorite(index) => app.config.toggle_favorite(&app.processes[index].name),
        ProcessAction::Block(index) => {
            app.config.block_process(&app.processes[index].name);
            app.resolve_selection();
        }
        ProcessAction::Unblock(index) => {
            app.config.unblock_at(index);
            app.resolve_selection();
        }
    }
    app.rebuild_process_rows();
    true
}

fn dll_panel(ui: &mut Ui, app: &mut InjectorApp) -> bool {
    heading(ui, "DLLs");
    let over = ui.files_hovered()
        && ui
            .pointer()
            .is_some_and(|at| ui.interact(ui.next_id()).rect.contains(at));
    let fill = if over { ui.theme.raised } else { Color::NONE };
    let list = ui.column().grow().scroll().fill(fill).sense().open();
    let mut changed = false;
    let mut action = None;
    let mut open_menu = None;
    if app.config.dlls.is_empty() {
        ui.note("No DLLs added yet.");
    } else {
        let can_inject = app.can_inject();
        let picture = app.dll_icon();
        let label_width =
            ui.available_size().w - 2.0 * ROW_PAD - ICON_SIZE - DLL_ICON_GAP - CHECKBOX_WIDTH;
        for index in ui.rows(app.config.dlls.len(), ui.theme.control_height) {
            let (toggled, menu, menu_open) = dll_row(
                ui,
                &mut app.config.dlls[index],
                picture,
                index,
                can_inject,
                label_width,
                app.dll_menu == Some(index),
            );
            changed |= toggled;
            action = menu.or(action);
            if menu_open {
                open_menu = Some(index);
            }
        }
        ui.close();
    }
    ui.close();
    if app.dll_menu != open_menu {
        app.dll_menu = open_menu;
        ui.rerun();
    }
    if let Some(paths) = ui.files_dropped(&list) {
        for path in paths {
            changed |= app.add_dll(path);
        }
    }
    if ui.context_menu(&list) {
        if ui.menu_item("Add DLL").clicked {
            action = Some(DllAction::Add);
        }
        ui.close();
    }
    if let Some(action) = action {
        changed |= apply_dll_action(app, action);
    }
    changed | dll_footer(ui, app)
}

fn dll_row(
    ui: &mut Ui,
    dll: &mut Dll,
    picture: Option<Picture>,
    index: usize,
    can_inject: bool,
    label_width: f32,
    was_open: bool,
) -> (bool, Option<DllAction>, bool) {
    let radius = ui.theme.radius;
    let mut row = ui.row().key(&dll.path).grow().sense().clip().open();
    ui.row()
        .grow()
        .pad((ROW_PAD, 0.0))
        .gap(DLL_ICON_GAP)
        .align(Align::Start, Align::Center)
        .fill(if row.hovered || row.contains_pointer || was_open {
            HOVER_ROW
        } else {
            Color::NONE
        })
        .radius(radius)
        .open();
    icon(ui, picture);
    let name = Path::new(&dll.path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&dll.path);
    let label = truncate(ui, name, label_width);
    let check = ui.checkbox(&mut dll.selected, &label);
    ui.tooltip(&check, &dll.path);
    ui.close();
    ui.close();

    // Inside the row the checkbox owns pointer and keyboard input.
    row.right_clicked |= check.right_clicked;
    row.focused |= check.focused;
    let mut action = None;
    let menu_open = ui.context_menu(&row);
    if menu_open {
        if ui.menu_item("Open file location").clicked {
            action = Some(DllAction::Open(index));
        }
        ui.set_enabled(can_inject);
        if ui.menu_item("Inject").clicked {
            action = Some(DllAction::Inject(index));
        }
        ui.set_enabled(true);
        if ui.menu_item("Delete").clicked {
            action = Some(DllAction::Delete(index));
        }
        ui.close();
    }
    (check.changed, action, menu_open)
}

fn truncate<'a>(ui: &mut Ui, text: &'a str, width: f32) -> Cow<'a, str> {
    let font = ui.theme.font;
    if ui.text_width(text, font).ceil() <= width {
        return Cow::Borrowed(text);
    }
    let width = width - ui.text_width("…", font).ceil();
    let ends: Vec<_> = text.char_indices().map(|(index, _)| index).collect();
    let fit = ends.partition_point(|&end| ui.text_width(&text[..end], font).ceil() <= width);
    let end = ends.get(fit.saturating_sub(1)).copied().unwrap_or(0);
    Cow::Owned(format!("{}…", &text[..end]))
}

enum DllAction {
    Add,
    Open(usize),
    Inject(usize),
    Delete(usize),
}

fn apply_dll_action(app: &mut InjectorApp, action: DllAction) -> bool {
    match action {
        DllAction::Add => app.pick_dll(),
        DllAction::Open(index) => {
            if let Err(error) = open_location(&app.config.dlls[index].path) {
                app.add_toast(ToastLevel::Error, error);
            }
        }
        DllAction::Inject(index) => {
            app.start_injection(vec![app.config.dlls[index].path.clone()]);
        }
        DllAction::Delete(index) => {
            app.config.dlls.remove(index);
            app.add_toast(ToastLevel::Info, "Removed 1 DLL.");
            return true;
        }
    }
    false
}

fn dll_footer(ui: &mut Ui, app: &mut InjectorApp) -> bool {
    ui.separator();
    ui.wrapping_row().gap(6.0).open();
    let copy = ui.checkbox(&mut app.config.copy_dll_on_inject, "Copy on inject");
    ui.tooltip(&copy, "Keeps the original DLL free for rebuilding.");
    ui.set_enabled(app.config.copy_dll_on_inject);
    let random = ui.checkbox(&mut app.config.randomize_dll_name, "Random name");
    ui.set_enabled(true);
    ui.tooltip(&random, "Gives the copied DLL a random name.");
    ui.close();

    let count = app.selected_dlls().count();
    let can_inject = count > 0 && app.can_inject();
    let inject_label = if app.is_injecting() {
        "Injecting…"
    } else {
        "Inject"
    };
    let gap = 6.0;
    let needed = 3.0 * (ui.text_width("Injecting…", ui.theme.font).ceil() + 24.0) + 2.0 * gap;
    let across = ui.available_size().w >= needed;
    let actions = if across { ui.row() } else { ui.column() };
    actions
        .w(Length::Grow(1.0))
        .gap(gap)
        .align(Align::Start, Align::Spread)
        .open();
    let add = footer_button(ui, across, "Add DLL", true);
    let inject = footer_button(ui, across, inject_label, can_inject);
    let remove = footer_button(ui, across, "Remove", count > 0);
    ui.close();
    if add {
        app.pick_dll();
    }
    if inject {
        app.start_injection(app.selected_dlls().map(str::to_owned).collect());
    }
    if remove {
        app.config.dlls.retain(|dll| !dll.selected);
        app.add_toast(ToastLevel::Info, format!("Removed {count} DLL(s)."));
    }
    copy.changed || random.changed || remove
}

fn footer_button(ui: &mut Ui, across: bool, label: &str, enabled: bool) -> bool {
    let button = ui.column().align(Align::Start, Align::Spread);
    if across {
        button.w(Length::Grow(1.0)).open();
    } else {
        button.open();
    }
    ui.set_enabled(enabled);
    let clicked = ui.button(label).clicked;
    ui.set_enabled(true);
    ui.close();
    clicked
}

fn heading(ui: &mut Ui, label: &str) {
    ui.text(label, ui.theme.font.bold(), ui.theme.text);
}

fn icon(ui: &mut Ui, picture: Option<Picture>) {
    if let Some(picture) = picture {
        // picture() resets the size to the image's own, so size() must follow it.
        ui.el().picture(picture).size(ICON_SIZE, ICON_SIZE).show();
    } else {
        let color = ui.theme.text_faint;
        ui.row()
            .size(ICON_SIZE, ICON_SIZE)
            .align(Align::Center, Align::Center)
            .open();
        ui.el().size(4.0, 4.0).radius(2.0).fill(color).show();
        ui.close();
    }
}

pub(crate) fn show_notices(ui: &mut Ui, notices: &[Notice], now: Instant) {
    let Some(expires) = notices.iter().map(|notice| notice.expires).min() else {
        return;
    };
    ui.wake_after(expires.saturating_duration_since(now));
    let t = ui.theme;
    ui.notices(Align::End, Align::End);
    for notice in notices {
        let (prefix, color) = match notice.level {
            ToastLevel::Info => ("Info", Color::rgb(0xc8c8c8)),
            ToastLevel::Success => ("OK", Color::rgb(0x78dc8c)),
            ToastLevel::Warning => ("Warn", Color::rgb(0xf0c850)),
            ToastLevel::Error => ("Error", Color::rgb(0xf06464)),
        };
        ui.el()
            .key(notice.id)
            .w(Length::Px(220.0))
            .pad((10.0, 8.0))
            .fill(t.raised)
            .border(t.line, 1.0)
            .radius(t.radius)
            .text(&format!("{prefix}: {}", notice.message), t.font, color)
            .wrap()
            .show();
    }
    ui.close();
}

fn open_location(path: &str) -> Result<(), String> {
    if !Path::new(path).exists() {
        return Err("DLL file not found.".into());
    }
    let params: Vec<u16> = format!("/select,\"{path}\"")
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let rc = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            w!("explorer.exe"),
            PCWSTR(params.as_ptr()),
            None,
            SW_SHOWNORMAL,
        )
    };
    if (rc.0 as isize) > 32 {
        return Ok(());
    }
    Err(format!("Failed to open location: error {}", rc.0 as isize))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Config, MIN_WINDOW_SIZE};
    use taut::testing::Trial;

    fn trial() -> Trial<InjectorApp> {
        let mut app = InjectorApp::new(Config::default());
        app.is_loading_processes = false;
        app.processes = ["Alpha.exe", "Beta.exe", "Gamma.exe"]
            .into_iter()
            .enumerate()
            .map(|(index, name)| ProcessInfo {
                name: name.into(),
                pid: index as u32 + 1,
                exe: Default::default(),
            })
            .collect();
        app.rebuild_process_rows();
        Trial::new(app, |app, ui| {
            ui.theme = theme();
            show(ui, app);
        })
    }

    #[test]
    fn large_icon_is_scaled_and_leaves_room_for_the_label() {
        let mut window = Trial::new(None, |picture, ui| {
            let picture = *picture.get_or_insert_with(|| ui.add_picture(384, vec![0; 384 * 512]));
            ui.row().open();
            icon(ui, Some(picture));
            ui.button("Label");
            ui.close();
        });
        assert!(window.center_of(Role::Button, "Label").x < 100.0);
    }

    #[test]
    fn search_accepts_keys_without_changing_selection() {
        let mut window = trial();
        window.press(Role::ListItem, "Alpha.exe (1)");
        window.key("Ctrl+F");
        window.fill("Filter by name…", "bEtA");
        assert!(window.has(Role::ListItem, "Beta.exe (2)"));
        assert!(!window.has(Role::ListItem, "Alpha.exe (1)"));
        window.key("Enter");
        assert_eq!(window.app.selected_process, Some(1));
        assert_eq!(window.ui.focus(), 0);
        window.key("Ctrl+F");
        window.key("Escape");
        assert!(window.app.process_search.is_empty());
        assert!(window.has(Role::ListItem, "Alpha.exe (1)"));
    }

    #[test]
    fn clear_filter_reveals_the_selected_row() {
        let mut window = trial();
        window.press(Role::ListItem, "Gamma.exe (3)");
        window.fill("Filter by name…", "Alpha");
        assert!(window.has(Role::Button, "Clear filter"));
        assert!(!window.has(Role::ListItem, "Gamma.exe (3)"));
        window.press(Role::Button, "Clear filter");
        assert!(window.has(Role::ListItem, "Gamma.exe (3)"));
        assert_eq!(window.app.selected_process, Some(3));
        assert!(!window.has(Role::Button, "Clear filter"));
    }

    #[test]
    fn dll_drop_uses_file_extension_and_canonical_identity() {
        let dir = std::env::temp_dir().join(format!(
            "fluffy-drop-test-{}-{}",
            std::process::id(),
            std::random::random::<u64>(..)
        ));
        std::fs::create_dir(&dir).unwrap();
        let dll = dir.join("test.DLL");
        let text = dir.join("test.txt");
        std::fs::write(&dll, []).unwrap();
        std::fs::write(&text, []).unwrap();

        let mut window = trial();
        window.drop_files(
            taut::geom::pt(540.0, 145.0),
            &[
                dll.clone(),
                text.clone(),
                dir.join("missing.dll"),
                dir.join(".").join("test.DLL"),
            ],
        );
        let count = window.app.config.dlls.len();
        let stored = window
            .app
            .config
            .dlls
            .first()
            .map(|entry| Path::new(&entry.path));
        std::fs::remove_file(&dll).unwrap();
        std::fs::remove_file(text).unwrap();
        std::fs::remove_dir(dir).unwrap();
        assert_eq!(count, 1);
        assert_eq!(stored, Some(dll.as_path()));
    }

    #[test]
    fn process_order_is_stable_and_favorites_first() {
        let mut window = trial();
        for (favorites, expected) in [
            (&[][..], [(0, false), (1, false), (2, false)]),
            (&["GAMMA.EXE"][..], [(2, true), (0, false), (1, false)]),
            (
                &["gAmMa.exe", "beta.EXE"][..],
                [(1, true), (2, true), (0, false)],
            ),
        ] {
            window.app.config.favorites = favorites.iter().map(|name| (*name).into()).collect();
            window.app.rebuild_process_rows();
            assert_eq!(window.app.process_rows, expected, "{favorites:?}");
        }
    }

    #[test]
    fn selection_after_a_process_disappears_requires_a_unique_match() {
        let mut window = trial();
        let cases: [(&[(&str, u32)], Option<u32>, bool); 4] = [
            (&[("Beta.exe", 1)], None, false),
            (&[("ALPHA.EXE", 4)], Some(4), false),
            (&[("Alpha.exe", 4), ("alpha.exe", 5)], None, true),
            (&[("Alpha.exe", 1), ("Alpha.exe", 4)], Some(1), false),
        ];
        for (rows, expected, ambiguous) in cases {
            window.app.selected_process = Some(1);
            window.app.config.last_selected_app = Some("Alpha.exe".into());
            window.app.processes = rows
                .iter()
                .map(|&(name, pid)| ProcessInfo {
                    name: name.into(),
                    pid,
                    exe: Default::default(),
                })
                .collect();
            window.app.resolve_selection();
            assert_eq!(window.app.selected_process, expected, "{rows:?}");
            assert_eq!(window.app.multiple_matches(), ambiguous, "{rows:?}");
            assert_eq!(window.app.can_inject(), expected.is_some(), "{rows:?}");
        }
    }

    #[test]
    fn process_menus_update_visible_rows_and_escape_closes_them() {
        let mut window = trial();
        let beta = window.center_of(Role::ListItem, "Beta.exe (2)");
        window.right_click(beta);
        window.press(Role::MenuItem, "Favorite");
        assert_eq!(window.app.process_rows[0], (1, true));

        let beta = window.center_of(Role::ListItem, "Beta.exe (2)");
        window.right_click(beta);
        window.press(Role::MenuItem, "Unfavorite");
        assert_eq!(window.app.process_rows[0], (0, false));

        let beta = window.center_of(Role::ListItem, "Beta.exe (2)");
        window.right_click(beta);
        window.key("Escape");
        assert!(!window.has(Role::MenuItem, "Block from list"));
        window.right_click(beta);
        window.press(Role::MenuItem, "Block from list");
        assert!(!window.has(Role::ListItem, "Beta.exe (2)"));
    }

    #[test]
    fn resizing_does_not_overwrite_a_dragged_split() {
        let mut window = trial();
        let from = window.center_of(Role::Slider, "Processes");
        window.drag(from, taut::geom::pt(from.x + 50.0, from.y));
        let dragged = window.app.config.split_ratio;
        assert!(dragged > DEFAULT_SPLIT_RATIO + 0.05);
        window.resize(MIN_WINDOW_SIZE[0], MIN_WINDOW_SIZE[1], 1.0);
        assert_eq!(window.app.config.split_ratio, dragged);
        assert!(window.has(Role::Button, "Add DLL"));
        window.resize(680.0, 450.0, 1.0);
        assert_eq!(window.app.config.split_ratio, dragged);
    }

    #[test]
    fn list_rows_and_controls_keep_the_same_height_in_layout_units_at_high_dpi() {
        for scale in [1.0, 1.25, 1.5, 2.0] {
            let mut window = trial();
            window.resize(680.0, 450.0, scale);
            let first = window.center_of(Role::ListItem, "Alpha.exe (1)");
            let second = window.center_of(Role::ListItem, "Beta.exe (2)");
            assert!((second.y - first.y - theme().control_height).abs() < 0.1);
            assert!(window.has(Role::Button, "Add DLL"));
        }
    }
}
