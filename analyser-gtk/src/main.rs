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
    files: DumpFiles,
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

    let dumps_path = Path::new("./dumps");

    let app_state = Rc::new(RefCell::new(AppState {
        files: DumpFiles::new(&dumps_path),
    }));

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

    cycle_delta_chart.set_events(cycle_delta_chart.events() | EventMask::POINTER_MOTION_MASK);
    cycle_delta_chart.connect_motion_notify_event(move |_widget, _cr| {
        // TODO: Find a way to get value from chart. This method is currently a noop but it was a
        // bit challenging to get it working so I'll leave it in.

        Inhibit(false)
    });

    round_trip_chart.set_events(round_trip_chart.events() | EventMask::POINTER_MOTION_MASK);
    round_trip_chart.connect_motion_notify_event(move |_widget, _cr| {
        // TODO: Find a way to get value from chart. This method is currently a noop but it was a
        // bit challenging to get it working so I'll leave it in.

        Inhibit(false)
    });

    // ---

    let (tx, mut rx) = futures::channel::mpsc::unbounded();

    thread::spawn(move || {
        let (local_tx, local_rx) = std::sync::mpsc::channel();

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

    app_state.borrow_mut().files.init_view(&mut dump_tree);

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

                        app_state.borrow_mut().files.update_items(paths);
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

                        app_state.borrow_mut().files.remove_items(paths);
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

    dump_selection.connect_changed(move |selection| {
        println!("Selected");

        selection.selected_foreach(|model, _path, iter| {
            let test_value: String = model
                .value(&iter, files::Columns::FullPath as i32)
                .get_owned()
                .expect("Not a string");

            println!("--> {}", test_value);
        });
    });

    window.show_all();
}

fn main() {
    let application = gtk::Application::new(Some("io.dump-analyser"), Default::default());

    application.connect_activate(|app| {
        build_ui(app);
    });

    application.run();
}
