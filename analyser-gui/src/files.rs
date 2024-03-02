use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use dump_analyser::PcapFile;

#[derive(Debug, Clone)]
pub struct Item {
    pub path: PathBuf,
    pub display_name: String,
    pub selected: bool,
    pub num_points: usize,

    pub round_trip_times: Vec<[f64; 2]>,
    pub cycle_delta_times: Vec<[f64; 2]>,
    // TODO: Stats fields
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
            let pairs = PcapFile::new(path).match_tx_rx();

            let round_trip_times = pairs
                .iter()
                .enumerate()
                .map(|(i, item)| [i as f64, item.delta_time.as_nanos() as f64 / 1000.0])
                .collect();

            let cycle_delta_times = pairs
                .windows(2)
                .into_iter()
                .enumerate()
                .map(|(i, stats)| {
                    let [prev, curr] = stats else { unreachable!() };

                    let t = curr.tx_time.as_nanos() - prev.tx_time.as_nanos();

                    [i as f64, t as f64 / 1000.0]
                })
                .collect();

            self.names.insert(
                path.clone(),
                Item {
                    selected: false,
                    path: path.clone(),
                    display_name: path.file_stem().unwrap().to_string_lossy().to_string(),
                    round_trip_times,
                    cycle_delta_times,
                    num_points: pairs.len(),
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

    pub fn selected_paths(&self) -> impl Iterator<Item = &Item> {
        self.names.values().filter(|v| v.selected)
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
