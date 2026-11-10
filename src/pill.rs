// the overlay pill. small floating window, bottom-center, click-through.
// shows only when recording or transcribing. idle = hidden.
use eframe::egui;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

fn socket_path() -> String {
    let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    format!("{}/typebird.sock", runtime)
}

fn fetch_state() -> String {
    if let Ok(mut stream) = UnixStream::connect(socket_path()) {
        let _ = writeln!(stream, "state");
        let _ = stream.flush();
        let mut reader = BufReader::new(stream);
        let mut resp = String::new();
        if reader.read_line(&mut resp).is_ok() {
            return resp.trim().to_string();
        }
    }
    "disconnected".to_string()
}

struct PillApp {
    state: String,
}

impl eframe::App for PillApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.state = fetch_state();

        let (color, label) = match self.state.as_str() {
            "listening" => (egui::Color32::from_rgb(220, 50, 50), "● recording"),
            "transcribing" => (egui::Color32::from_rgb(255, 165, 0), "transcribing…"),
            "disconnected" => (egui::Color32::GRAY, "daemon down"),
            _ => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                ctx.request_repaint_after(std::time::Duration::from_millis(500));
                return;
            }
        };

        // force small size every frame
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(140.0, 32.0)));
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));

        let mut visuals = egui::Visuals::dark();
        visuals.window_fill = egui::Color32::from_rgba_unmultiplied(25, 25, 25, 230);
        ctx.set_visuals(visuals);

        egui::CentralPanel::default()
            .frame(egui::Frame::none().rounding(16.0).inner_margin(egui::Margin::symmetric(12.0, 6.0)))
            .show(ctx, |ui| {
                ui.colored_label(color, egui::RichText::new(label).size(12.0).strong());
            });

        ctx.request_repaint_after(std::time::Duration::from_millis(200));
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([140.0, 32.0])
            .with_always_on_top()
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(false),
        ..Default::default()
    };
    eframe::run_native(
        "typebird-pill",
        options,
        Box::new(|_cc| Ok(Box::new(PillApp { state: "ready".to_string() }))),
    )
}