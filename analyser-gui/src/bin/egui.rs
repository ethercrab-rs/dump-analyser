use analyser_gui::files::{DumpFile, DumpFiles};
use eframe::egui;
use egui::epaint::Hsva;
use egui::{Color32, TextStyle, Ui};
use egui_extras::{Column, TableBuilder};
use egui_extras::{Size, StripBuilder};
use egui_plot::{Legend, Line, LineStyle, Plot, PlotPoints, VLine};
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
    files: Arc<RwLock<DumpFiles>>,
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
                let names = self
                    .files
                    .read_arc_recursive()
                    .all()
                    .iter()
                    .map(|f| (f.path.clone(), f.selected, f.display_name.clone()))
                    .collect::<Vec<_>>();

                for (row_index, (path, selected, file)) in names.into_iter().enumerate() {
                    body.row(18.0, |mut row| {
                        row.set_selected(selected);

                        row.col(|ui| {
                            ui.label(row_index.to_string());
                        });
                        row.col(|ui| {
                            ui.label(&file);
                        });

                        if row.response().clicked() {
                            self.files
                                .try_write_arc()
                                .expect("Write locked")
                                .toggle_selection(&path);
                        }
                    });
                }
            });
    }

    fn round_trip_stats_list(&mut self, ui: &mut Ui, selected_files: &[&DumpFile]) {
        ui.heading("TX/RX statistics");

        egui::ScrollArea::vertical().show(ui, |ui| {
            let table = TableBuilder::new(ui)
                .striped(false)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::auto().at_least(350.0))
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .min_scrolled_height(0.0);
            // .sense(egui::Sense::click());

            table
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("File");
                    });
                    header.col(|ui| {
                        ui.strong("Std. Dev.");
                    });
                    header.col(|ui| {
                        ui.strong("Variance");
                    });

                    header.col(|ui| {
                        ui.strong("P25");
                    });
                    header.col(|ui| {
                        ui.strong("P50");
                    });
                    header.col(|ui| {
                        ui.strong("P90");
                    });
                    header.col(|ui| {
                        ui.strong("P99");
                    });

                    header.col(|ui| {
                        ui.strong("Min");
                    });
                    header.col(|ui| {
                        ui.strong("Mean");
                    });
                    header.col(|ui| {
                        ui.strong("Max");
                    });
                })
                .body(|mut body| {
                    for (idx, item) in selected_files.iter().enumerate() {
                        let c = idx_to_colour(idx);

                        body.row(18.0, |mut row| {
                            row.col(|ui| {
                                ui.colored_label(c, &item.display_name);
                            });

                            row.col(|ui| {
                                ui.colored_label(
                                    c,
                                    format!("{:.3} us", item.round_trip_stats.std_dev),
                                );
                            });

                            row.col(|ui| {
                                ui.colored_label(
                                    c,
                                    format!("{:.3} us", item.round_trip_stats.variance),
                                );
                            });

                            row.col(|ui| {
                                ui.colored_label(c, format!("{:.3} us", item.round_trip_stats.p25));
                            });
                            row.col(|ui| {
                                ui.colored_label(c, format!("{:.3} us", item.round_trip_stats.p50));
                            });
                            row.col(|ui| {
                                ui.colored_label(c, format!("{:.3} us", item.round_trip_stats.p90));
                            });
                            row.col(|ui| {
                                ui.colored_label(c, format!("{:.3} us", item.round_trip_stats.p99));
                            });

                            row.col(|ui| {
                                ui.colored_label(c, format!("{:.3} us", item.round_trip_stats.min));
                            });
                            row.col(|ui| {
                                ui.colored_label(
                                    c,
                                    format!("{:.3} us", item.round_trip_stats.mean),
                                );
                            });
                            row.col(|ui| {
                                ui.colored_label(c, format!("{:.3} us", item.round_trip_stats.max));
                            });

                            // if row.response().clicked() {
                            //     self.files.write().toggle_selection(&file.path);

                            //     self.recompute_plots();
                            // }
                        });
                    }
                });
        });
    }

    /// Returns `(start count, end count, stride)`. Used for showing a subset of some data on the
    /// graph to improve performance.
    fn compute_bounds(&self, plot_ui: &mut egui_plot::PlotUi) -> (usize, usize, usize) {
        // Bounds of the plot by data values, not pixels
        let plot_bounds = plot_ui.plot_bounds();

        let (start_count, end_count) = if plot_bounds.min()[0] <= 0.0 {
            (
                0usize,
                self.files
                    .try_read_recursive_arc()
                    .expect("Compute bounds")
                    .selected_paths()
                    .map(|item| item.num_points)
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

    /// Take a series of points and filter them down to a subset where:
    ///
    /// - Only visible points are shown.
    /// - If the data is dense enough that multiple points span a single pixel, two points (min,
    ///   max) are created for that pixel.
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
                let x = xs.clone().sum::<f64>() / chunk.len() as f64;

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
                .size(Size::remainder())
                .vertical(|mut strip| {
                    let borrow = self.files.read_arc_recursive();

                    let files = borrow.selected_paths().collect::<Vec<_>>();

                    strip.cell(|ui| {
                        self.round_trip_stats_list(ui, &files);
                    });

                    // TX/RX round trip time
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
                                    StripBuilder::new(ui)
                                        .size(Size::relative(0.6))
                                        .size(Size::remainder())
                                        .horizontal(|mut strip| {
                                            strip.cell(|ui| {
                                                Plot::new("round_trips")
                                                    .x_axis_label("Packet number")
                                                    .y_axis_label("TX/RX round trip time (us)")
                                                    .legend(Legend::default())
                                                    .show(ui, |plot_ui| {
                                                        let bounds = self.compute_bounds(plot_ui);

                                                        for (idx, item) in files.iter().enumerate()
                                                        {
                                                            let points = self.aggregate(
                                                                bounds,
                                                                &item.round_trip_times,
                                                            );

                                                            plot_ui.line(
                                                                Line::new(PlotPoints::new(points))
                                                                    .color(idx_to_colour(idx))
                                                                    .name(&item.display_name),
                                                            );
                                                        }
                                                    });
                                            });

                                            strip.cell(|ui| {
                                                Plot::new("round_trips_histo")
                                                    // Y is just a count of the bucket, so is meaningless
                                                    .show_y(false)
                                                    .y_axis_formatter(|_, _, _| String::new())
                                                    .x_axis_label("Round trip time (us)")
                                                    .legend(Legend::default())
                                                    .show(ui, |plot_ui| {
                                                        for (idx, item) in files.iter().enumerate()
                                                        {
                                                            let c = idx_to_colour(idx);

                                                            let points = item
                                                                .round_trip_histo
                                                                .iter_all()
                                                                .enumerate()
                                                                .map(|(idx, bucket)| {
                                                                    [
                                                                        idx as f64,
                                                                        bucket.count_at_value()
                                                                            as f64,
                                                                    ]
                                                                })
                                                                .collect::<Vec<_>>();

                                                            // Mean
                                                            plot_ui.vline(
                                                                VLine::new(
                                                                    item.round_trip_stats.mean,
                                                                )
                                                                .style(LineStyle::dashed_dense())
                                                                .color(
                                                                    c
                                                                        // 50% alpha
                                                                        .gamma_multiply(0.75),
                                                                ),
                                                            );

                                                            // Std dev
                                                            plot_ui.vline(
                                                                VLine::new(
                                                                    item.round_trip_stats.mean
                                                                        - item
                                                                            .round_trip_stats
                                                                            .std_dev
                                                                            / 2.0,
                                                                )
                                                                .style(LineStyle::dashed_dense())
                                                                .color(c.gamma_multiply(0.5)),
                                                            );
                                                            plot_ui.vline(
                                                                VLine::new(
                                                                    item.round_trip_stats.mean
                                                                        + item
                                                                            .round_trip_stats
                                                                            .std_dev
                                                                            / 2.0,
                                                                )
                                                                .style(LineStyle::dashed_dense())
                                                                .color(c.gamma_multiply(0.5)),
                                                            );

                                                            plot_ui.line(
                                                                Line::new(PlotPoints::new(points))
                                                                    .color(c)
                                                                    .name(&item.display_name),
                                                            );
                                                        }
                                                    });
                                            });
                                        });
                                });
                            });
                    });
                    // Cycle to cycle delta
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
                                    StripBuilder::new(ui)
                                        .size(Size::relative(0.6))
                                        .size(Size::remainder())
                                        .horizontal(|mut strip| {
                                            strip.cell(|ui| {
                                                Plot::new("cycle_delta")
                                                    .x_axis_label("Packet number")
                                                    .y_axis_label("Cycle to cycle delta time (us)")
                                                    .legend(Legend::default())
                                                    .show(ui, |plot_ui| {
                                                        let bounds = self.compute_bounds(plot_ui);

                                                        for (idx, item) in files.iter().enumerate()
                                                        {
                                                            let points = self.aggregate(
                                                                bounds,
                                                                &item.cycle_delta_times,
                                                            );

                                                            plot_ui.line(
                                                                Line::new(PlotPoints::new(points))
                                                                    .color(idx_to_colour(idx))
                                                                    .name(&item.display_name),
                                                            );
                                                        }
                                                    });
                                            });

                                            strip.cell(|ui| {
                                                Plot::new("cycle_delta_histo")
                                                    // Y is just a count of the bucket, so is meaningless
                                                    .show_y(false)
                                                    .y_axis_formatter(|_, _, _| String::new())
                                                    .x_axis_label("Cycle to cycle delta (us)")
                                                    .legend(Legend::default())
                                                    .show(ui, |plot_ui| {
                                                        for (idx, item) in files.iter().enumerate()
                                                        {
                                                            let c = idx_to_colour(idx);

                                                            let points: Vec<[f64; 2]> = item
                                                                .cycle_delta_histo
                                                                .iter_all()
                                                                .enumerate()
                                                                .map(|(idx, bucket)| {
                                                                    [
                                                                        idx as f64,
                                                                        bucket.count_at_value()
                                                                            as f64,
                                                                    ]
                                                                })
                                                                .collect::<Vec<_>>();

                                                            // Mean
                                                            plot_ui.vline(
                                                                VLine::new(
                                                                    item.cycle_delta_stats.mean,
                                                                )
                                                                .style(LineStyle::dashed_dense())
                                                                .color(
                                                                    c
                                                                        // 50% alpha
                                                                        .gamma_multiply(0.75),
                                                                ),
                                                            );

                                                            // Std dev
                                                            plot_ui.vline(
                                                                VLine::new(
                                                                    item.cycle_delta_stats.mean
                                                                        - item
                                                                            .cycle_delta_stats
                                                                            .std_dev
                                                                            / 2.0,
                                                                )
                                                                .style(LineStyle::dashed_dense())
                                                                .color(c.gamma_multiply(0.5)),
                                                            );
                                                            plot_ui.vline(
                                                                VLine::new(
                                                                    item.cycle_delta_stats.mean
                                                                        + item
                                                                            .round_trip_stats
                                                                            .std_dev
                                                                            / 2.0,
                                                                )
                                                                .style(LineStyle::dashed_dense())
                                                                .color(c.gamma_multiply(0.5)),
                                                            );

                                                            plot_ui.line(
                                                                Line::new(PlotPoints::new(points))
                                                                    .color(c)
                                                                    .name(&item.display_name),
                                                            );
                                                        }
                                                    });
                                            });
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

            Box::new(MyApp { files })
        }),
    )
}

// Nicked from <https://github.com/emilk/egui/blob/e29022efc4783fe06842a46371d5bd88e3f13bdd/crates/egui_plot/src/plot_ui.rs#L16C5-L22C6>
fn idx_to_colour(idx: usize) -> Color32 {
    let i = idx as f32;
    let golden_ratio = (5.0_f32.sqrt() - 1.0) / 2.0; // 0.61803398875
    let h = i as f32 * golden_ratio;
    Hsva::new(h, 0.85, 0.5, 1.0).into() // TODO(emilk): OkLab or some other perspective color space
}
