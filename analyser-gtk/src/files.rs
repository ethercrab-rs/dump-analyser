use gio::glib::{self};
use gtk::prelude::*;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

#[repr(u16)]
pub enum Columns {
    FullPath,
    Test,
}

const DUMP_LIST_COL_TYPES: &[glib::Type] = &[glib::Type::STRING, glib::Type::STRING];

struct Item {
    // TODO
}

pub struct DumpFiles {
    store: gtk::ListStore,
    names: HashMap<PathBuf, Item>,
}

impl DumpFiles {
    pub fn new(path: &Path) -> Self {
        assert!(path.is_dir(), "Path must point to a directory");

        let mut self_ = Self {
            store: gtk::ListStore::new(DUMP_LIST_COL_TYPES),
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

    /// Only call this once.
    pub fn init_view(&self, tree: &mut gtk::TreeView) {
        tree.set_model(Some(&self.store));

        // Full path, not visible but use to retrieve values
        {
            // let renderer = gtk::CellRendererText::new();
            let column = gtk::TreeViewColumn::new();
            // TreeViewColumnExt::pack_start(&column, &renderer, true);
            column.set_title("File path");
            // TreeViewColumnExt::add_attribute(&column, &renderer, "text", Columns::FullPath as i32);
            column.set_sort_column_id(Columns::FullPath as i32);
            column.set_visible(false);
            tree.append_column(&column);
        }

        // File stem, aka display name
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
        let paths = paths
            .into_iter()
            .filter(|path| {
                path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("pcapng")
            })
            .collect::<HashSet<_>>();

        let n2 = self.names.keys().cloned().collect::<HashSet<_>>();

        let new = paths.difference(&n2);

        for path in new.into_iter() {
            let p = path.file_stem().to_owned().unwrap().to_string_lossy();

            let display_name = p.as_ref();

            self.names.insert(path.clone(), Item {});

            self.store.set(
                &self.store.append(),
                &[
                    (Columns::FullPath as u32, &path),
                    (Columns::Test as u32, &display_name),
                ],
            );
        }
    }

    pub fn remove_items(&mut self, remove: Vec<PathBuf>) {
        for path in remove.into_iter() {
            let p = path.file_stem().to_owned().unwrap().to_string_lossy();

            let display_name = p.as_ref();

            self.names.remove(&path);

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
