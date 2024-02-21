use analyser_gtk::files::{self, DumpFiles};
use dump_analyser::PcapFile;
use futures::StreamExt;
use gio::glib;
use gtk::{gdk::EventMask, prelude::*};
use notify_debouncer_full::notify::event::{AccessKind, AccessMode, CreateKind, RemoveKind};
use notify_debouncer_full::notify::{Event, EventKind, RecursiveMode, Watcher};
use notify_debouncer_full::{DebounceEventResult, DebouncedEvent};
use plotters::prelude::*;
use plotters::style::full_palette::GREY_500;
use plotters_cairo::CairoBackend;
use std::cell::RefCell;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::thread;
use std::time::{Duration, Instant};

const GLADE_UI_SOURCE: &'static str = include_str!("ui.glade");

struct AppState {
    files: DumpFiles,
    cycle_delta_drag_start: Option<(f64, f64)>,
    cycle_delta_drag_end: Option<(f64, f64)>,
}

impl AppState {
    /// Plot packet TX/RX round trip time.
    fn plot_roundtrip_times<'a, DB: DrawingBackend + 'a>(
        &self,
        backend: DB,
    ) -> Result<(), Box<dyn Error + 'a>> {
        let root = backend.into_drawing_area();

        root.fill(&GREY_500)?;

        let mut max_points = 0;
        let mut min_delta = 0;
        let mut max_delta = 0;

        let mut series = Vec::new();

        for (colour_idx, file) in self.files.selected_paths().enumerate() {
            let start = Instant::now();

            let display_name = file.file_stem().unwrap().to_string_lossy().to_string();
            let pairs = PcapFile::new(file).match_tx_rx();

            max_points = max_points.max(pairs.len());

            series.push((
                LineSeries::new(
                    pairs.into_iter().enumerate().map(|(i, stat)| {
                        let t = stat.delta_time.as_nanos();

                        min_delta = min_delta.min(t);
                        max_delta = max_delta.max(t);

                        (i as f32, t as f32)
                    }),
                    Palette9999::pick(colour_idx),
                ),
                display_name.clone(),
                colour_idx,
            ));

            println!(
                "Processed {} in {} us",
                display_name,
                start.elapsed().as_micros()
            );
        }

        let mut chart = ChartBuilder::on(&root)
            .caption("Packet round trip times", ("sans-serif", 16).into_font())
            .margin(5)
            .x_label_area_size(30)
            .y_label_area_size(30)
            .build_cartesian_2d(0.0..max_points as f32, min_delta as f32..max_delta as f32)?;

        chart
            .configure_mesh()
            .max_light_lines(0)
            .x_desc("Packet number")
            .y_desc("Packet round trip time (ns)")
            .draw()?;

        let start = Instant::now();

        for (s, label, colour_idx) in series.into_iter() {
            chart.draw_series(s)?.label(&label).legend(move |(x, y)| {
                let c = Palette9999::pick(colour_idx);

                Rectangle::new([(x, y + 1), (x + 8, y)], c)
            });
        }

        println!("Drew series in {} us", start.elapsed().as_micros());

        let start = Instant::now();

        chart
            .configure_series_labels()
            .position(SeriesLabelPosition::UpperRight)
            .border_style(&BLACK)
            .draw()?;

        println!("Drew chart in {} us", start.elapsed().as_micros());

        let start = Instant::now();

        root.present()?;

        println!("Present in {} us", start.elapsed().as_micros());

        Ok(())
    }

    /// Plot previous packet to current packet TX delta.
    fn plot_cycle_delta<'a, DB: DrawingBackend + 'a>(
        &mut self,
        backend: DB,
    ) -> Result<(), Box<dyn Error + 'a>> {
        let root = backend.into_drawing_area();

        root.fill(&GREY_500)?;

        let mut max_points = 0;
        let mut min_delta = 0;
        let mut max_delta = 0;

        let mut series = Vec::new();

        for (colour_idx, file) in self.files.selected_paths().enumerate() {
            let start = Instant::now();

            let display_name = file.file_stem().unwrap().to_string_lossy().to_string();
            let pairs = PcapFile::new(file).match_tx_rx();

            max_points = max_points.max(pairs.len());

            series.push((
                LineSeries::new(
                    pairs.windows(2).into_iter().enumerate().map(|(i, stats)| {
                        let [prev, curr] = stats else { unreachable!() };

                        let t = curr.tx_time.as_nanos() - prev.tx_time.as_nanos();

                        min_delta = min_delta.min(t);
                        max_delta = max_delta.max(t);

                        (i as f32, t as f32)
                    }),
                    Palette9999::pick(colour_idx),
                ),
                display_name.clone(),
                colour_idx,
            ));

            println!(
                "Processed cycle delta {} in {} us",
                display_name,
                start.elapsed().as_micros()
            );
        }

        let mut chart = ChartBuilder::on(&root)
            .caption(
                "Cycle to cycle packet TX delta",
                ("sans-serif", 16).into_font(),
            )
            .margin(5)
            .x_label_area_size(40)
            .y_label_area_size(30)
            .build_cartesian_2d(0.0..max_points as f32, min_delta as f32..max_delta as f32)?;

        chart
            .configure_mesh()
            .max_light_lines(0)
            .x_desc("Packet number")
            .y_desc("Cycle-cycle delta (ns)")
            .draw()?;

        if let Some((start, end)) = self
            .cycle_delta_drag_start
            .take()
            .zip(self.cycle_delta_drag_end.take())
        {
            println!("{:?} -> {:?}", start, end);
        }

        for (s, label, colour_idx) in series {
            chart
                .draw_series(s)?
                .label(&label)
                // TODO: Pass palette through
                .legend(move |(x, y)| {
                    let c = Palette9999::pick(colour_idx);

                    Rectangle::new([(x, y + 1), (x + 8, y)], c)
                });
        }

        chart
            .configure_series_labels()
            .position(SeriesLabelPosition::UpperRight)
            .border_style(&BLACK)
            .draw()?;

        root.present()?;

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
        cycle_delta_drag_start: None,
        cycle_delta_drag_end: None,
    }));

    window.set_application(Some(app));

    let mut dump_tree = builder
        .object::<gtk::TreeView>("DumpTree")
        .expect("DumpTree");

    let cycle_delta_chart = builder
        .object::<gtk::DrawingArea>("CycleDeltaChart")
        .expect("CycleDeltaChart");

    let round_trip_chart = builder
        .object::<gtk::DrawingArea>("RoundTripChart")
        .expect("RoundTripChart");

    cycle_delta_chart.set_events(
        cycle_delta_chart.events()
            | EventMask::POINTER_MOTION_MASK
            | EventMask::BUTTON_PRESS_MASK
            | EventMask::BUTTON_RELEASE_MASK,
    );
    cycle_delta_chart.connect_motion_notify_event(move |_widget, _cr| {
        // NOTE: Unused, but was challenging to get working so I'll leave this handler in.

        Inhibit(false)
    });
    let state_cloned = app_state.clone();
    cycle_delta_chart.connect_button_press_event(move |_, cr| {
        let mut state = state_cloned.borrow_mut();

        // cr.position() is relative to drawing area
        state.cycle_delta_drag_start = Some(cr.position());

        println!("Drag begin, {:?}", cr.position());

        Inhibit(false)
    });
    let state_cloned = app_state.clone();
    cycle_delta_chart.connect_button_release_event(move |widget, cr| {
        let mut state = state_cloned.borrow_mut();

        // cr.position() is relative to drawing area
        state.cycle_delta_drag_end = Some(cr.position());

        println!("Drag end, {:?}", cr.position());

        // let w = widget.allocated_width();
        // let h = widget.allocated_height();

        // let backend = CairoBackend::new(cr, (w as u32, h as u32)).unwrap();

        // state.plot_cycle_delta(backend).unwrap();

        widget.queue_draw();

        Inhibit(false)
    });

    round_trip_chart.set_events(round_trip_chart.events() | EventMask::POINTER_MOTION_MASK);
    round_trip_chart.connect_motion_notify_event(move |_widget, _cr| {
        // TODO: Find a way to get value from chart. This method is currently a noop but it was a
        // bit challenging to get it working so I'll leave it in.

        Inhibit(false)
    });

    // ---

    let state_cloned = app_state.clone();
    round_trip_chart.connect_draw(move |widget, cr| {
        let state = state_cloned.borrow();

        let w = widget.allocated_width();
        let h = widget.allocated_height();

        let backend = CairoBackend::new(cr, (w as u32, h as u32)).unwrap();

        state.plot_roundtrip_times(backend).unwrap();

        Inhibit(false)
    });

    let state_cloned = app_state.clone();
    cycle_delta_chart.connect_draw(move |widget, cr| {
        let mut state = state_cloned.borrow_mut();

        let w = widget.allocated_width();
        let h = widget.allocated_height();

        let backend = CairoBackend::new(cr, (w as u32, h as u32)).unwrap();

        state.plot_cycle_delta(backend).unwrap();

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

    let state_cloned = app_state.clone();
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

                        state_cloned.borrow_mut().files.update_items(paths);
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

                        state_cloned.borrow_mut().files.remove_items(paths);
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

    let state_cloned = app_state.clone();
    dump_selection.connect_changed(move |selection| {
        let mut selected = Vec::new();

        selection.selected_foreach(|model, _path, iter| {
            let path = PathBuf::from(
                model
                    .value(&iter, files::Columns::FullPath as i32)
                    .get_owned::<String>()
                    .expect("Not a string"),
            );

            selected.push(path);
        });

        state_cloned.borrow_mut().files.update_selection(selected);

        cycle_delta_chart.queue_draw();
        round_trip_chart.queue_draw();
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
