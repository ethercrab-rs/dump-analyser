use std::path::PathBuf;

use dump_analyser::PcapFile;
use eframe::egui;
use egui_plot::{Legend, Line, Plot, PlotPoints};

#[derive(Default)]
struct MyApp {
    plots: Vec<[f64; 2]>,
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

        let mut plot_rect = None;

        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Save Plot").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
            }

            let my_plot = Plot::new("My Plot").legend(Legend::default());

            // let's create a dummy line in the plot
            // let graph: Vec<[f64; 2]> = vec![[0.0, 1.0], [2.0, 3.0], [3.0, 2.0]];
            let inner = my_plot.show(ui, |plot_ui| {
                plot_ui.line(Line::new(PlotPoints::new(self.plots.clone())).name("curve"));
            });

            // Remember the position of the plot
            plot_rect = Some(inner.response.rect);
        });

        // // Check for returned screenshot:
        // let screenshot = ctx.input(|i| {
        //     for event in &i.raw.events {
        //         if let egui::Event::Screenshot { image, .. } = event {
        //             return Some(image.clone());
        //         }
        //     }
        //     None
        // });
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
        Box::new(|cc| Box::new(MyApp { plots: graph })),
    )
}
