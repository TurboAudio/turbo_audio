use eframe::{
    egui::{self, Vec2b},
    CreationContext, EventLoopBuilder, UserEvent,
};
use egui_plot::{Line, Plot, PlotPoint, PlotPoints};
use egui_winit::winit::platform::x11::EventLoopBuilderExtX11;
use turbo_plugin::{
    audio_api::get_all_bins, general_plugin::NativeGeneralPlugin, make_general_effect_plugin,
};

fn ui_thread(context_sender: oneshot::Sender<egui::Context>) {
    let native_options = eframe::NativeOptions {
        event_loop_builder: Some(Box::new(|builder: &mut EventLoopBuilder<UserEvent>| {
            builder.with_any_thread(true);
        })),
        ..Default::default()
    };

    eframe::run_native(
        "My egui App",
        native_options,
        Box::new(|cc| {
            let _ = context_sender.send(cc.egui_ctx.clone());
            Box::new(MyEguiApp::new(cc))
        }),
    )
    .unwrap();
}

#[derive(Default)]
struct MyEguiApp {
    bins: Vec<f32>,
    points: Vec<PlotPoint>,
}

impl MyEguiApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        const MAX_SMOOTHING: f32 = 0.9;
        const MIN_SMOOTHING: f32 = 0.9;
        const NUM_BINS_VISUAL: usize = 20;
        const GAMMA: f32 = 2.0;
        const MAX_AVERAGE: usize = 2;

        get_all_bins(&mut self.bins);
        let bins_len = (self.bins.len() as f32 * 0.66).round() as usize;
        let bins = &mut self.bins.as_mut_slice()[0..bins_len];

        for (index, bin) in bins.iter_mut().enumerate() {
            *bin = (((*bin * (index + 1) as f32) + 1.0_f32).log10()).max(0.0);
        }

        self.points
            .resize(NUM_BINS_VISUAL, PlotPoint::new(0.0, 0.0));

        let mut memory_scratchpad = Vec::new();

        for (i, point) in self.points.iter_mut().enumerate() {
            let current_proportion = (i as f32) / (NUM_BINS_VISUAL as f32);
            let next_proportion = ((i + 1) as f32) / (NUM_BINS_VISUAL as f32);

            let current = ((current_proportion).powf(GAMMA) * bins_len as f32)
                .round()
                .max(0.0) as usize;
            let next = ((next_proportion).powf(GAMMA) * bins_len as f32)
                .round()
                .min((bins_len - 1) as f32) as usize;

            let bin_slice = &bins[current..next];

            memory_scratchpad.clear();
            memory_scratchpad.extend_from_slice(bin_slice);
            // reverse sort aka largests first
            memory_scratchpad.sort_unstable_by(|a, b| b.total_cmp(a));

            let max_average_actual = MAX_AVERAGE.max(memory_scratchpad.len()) - 1;
            let value = memory_scratchpad.as_slice()[0..max_average_actual]
                .iter()
                .sum::<f32>()
                / max_average_actual as f32;

            let smoothing = MAX_SMOOTHING - ((MAX_SMOOTHING - MIN_SMOOTHING) * current_proportion);
            point.x = i as f64;
            point.y = point.y * smoothing as f64 + value as f64 * (1.0 - smoothing as f64);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let points = PlotPoints::Owned(self.points.clone());

            let line = Line::new(points);
            Plot::new("my_plot")
                .view_aspect(2.0)
                .auto_bounds(Vec2b::new(true, false))
                .allow_zoom(false)
                .allow_scroll(false)
                .allow_drag(true)
                .allow_boxed_zoom(false)
                .allow_double_click_reset(false)
                .include_y(0.0)
                .include_y(2.0)
                .show_grid(false)
                .show(ui, |plot_ui| plot_ui.line(line));
        });
    }
}

struct Ui {
    egui_ctx: egui::Context,
}

impl Drop for Ui {
    fn drop(&mut self) {
        println!("Dropping UI library instance");
    }
}

impl Ui {
    pub fn new() -> Self {
        let (sender, rcv) = oneshot::channel();
        std::thread::spawn(|| {
            ui_thread(sender);
        });
        let egui_ctx = rcv.recv().unwrap();
        Self {
            egui_ctx
        }
    }
}

impl NativeGeneralPlugin for Ui {
    fn name(&self) -> *const std::ffi::c_char {
        c"UI".as_ptr()
    }

    fn tick(&mut self) {
        self.egui_ctx.request_repaint();
    }

    fn load() {
        println!("UI library loaded")
    }

    fn unload() {
        println!("UI library unloaded");
    }
}

make_general_effect_plugin!(Ui, Ui::new());
