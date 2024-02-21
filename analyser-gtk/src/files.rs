use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct Item {
    pub path: PathBuf,
    pub display_name: String,
    pub selected: bool,
}

#[derive(Default, Clone)]
pub struct DumpFiles {
    pub names: HashMap<PathBuf, Item>,
}

impl DumpFiles {
    pub fn new(path: PathBuf) -> Self {
        assert!(path.is_dir(), "Path must point to a directory");

        let mut self_ = Self {
            names: HashMap::new(),
        };

        let paths = fs::read_dir(path)
            .expect("read_dir")
            .map(|entry| {
                let entry = entry.unwrap();

                entry.path()
            })
            .collect::<Vec<_>>();

        self_.update_items(paths);

        self_
    }

    pub fn update_items(&mut self, paths: Vec<PathBuf>) {
        let paths = paths
            .into_iter()
            .filter(|path| {
                path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("pcapng")
            })
            .collect::<HashSet<_>>();

        let n2 = self.names.keys().cloned().collect::<HashSet<_>>();

        let new = paths.difference(&n2);

        for path in new.into_iter() {
            self.names.insert(
                path.clone(),
                Item {
                    selected: false,
                    path: path.clone(),
                    display_name: path.file_stem().unwrap().to_string_lossy().to_string(),
                },
            );
        }
    }

    pub fn remove_items(&mut self, remove: Vec<PathBuf>) {
        for path in remove.into_iter() {
            self.names.remove(&path);
        }
    }

    pub fn update_selection(&mut self, selected: Vec<PathBuf>) {
        self.names
            .values_mut()
            .for_each(|value| value.selected = false);

        for path in selected {
            if let Some(item) = self.names.get_mut(&path) {
                item.selected = true;
            }
        }
    }

    pub fn selected_paths(&self) -> impl Iterator<Item = &PathBuf> {
        self.names
            .iter()
            .filter_map(|(k, v)| if v.selected { Some(k) } else { None })
    }

    pub fn all(&self) -> Vec<&Item> {
        let mut sorted = self.names.values().collect::<Vec<_>>();

        sorted.sort_by_key(|item| &item.display_name);

        sorted
    }

    pub fn toggle_selection(&mut self, item: &Path) {
        if let Some(item) = self.names.get_mut(item) {
            item.selected = !item.selected
        }
    }
}
