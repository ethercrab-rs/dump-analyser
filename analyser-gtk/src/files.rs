use gio::glib::{self, ToValue};
use gtk::prelude::*;

enum Columns {
    Test,
}

const DUMP_LIST_COL_TYPES: &[glib::Type] = &[glib::Type::STRING];

pub fn init_list(dump_tree: &mut gtk::TreeView) {
    let dump_list_store = gtk::ListStore::new(DUMP_LIST_COL_TYPES);

    dump_tree.set_model(Some(&dump_list_store));

    // Add a test column
    {
        let renderer = gtk::CellRendererText::new();
        let column = gtk::TreeViewColumn::new();
        TreeViewColumnExt::pack_start(&column, &renderer, true);
        column.set_title("Testing");
        TreeViewColumnExt::add_attribute(&column, &renderer, "text", Columns::Test as i32);
        column.set_sort_column_id(Columns::Test as i32);
        dump_tree.append_column(&column);
    }

    let values: [(u32, &dyn ToValue); 1] = [(0u32, &"Hello world")];

    dump_list_store.set(&dump_list_store.append(), &values);
}
