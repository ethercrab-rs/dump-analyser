use gio::glib::{self, ToValue};
use gtk::prelude::*;
use notify_debouncer_full::{new_debouncer, notify::*, DebounceEventResult, Debouncer, FileIdMap};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

#[repr(u16)]
enum Columns {
    Test,
}

const DUMP_LIST_COL_TYPES: &[glib::Type] = &[glib::Type::STRING];

pub struct DumpFiles {
    store: gtk::ListStore,
    names: HashSet<PathBuf>,
}

impl DumpFiles {
    pub fn new(path: &Path) -> Self {
        assert!(path.is_dir(), "Path must point to a directory");

        let mut self_ = Self {
            store: gtk::ListStore::new(DUMP_LIST_COL_TYPES),
            names: HashSet::new(),
        };

        let paths = fs::read_dir(path)
            .expect("read_dir")
            .filter_map(|entry| {
                let entry = entry.unwrap();
                let path = entry.path();

                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("pcapng") {
                    Some(path)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        self_.update_items(paths);

        self_
    }

    pub async fn start_watch_dir(&self, rx: async_channel::Receiver<DebounceEventResult>) {
        println!("Start watch future");
        while let Ok(result) = rx.recv().await {
            println!("Change event");

            // TODO: This needs to run on the main thread with `glib::source::idle_add_once`
            // self.add_item("Changed".to_string());

            match result {
                Ok(event) => println!("changed: {:?}", event),
                Err(e) => println!("watch error: {:?}", e),
            }
        }
    }

    /// Only call this once.
    pub fn init_view(&self, tree: &mut gtk::TreeView) {
        tree.set_model(Some(&self.store));

        // Add a test column
        {
            let renderer = gtk::CellRendererText::new();
            let column = gtk::TreeViewColumn::new();
            TreeViewColumnExt::pack_start(&column, &renderer, true);
            column.set_title("Testing");
            TreeViewColumnExt::add_attribute(&column, &renderer, "text", Columns::Test as i32);
            column.set_sort_column_id(Columns::Test as i32);
            tree.append_column(&column);
        }
    }

    pub fn update_items(&mut self, paths: Vec<PathBuf>) {
        let paths = paths.into_iter().collect::<HashSet<_>>();

        let n2 = self.names.clone();

        let new = paths.difference(&n2);

        for path in new.into_iter() {
            let p = path.file_stem().to_owned().unwrap().to_string_lossy();

            let display_name = p.as_ref();

            self.names.insert(path.clone());

            self.store.set(
                &self.store.append(),
                &[(Columns::Test as u32, &display_name)],
            );
        }
    }

    pub fn remove_items(&mut self, remove: Vec<PathBuf>) {
        for path in remove.into_iter() {
            let p = path.file_stem().to_owned().unwrap().to_string_lossy();

            let display_name = p.as_ref();

            self.names.remove(&path);

            // self.store.remove();

            if let Some(it) = self.store.iter_first() {
                loop {
                    let this_name = self
                        .store
                        .value(&it, Columns::Test as i32)
                        .get::<String>()
                        .unwrap();

                    if this_name == display_name {
                        println!("Remove {}", this_name);

                        self.store.remove(&it);
                    }

                    if !self.store.iter_next(&it) {
                        break;
                    }
                }
            } else {
                println!("No it???")
            }
        }
    }
}
