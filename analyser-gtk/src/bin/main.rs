use analyser_gtk::files::DumpFiles;
use dump_analyser::PcapFile;
use eframe::egui;
use egui::Ui;
use egui_extras::{Column, TableBuilder};
use egui_plot::{Legend, Line, Plot, PlotBounds, PlotPoints};
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
    plots: Arc<RwLock<Vec<Vec<[f64; 2]>>>>,
    prev_bounds: Option<PlotBounds>,
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
                for (row_index, file) in self.files.read().all().iter().enumerate() {
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

        let mut new = Vec::new();

        for selected_file in files.selected_paths() {
            // TODO: Read pcap files on startup/when added?
            let pairs = PcapFile::new(selected_file).match_tx_rx();

            let roundtrip_times = pairs
                .iter()
                .enumerate()
                .map(|(i, item)| [i as f64, item.delta_time.as_nanos() as f64])
                .collect();

            new.push(roundtrip_times);
        }

        *self.plots.write() = new;
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

            let my_plot = Plot::new("My Plot").legend(Legend::default());

            // Perf: https://github.com/emilk/egui/pull/3849
            my_plot.show(ui, |plot_ui| {
                let plot_bounds = plot_ui.plot_bounds();

                // TODO: Loop through series, calculate bounds properly.

                let points = if let Some(_) = self.prev_bounds {
                    let start_count = plot_bounds.min()[0] as usize;
                    let end_count = (plot_bounds.max()[0] as usize).min(self.plots.read().len());

                    let display_range = start_count..end_count;

                    let values_width = plot_bounds.width();

                    let pixels_width = {
                        plot_ui.screen_from_plot(plot_bounds.max().into())[0]
                            - plot_ui.screen_from_plot(plot_bounds.min().into())[0]
                    } as f64;

                    let stride = (values_width / pixels_width).max(1.0) as usize;

                    self.plots.read()[0][display_range]
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
                } else {
                    self.plots.read()[0].clone()
                };

                plot_ui.line(Line::new(PlotPoints::new(points)).name("curve"));

                self.prev_bounds = Some(plot_bounds);
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

    let graph = PcapFile::new(&PathBuf::from("./dumps/smol-io-uring-new.pcapng"))
        .match_tx_rx()
        .into_iter()
        .enumerate()
        .map(|(i, stat)| [i as f64, stat.delta_time.as_nanos() as f64 / 1000.0])
        .collect::<Vec<_>>();

    let dumps_path = PathBuf::from("./dumps");

    eframe::run_native(
        "eframe template",
        native_options,
        Box::new(|cc| {
            let ctx = cc.egui_ctx.clone();

            let p = dumps_path.clone();

            let files = Arc::new(parking_lot::RwLock::new(DumpFiles::new(dumps_path)));
            let plots = Arc::new(parking_lot::RwLock::new(Vec::new()));

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
                plots,
                prev_bounds: None,
                // rx,
            })
        }),
    )
}
