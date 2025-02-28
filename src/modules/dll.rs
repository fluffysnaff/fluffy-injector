use rfd::FileDialog;

pub struct DLLManager {
    dlls: Vec<String>,
    selected: Option<usize>,
}

impl DLLManager {
    pub fn new() -> Self {
        Self {
            dlls: vec![],
            selected: None,
        }
    }

    pub fn add(&mut self, path: String) {
        self.dlls.push(path);
    }

    pub fn get_dlls(&self) -> &Vec<String> {
        &self.dlls
    }

    pub fn select(&mut self, index: usize) {
        self.selected = Some(index);
    }

    pub fn selected_dll(&self) -> Option<usize> {
        self.selected
    }

    pub fn selected_path(&self) -> Option<String> {
        self.selected.map(|i| self.dlls[i].clone())
    }
}

pub fn select_dll() -> Option<String> {
    FileDialog::new()
        .add_filter("DLL Files", &["dll"])
        .pick_file()
        .map(|p| p.to_string_lossy().into_owned())
}
