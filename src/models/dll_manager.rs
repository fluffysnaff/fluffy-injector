#[derive(Default)]
pub struct DLLManager {
    dlls: Vec<String>,
    selected: Option<usize>,
}

impl DLLManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, path: String) {
        self.dlls.push(path);
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.dlls.len() {
            self.dlls.remove(index);
            if let Some(selected_idx) = self.selected {
                if selected_idx == index {
                    self.selected = None;
                } else if selected_idx > index {
                    self.selected = Some(selected_idx - 1);
                }
            }
        }
    }

    pub fn get_dlls(&self) -> &Vec<String> {
        &self.dlls
    }

    pub fn select(&mut self, index: Option<usize>) {
        self.selected = index;
    }

    pub fn selected_dll(&self) -> Option<usize> {
        self.selected
    }

    pub fn selected_path(&self) -> Option<String> {
        self.selected.map(|i| self.dlls[i].clone())
    }
}