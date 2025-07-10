use rfd::FileDialog;

pub fn select_dll() -> Option<String> {
    FileDialog::new()
        .add_filter("DLL Files", &["dll"])
        .pick_file()
        .map(|p| p.to_string_lossy().into_owned())
}