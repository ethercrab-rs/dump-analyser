use eframe::egui;
use egui_plot::{Legend, Line, Plot, PlotPoints};

#[derive(Default)]
struct MyApp {}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut plot_rect = None;
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Save Plot").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
            }

            let my_plot = Plot::new("My Plot").legend(Legend::default());

            // let's create a dummy line in the plot
            let graph: Vec<[f64; 2]> = vec![[0.0, 1.0], [2.0, 3.0], [3.0, 2.0]];
            let inner = my_plot.show(ui, |plot_ui| {
                plot_ui.line(Line::new(PlotPoints::from(graph)).name("curve"));
            });
            // Remember the position of the plot
            plot_rect = Some(inner.response.rect);
        });

        // Check for returned screenshot:
        let screenshot = ctx.input(|i| {
            for event in &i.raw.events {
                if let egui::Event::Screenshot { image, .. } = event {
                    return Some(image.clone());
                }
            }
            None
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    // let options = eframe::NativeOptions {
    //     viewport: egui::ViewportBuilder::default().with_inner_size([350.0, 200.0]),
    //     ..Default::default()
    // };

    // eframe::run_native(
    //     "My egui App with a plot",
    //     options,
    //     Box::new(|_cc| Box::<MyApp>::default()),
    // )

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };

    eframe::run_native(
        "eframe template",
        native_options,
        Box::new(|cc| Box::new(MyApp {})),
    )
}
