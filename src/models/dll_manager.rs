#[derive(Default)]
pub struct DLLManager {
    dlls: Vec<String>,
    selected: Vec<bool>,
}

impl DLLManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, path: String) {
        self.dlls.push(path);
        self.selected.push(false);
    }

    pub fn remove_selected(&mut self) -> usize {
        let removed = self.selected.iter().filter(|&&selected| selected).count();
        let mut index = 0;
        self.dlls.retain(|_| {
            let keep = !self.selected[index];
            index += 1;
            keep
        });
        self.selected.retain(|selected| !*selected);
        removed
    }

    pub fn get_dlls(&self) -> &[String] {
        &self.dlls
    }

    pub fn set_selected(&mut self, index: usize, selected: bool) {
        if let Some(value) = self.selected.get_mut(index) {
            *value = selected;
        }
    }

    pub fn is_selected(&self, index: usize) -> bool {
        self.selected.get(index).copied().unwrap_or(false)
    }

    pub fn selected_count(&self) -> usize {
        self.selected.iter().filter(|&&selected| selected).count()
    }

    pub fn selected_paths(&self) -> impl Iterator<Item = &str> {
        self.dlls
            .iter()
            .zip(&self.selected)
            .filter_map(|(path, &selected)| selected.then_some(path.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::DLLManager;

    #[test]
    fn selects_and_removes_multiple_dlls() {
        let mut manager = DLLManager::new();
        for path in ["a.dll", "b.dll", "c.dll"] {
            manager.add(path.into());
        }
        manager.set_selected(0, true);
        manager.set_selected(2, true);

        assert_eq!(
            manager.selected_paths().collect::<Vec<_>>(),
            ["a.dll", "c.dll"]
        );
        assert_eq!(manager.remove_selected(), 2);
        assert_eq!(manager.get_dlls(), ["b.dll"]);
        assert_eq!(manager.selected_count(), 0);
    }
}
