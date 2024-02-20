mod files;

use files::DumpFiles;
use futures::StreamExt;
use gio::glib;
use gtk::{gdk::EventMask, prelude::*};
use notify_debouncer_full::notify::event::{AccessKind, AccessMode, CreateKind, RemoveKind};
use notify_debouncer_full::notify::{Event, EventKind, RecursiveMode, Watcher};
use notify_debouncer_full::{DebounceEventResult, DebouncedEvent};
use plotters::prelude::*;
use plotters::style::full_palette;
use plotters_cairo::CairoBackend;
use std::cell::RefCell;
use std::collections::HashSet;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

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

fn build_ui(app: &gtk::Application) {
    let builder = gtk::Builder::from_string(GLADE_UI_SOURCE);
    let window = builder
        .object::<gtk::Window>("MainWindow")
        .expect("MainWindow");

    window.set_title("EtherCrab packet dump analyser");
    window.set_events(window.events() | EventMask::POINTER_MOTION_MASK);

    let app_state = Rc::new(RefCell::new(AppState {}));

    window.set_application(Some(app));

    let mut dump_tree = builder
        .object::<gtk::TreeView>("DumpTree")
        .expect("DumpTree");

    let mut cycle_delta_chart = builder
        .object::<gtk::DrawingArea>("CycleDeltaChart")
        .expect("CycleDeltaChart");

    let mut round_trip_chart = builder
        .object::<gtk::DrawingArea>("RoundTripChart")
        .expect("RoundTripChart");

    let dumps_path = Path::new("./dumps");

    let (tx, mut rx) = futures::channel::mpsc::unbounded();

    thread::spawn(move || {
        let (local_tx, local_rx) = std::sync::mpsc::channel();

        // let mut watcher = RecommendedWatcher::new(
        //     move |event| {
        //         println!("Watch event!");

        //         local_tx.send(event).expect("Local send");
        //     },
        //     notify_debouncer_full::notify::Config::default(),
        // )
        // .expect("Watcher");

        // watcher
        //     .watch(&PathBuf::from("./dumps"), RecursiveMode::Recursive)
        //     .expect("Start watch");

        let mut debouncer = notify_debouncer_full::new_debouncer(
            Duration::from_millis(500),
            None,
            move |result: DebounceEventResult| {
                println!("Got an event");

                local_tx.send(result).expect("Local tx");
            },
        )
        .unwrap();

        debouncer
            .watcher()
            .watch(&dumps_path, RecursiveMode::Recursive)
            .unwrap();

        while let Ok(event) = local_rx.recv() {
            tx.unbounded_send(event).expect("Send file event");
        }
    });

    let mut files = Arc::new(RwLock::new(DumpFiles::new(&dumps_path)));

    files.write().unwrap().init_view(&mut dump_tree);

    glib::MainContext::default().spawn_local(async move {
        println!("Start watch future");

        while let Some(Ok(events)) = rx.next().await {
            for event in events {
                match event {
                    DebouncedEvent {
                        event:
                            Event {
                                kind: EventKind::Create(CreateKind::File),
                                paths,
                                ..
                            },
                        ..
                    } => {
                        println!("Files created {:?}", paths);

                        files.write().unwrap().update_items(paths);
                    }
                    DebouncedEvent {
                        event:
                            Event {
                                kind: EventKind::Remove(RemoveKind::File),
                                paths,
                                ..
                            },
                        ..
                    } => {
                        println!("Files deleted {:?}", paths);

                        files.write().unwrap().remove_items(paths);
                    }

                    DebouncedEvent {
                        event:
                            Event {
                                kind: EventKind::Access(AccessKind::Close(AccessMode::Write)),
                                ..
                            },
                        ..
                    } => {
                        println!("Files updated {:?}", event.paths);
                    }
                    _other => println!("Other events"),
                }
            }
        }
    });

    let dump_selection = dump_tree.selection();
    dump_selection.set_mode(gtk::SelectionMode::Multiple);

    // dump_selection.connect_changed(move |selection| {
    //     println!("Selected");

    //     selection.selected_foreach(|model, _path, iter| {
    //         let test_value: String = model.value(&iter, 0).get_owned().expect("Not a string");

    //         println!("--> {}", test_value);
    //     });

    //     // if let Some((model, iter)) = selection.selected() {
    //     //     // let mut path = dump_store.path(&iter).expect("Couldn't get path");

    //     //     // dbg!(path);
    //     //     // // get the top-level element path
    //     //     // while path.depth() > 1 {
    //     //     //     path.up();
    //     //     // }

    //     //     while let Some(iter) = model.iter_first() {
    //     //         dbg!(dump_store.iter_is_valid(&iter));

    //     //         dbg!(model.value(&iter, 0).get_owned::<String>());

    //     //         model.iter_next(&iter);
    //     //     }
    //     // }
    // });

    // dump_tree.connect_row_deactivated(|tree, path, column| {
    //     println!("Deactivated");
    // });

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

    // let mut debouncer = new_debouncer(
    //     Duration::from_millis(10),
    //     None,
    //     |result: DebounceEventResult| match result {
    //         Ok(events) => events.iter().for_each(|event| println!("{event:?}")),
    //         Err(errors) => errors.iter().for_each(|error| println!("{error:?}")),
    //     },
    // )
    // .unwrap();

    // debouncer
    //     .watcher()
    //     .watch(Path::new("./dumps"), RecursiveMode::Recursive)
    //     .unwrap();

    // std::thread::sleep(Duration::from_secs(10));
}
