use gio::glib::{self, ToValue};
use gtk::prelude::*;

enum Columns {
    Test,
}

const DUMP_LIST_COL_TYPES: &[glib::Type] = &[glib::Type::STRING];

pub struct DumpFiles {
    store: gtk::ListStore,
}

impl DumpFiles {
    pub fn new() -> Self {
        Self {
            store: gtk::ListStore::new(DUMP_LIST_COL_TYPES),
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

    // TODO: Item type that isn't just `String`.
    pub fn add_item(&self, item: String) {
        self.store
            .set(&self.store.append(), &[(0u32, &"Hello world")]);
    }
}
