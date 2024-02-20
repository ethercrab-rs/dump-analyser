use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;

use gio::glib;
use gtk::{gdk::EventMask, prelude::*};
use plotters::prelude::*;
use plotters::style::full_palette;
use plotters_cairo::CairoBackend;

const GLADE_UI_SOURCE: &'static str = include_str!("ui.glade");

struct AppState {
    //
}

impl AppState {
    fn draw_charts<'a, DB: DrawingBackend + 'a>(
        &self,
        backend: DB,
    ) -> Result<(), Box<dyn Error + 'a>> {
        //

        // root.present()?;
        Ok(())
    }
}

enum Columns {
    Test,
}

fn build_ui(app: &gtk::Application) {
    let builder = gtk::Builder::from_string(GLADE_UI_SOURCE);
    let window = builder.object::<gtk::Window>("MainWindow").unwrap();

    window.set_title("EtherCrab packet dump analyser");
    window.set_events(window.events() | EventMask::POINTER_MOTION_MASK);

    let app_state = Rc::new(RefCell::new(AppState {}));

    window.set_application(Some(app));

    let dump_list_col_types = [glib::Type::STRING];

    let dump_list_store = gtk::ListStore::new(&dump_list_col_types);

    let dump_tree = builder
        .object::<gtk::TreeView>("DumpTree")
        .expect("DumpTree");

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

    // let state_cloned = app_state.clone();
    // drawing_area.connect_draw(move |widget, cr| {
    //     let state = state_cloned.borrow();
    //     let w = widget.allocated_width();
    //     let h = widget.allocated_height();
    //     let backend = CairoBackend::new(cr, (w as u32, h as u32)).unwrap();
    //     state.plot_pdf(backend).unwrap();
    //     Inhibit(false)
    // });

    // // let state_cloned = app_state.clone();
    // drawing_area.set_events(drawing_area.events() | EventMask::POINTER_MOTION_MASK);
    // drawing_area.connect_motion_notify_event(move |_widget, _cr| {
    //     // TODO: Find a way to get value from chart. This method is currently a noop but it was a
    //     // bit challenging to get it working so I'll leave it in.

    //     Inhibit(false)
    // });

    // let state_cloned = app_state.clone();
    // times.connect_draw(move |widget, _cr| {
    //     let app_state = state_cloned.borrow();

    //     let times = app_state.seg.times();

    //     widget.set_text(&format!(
    //         "Total {:>5}, t_j1 {:>5}, t_a {:>5}, t_v {:>5}, t_j2 {:>5}, t_d {:>5}",
    //         times.total_time, times.t_j1, times.t_a, times.t_v, times.t_j2, times.t_d
    //     ));

    //     Inhibit(false)
    // });

    // let handle_change =
    //     |what: &gtk::Scale, how: Box<dyn Fn(&mut PlottingState) -> &mut f64 + 'static>| {
    //         let app_state = app_state.clone();
    //         let drawing_area = drawing_area.clone();
    //         let times = times.clone();
    //         what.connect_value_changed(move |target| {
    //             let mut state = app_state.borrow_mut();
    //             *how(&mut *state) = target.value();

    //             state.seg = Segment::new(
    //                 state.q0 as f32,
    //                 state.q1 as f32,
    //                 state.v0 as f32,
    //                 state.v1 as f32,
    //                 &Lim {
    //                     vel: state.lim_vel as f32,
    //                     acc: state.lim_acc as f32,
    //                 },
    //             );

    //             drawing_area.queue_draw();
    //             times.queue_draw();
    //         });

    //         // Reset to 0 on double click
    //         what.connect_button_press_event(move |target, event| {
    //             if event.button() == 1 && event.click_count() == Some(2) {
    //                 target.set_value(0.0);
    //             }

    //             Inhibit(false)
    //         })
    //     };

    // let handle_bool_change =
    //     |what: &gtk::ToggleButton, how: Box<dyn Fn(&mut PlottingState) -> &mut bool + 'static>| {
    //         let app_state = app_state.clone();
    //         let drawing_area = drawing_area.clone();
    //         let times = times.clone();
    //         what.connect_toggled(move |target| {
    //             let mut state = app_state.borrow_mut();
    //             *how(&mut *state) = target.is_active();
    //             drawing_area.queue_draw();
    //             times.queue_draw();
    //         });
    //     };

    // handle_change(&q0_scale, Box::new(|s| &mut s.q0));
    // handle_change(&q1_scale, Box::new(|s| &mut s.q1));
    // handle_change(&v0_scale, Box::new(|s| &mut s.v0));
    // handle_change(&v1_scale, Box::new(|s| &mut s.v1));
    // handle_change(&lim_vel_scale, Box::new(|s| &mut s.lim_vel));
    // handle_change(&lim_acc_scale, Box::new(|s| &mut s.lim_acc));
    // handle_change(&lim_jerk_scale, Box::new(|s| &mut s.lim_jerk));
    // handle_bool_change(&show_pos, Box::new(|s| &mut s.show_pos));
    // handle_bool_change(&show_vel, Box::new(|s| &mut s.show_vel));
    // handle_bool_change(&show_acc, Box::new(|s| &mut s.show_acc));
    // handle_bool_change(&show_jerk, Box::new(|s| &mut s.show_jerk));

    window.show_all();
}

fn main() {
    let application = gtk::Application::new(Some("io.dump-analyser"), Default::default());

    application.connect_activate(|app| {
        build_ui(app);
    });

    application.run();
}