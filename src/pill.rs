// the overlay pill. subscribes to daemon socket, shows state.
// egui + eframe. native, no webview, no python.
use eframe::egui;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

fn socket_path() -> String {
    let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    format!("{}/typebird.sock", runtime)
}

struct PillApp {
    state: String,
}

impl eframe::App for PillApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // poll state from daemon
        if let Ok(mut stream) = UnixStream::connect(socket_path()) {
            let _ = writeln!(stream, "state");
            let _ = stream.flush();
            let mut reader = BufReader::new(stream);
            let mut resp = String::new();
            if reader.read_line(&mut resp).is_ok() {
                self.state = resp.trim().to_string();
            }
        }

        // render pill
        egui::CentralPanel::default().show(ctx, |ui| {
            let color = match self.state.as_str() {
                "listening" => egui::Color32::RED,
                "transcribing" => egui::Color32::from_rgb(255, 165, 0),
                _ => egui::Color32::GRAY,
            };
            ui.colored_label(color, &self.state);
        });

        // request repaint
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_always_on_top()
            .with_decorations(false)
            .with_transparent(true),
        ..Default::default()
    };
    eframe::run_native(
        "typebird-pill",
        options,
        Box::new(|_cc| Ok(Box::new(PillApp { state: "ready".to_string() }))),
    )
}
