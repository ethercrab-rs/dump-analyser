use analyser_gui::files::DumpFiles;
use dump_analyser::PcapFile;
use eframe::egui;
use egui::{TextStyle, Ui};
use egui_extras::{Column, TableBuilder};
use egui_extras::{Size, StripBuilder};
use egui_plot::{Legend, Line, Plot, PlotPoints};
use notify_debouncer_full::{
    notify::{
        event::{AccessKind, AccessMode, CreateKind, RemoveKind},
        Event, EventKind, RecursiveMode, Watcher,
    },
    DebounceEventResult, DebouncedEvent,
};
use parking_lot::RwLock;
use std::{path::PathBuf, sync::Arc, thread, time::Duration};

struct MyApp {
    round_trip_times: Arc<RwLock<Vec<(String, Vec<[f64; 2]>)>>>,
    cycle_delta_times: Arc<RwLock<Vec<(String, Vec<[f64; 2]>)>>>,
    // prev_bounds: Option<PlotBounds>,
    files: Arc<RwLock<DumpFiles>>,
    // rx: tokio::sync::mpsc::UnboundedReceiver<DebounceEventResult>,
}

impl MyApp {
    fn file_list(&mut self, ui: &mut Ui) {
        let table = TableBuilder::new(ui)
            .striped(false)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto())
            // Name is widest column
            .column(Column::remainder())
            .min_scrolled_height(0.0)
            .sense(egui::Sense::click());

        // if let Some(row_index) = self.scroll_to_row.take() {
        //     table = table.scroll_to_row(row_index, None);
        // }

        table
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("#");
                });
                header.col(|ui| {
                    ui.strong("File");
                });
            })
            .body(|mut body| {
                // Gotta clone to prevent deadlocks
                let files = self.files.read().clone();

                for (row_index, file) in files.all().iter().enumerate() {
                    body.row(18.0, |mut row| {
                        row.set_selected(file.selected);

                        row.col(|ui| {
                            ui.label(row_index.to_string());
                        });
                        row.col(|ui| {
                            ui.label(&file.display_name);
                        });

                        if row.response().clicked() {
                            self.files.write().toggle_selection(&file.path);

                            self.recompute_plots();
                        }
                    });
                }
            });
    }

    fn recompute_plots(&self) {
        let files = self.files.read();

        //

        let mut new = Vec::new();
        let mut new_cycle_deltas = Vec::new();

        for selected_file in files.selected_paths() {
            let pairs = PcapFile::new(&selected_file.path).match_tx_rx();

            let roundtrip_times = (
                selected_file.display_name.clone(),
                pairs
                    .iter()
                    .enumerate()
                    .map(|(i, item)| [i as f64, item.delta_time.as_nanos() as f64 / 1000.0])
                    .collect(),
            );

            let cycle_delta_times = (
                selected_file.display_name.clone(),
                pairs
                    .windows(2)
                    .into_iter()
                    .enumerate()
                    .map(|(i, stats)| {
                        let [prev, curr] = stats else { unreachable!() };

                        let t = curr.tx_time.as_nanos() - prev.tx_time.as_nanos();

                        [i as f64, t as f64 / 1000.0]
                    })
                    .collect(),
            );

            new.push(roundtrip_times);
            new_cycle_deltas.push(cycle_delta_times);
        }

        *self.round_trip_times.write() = new;
        *self.cycle_delta_times.write() = new_cycle_deltas;
    }

    fn compute_bounds(&self, plot_ui: &mut egui_plot::PlotUi) -> (usize, usize, usize) {
        // Bounds of the plot by data values, not pixels
        let plot_bounds = plot_ui.plot_bounds();

        let (start_count, end_count) = if plot_bounds.min()[0] <= 0.0 {
            (
                0usize,
                self.round_trip_times
                    .read()
                    .iter()
                    .map(|(_name, points)| points.len())
                    .max()
                    .unwrap_or(0),
            )
        } else {
            (plot_bounds.min()[0] as usize, plot_bounds.max()[0] as usize)
        };

        let values_width = plot_bounds.width();

        let pixels_width = {
            plot_ui.screen_from_plot(plot_bounds.max().into())[0]
                - plot_ui.screen_from_plot(plot_bounds.min().into())[0]
        } as f64;

        let stride = (values_width / pixels_width).max(1.0) as usize;

        (start_count, end_count, stride)
    }

    fn aggregate(
        &self,
        (start_count, end_count, stride): (usize, usize, usize),
        series: &[[f64; 2]],
    ) -> Vec<[f64; 2]> {
        let display_range = start_count..end_count.min(series.len());

        series[display_range]
            .chunks(stride)
            .into_iter()
            .map(|chunk| {
                let ys = chunk.iter().map(|[_x, y]| *y);
                let xs = chunk.iter().map(|[x, _y]| *x);

                // Put X coord in middle of chunk
                let x = xs.clone().sum::<f64>() / stride as f64;

                [
                    [
                        x,
                        ys.clone()
                            .min_by(|a, b| (*a as u32).cmp(&(*b as u32)))
                            .unwrap(),
                    ],
                    [x, ys.max_by(|a, b| (*a as u32).cmp(&(*b as u32))).unwrap()],
                ]
            })
            .flatten()
            .collect::<Vec<_>>()
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("left_panel")
            // .resizable(true)
            .default_width(200.0)
            // .width_range(200.0..=500.0)
            .show(ctx, |ui| {
                // ui.vertical_centered(|ui| {
                ui.heading("Captures");
                // });

                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.file_list(ui);
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            // if ui.button("Save Plot").clicked() {
            //     ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
            // }

            let heading_text_size = TextStyle::Heading.resolve(ui.style()).size;

            StripBuilder::new(ui)
                .size(Size::remainder())
                .size(Size::remainder())
                .vertical(|mut strip| {
                    strip.cell(|ui| {
                        StripBuilder::new(ui)
                            // Heading
                            .size(Size::exact(heading_text_size))
                            // Chart
                            .size(Size::remainder())
                            .vertical(|mut strip| {
                                strip.cell(|ui| {
                                    ui.heading("Packet round trip times (us)");
                                });
                                strip.cell(|ui| {
                                    Plot::new("round_trips")
                                        .x_axis_label("Packet number")
                                        .y_axis_label("TX/RX round trip time (us)")
                                        .legend(Legend::default())
                                        .show(ui, |plot_ui| {
                                            let bounds = self.compute_bounds(plot_ui);

                                            for (name, series) in
                                                self.round_trip_times.read().iter()
                                            {
                                                let points = self.aggregate(bounds, series);

                                                plot_ui.line(
                                                    Line::new(PlotPoints::new(points)).name(name),
                                                );
                                            }
                                        });
                                });
                            });
                    });
                    strip.cell(|ui| {
                        StripBuilder::new(ui)
                            // Heading
                            .size(Size::exact(heading_text_size))
                            // Chart
                            .size(Size::remainder())
                            .vertical(|mut strip| {
                                strip.cell(|ui| {
                                    ui.heading("Cycle-cycle delta (us)");
                                });
                                strip.cell(|ui| {
                                    Plot::new("cycle_delta")
                                        .x_axis_label("Packet number")
                                        .y_axis_label("Cycle to cycle delta time (us)")
                                        .legend(Legend::default())
                                        .show(ui, |plot_ui| {
                                            let bounds = self.compute_bounds(plot_ui);

                                            for (name, series) in
                                                self.cycle_delta_times.read().iter()
                                            {
                                                let points = self.aggregate(bounds, series);

                                                plot_ui.line(
                                                    Line::new(PlotPoints::new(points)).name(name),
                                                );
                                            }
                                        });
                                });
                            });
                    });
                });
        });
    }
}

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1920.0, 1080.0])
            .with_min_inner_size([1280.0, 720.0]),
        ..Default::default()
    };

    let dumps_path = PathBuf::from("./dumps");

    eframe::run_native(
        "eframe template",
        native_options,
        Box::new(|cc| {
            let ctx = cc.egui_ctx.clone();

            let p = dumps_path.clone();

            let files = Arc::new(parking_lot::RwLock::new(DumpFiles::new(dumps_path)));
            let round_trip_times = Arc::new(parking_lot::RwLock::new(Vec::new()));
            let cycle_delta_times = Arc::new(parking_lot::RwLock::new(Vec::new()));

            let f2 = files.clone();

            thread::spawn(move || {
                let files = f2.clone();

                let (local_tx, local_rx) = std::sync::mpsc::channel();

                let mut debouncer = notify_debouncer_full::new_debouncer(
                    Duration::from_millis(500),
                    None,
                    move |result: DebounceEventResult| {
                        // println!("Got an event");

                        local_tx.send(result).expect("Local tx");
                    },
                )
                .unwrap();

                debouncer
                    .watcher()
                    .watch(&p, RecursiveMode::Recursive)
                    .unwrap();

                while let Ok(Ok(events)) = local_rx.recv() {
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

                                files.write().update_items(paths);
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

                                files.write().remove_items(paths);
                            }

                            DebouncedEvent {
                                event:
                                    Event {
                                        kind:
                                            EventKind::Access(AccessKind::Close(AccessMode::Write)),
                                        ..
                                    },
                                ..
                            } => {
                                println!("Files updated {:?}", event.paths);
                            }
                            _other => println!("Other events"),
                        }
                    }

                    ctx.request_repaint();
                }
            });

            Box::new(MyApp {
                files,
                round_trip_times,
                cycle_delta_times,
            })
        }),
    )
}
