use std::path::PathBuf;

use dump_analyser::PcapFile;
use eframe::egui;
use egui_plot::{Legend, Line, Plot, PlotBounds, PlotPoints};

#[derive(Default)]
struct MyApp {
    plots: Vec<[f64; 2]>,
    prev_bounds: Option<PlotBounds>,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(150.0)
            .width_range(80.0..=200.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Left Panel");
                });

                egui::ScrollArea::vertical().show(ui, |ui| {
                    // lorem_ipsum(ui);
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Save Plot").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
            }

            let my_plot = Plot::new("My Plot").legend(Legend::default());

            // Perf: https://github.com/emilk/egui/pull/3849
            my_plot.show(ui, |plot_ui| {
                let plot_bounds = plot_ui.plot_bounds();

                let points = if let Some(_) = self.prev_bounds {
                    let start_count = plot_bounds.min()[0] as usize;
                    let end_count = (plot_bounds.max()[0] as usize).min(self.plots.len());

                    let display_range = start_count..end_count;

                    let values_width = plot_bounds.width();

                    let pixels_width = {
                        plot_ui.screen_from_plot(plot_bounds.max().into())[0]
                            - plot_ui.screen_from_plot(plot_bounds.min().into())[0]
                    } as f64;

                    let stride = (values_width / pixels_width).max(1.0) as usize;

                    self.plots[display_range]
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
                    self.plots.clone()
                };

                plot_ui.line(Line::new(PlotPoints::new(points)).name("curve"));

                self.prev_bounds = Some(plot_bounds);
            });
        });
    }
}

fn main() -> Result<(), eframe::Error> {
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

    eframe::run_native(
        "eframe template",
        native_options,
        Box::new(|cc| {
            Box::new(MyApp {
                plots: graph,
                prev_bounds: None,
            })
        }),
    )
}
