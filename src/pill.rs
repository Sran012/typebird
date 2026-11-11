// overlay pill: small, always on top, bottom-center of the screen.
// hidden when idle. appears when recording starts and goes away when it stops.
//
// on GNOME Wayland a client cannot choose its own window position, so the
// pill runs through XWayland (X11). that is the only way to pin a normal
// window to the bottom edge without a shell extension.
use eframe::egui;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

const PILL_W: f32 = 214.0;
const PILL_H: f32 = 46.0;
const BOTTOM_GAP: f32 = 56.0;

fn socket_path() -> String {
    let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    format!("{}/typebird.sock", runtime)
}

fn send_cmd(cmd: &str) -> String {
    if let Ok(mut stream) = UnixStream::connect(socket_path()) {
        let _ = writeln!(stream, "{}", cmd);
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
    // keep everything outside the rounded card fully transparent
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.state = send_cmd("state");

        let show = matches!(self.state.as_str(), "listening" | "transcribing");
        if !show {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            ctx.request_repaint_after(std::time::Duration::from_millis(150));
            return;
        }

        // map first, then move. the window manager places a freshly mapped
        // window itself, so the position has to be reasserted after it is up.
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));

        let (mx, my) = ctx
            .input(|i| i.viewport().monitor_size)
            .map(|m| (m.x, m.y))
            .unwrap_or((1920.0, 1080.0));
        let x = (mx - PILL_W) / 2.0;
        let y = my - PILL_H - BOTTOM_GAP;
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(x, y)));
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            egui::WindowLevel::AlwaysOnTop,
        ));
        ctx.request_repaint_after(std::time::Duration::from_millis(150));

        let (dot, label) = match self.state.as_str() {
            "listening" => (egui::Color32::from_rgb(235, 64, 52), "recording"),
            "transcribing" => (egui::Color32::from_rgb(245, 158, 11), "transcribing"),
            _ => (egui::Color32::from_rgb(90, 200, 120), "ready"),
        };

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgba_unmultiplied(22, 22, 26, 240))
                    .rounding(PILL_H / 2.0)
                    .inner_margin(egui::Margin::symmetric(14.0, 8.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let (dot_rect, _) =
                        ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                    ui.painter().circle_filled(dot_rect.center(), 5.0, dot);
                    ui.add_space(7.0);
                    ui.label(
                        egui::RichText::new(label)
                            .size(13.0)
                            .strong()
                            .color(egui::Color32::from_rgb(235, 235, 240)),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let stop = egui::Button::new(
                            egui::RichText::new("stop")
                                .size(12.0)
                                .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(180, 45, 38))
                        .rounding(11.0)
                        .min_size(egui::vec2(58.0, 24.0));
                        if ui.add(stop).clicked() {
                            send_cmd("stop");
                        }
                    });
                });
            });
    }
}

fn main() -> eframe::Result<()> {
    // force XWayland: drop the Wayland display so winit picks X11, which can
    // position and keep the window on top. harmless if only X11 is present.
    if std::env::var_os("DISPLAY").is_some() {
        unsafe {
            std::env::remove_var("WAYLAND_DISPLAY");
            std::env::remove_var("WAYLAND_SOCKET");
        }
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([PILL_W, PILL_H])
            .with_always_on_top()
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(false)
            .with_active(false)
            .with_visible(false),
        ..Default::default()
    };

    eframe::run_native(
        "typebird-pill",
        options,
        Box::new(|_cc| {
            Ok(Box::new(PillApp {
                state: "ready".to_string(),
            }))
        }),
    )
}
