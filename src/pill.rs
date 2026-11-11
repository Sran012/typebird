// overlay pill: small, always on top, bottom-center of the screen.
// hidden when idle, shown while recording.
//
// the window is created once and never unmapped. unmapping and remapping makes
// the window manager focus the pill, and then every keystroke goes to the pill
// instead of the app you were typing in. instead the window stays mapped and
// shrinks to 1x1 when idle, so it never takes focus again.
//
// on GNOME Wayland a client cannot choose its own window position, so the
// pill runs through XWayland (X11).
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
    shown: bool,
    configured: bool,
}

// ICCCM: a window with WM_HINTS.input = False never receives keyboard focus.
// without this the window manager focuses the pill every time it is shown,
// and typed text goes to the pill instead of the app behind it.
#[cfg(target_os = "linux")]
fn make_non_focusable() {
    use x11_dl::xlib;

    let xlib = match xlib::Xlib::open() {
        Ok(x) => x,
        Err(_) => return,
    };
    unsafe {
        let display = (xlib.XOpenDisplay)(std::ptr::null());
        if display.is_null() {
            return;
        }
        let root = (xlib.XDefaultRootWindow)(display);
        if let Some(win) = find_pill_window(&xlib, display, root, 0) {
            let mut hints: xlib::XWMHints = std::mem::zeroed();
            hints.flags = xlib::InputHint;
            hints.input = 0;
            (xlib.XSetWMHints)(display, win, &mut hints);
            (xlib.XFlush)(display);
        }
        (xlib.XCloseDisplay)(display);
    }
}

#[cfg(target_os = "linux")]
fn find_pill_window(
    xlib: &x11_dl::xlib::Xlib,
    display: *mut x11_dl::xlib::Display,
    win: x11_dl::xlib::Window,
    depth: u32,
) -> Option<x11_dl::xlib::Window> {
    use std::ffi::CStr;
    use x11_dl::xlib;

    if depth > 5 {
        return None;
    }
    unsafe {
        let mut name: *mut std::os::raw::c_char = std::ptr::null_mut();
        if (xlib.XFetchName)(display, win, &mut name) != 0 && !name.is_null() {
            let s = CStr::from_ptr(name).to_string_lossy().into_owned();
            (xlib.XFree)(name as *mut _);
            if s == "typebird-pill" {
                return Some(win);
            }
        }
        let mut root_ret: xlib::Window = 0;
        let mut parent_ret: xlib::Window = 0;
        let mut children: *mut xlib::Window = std::ptr::null_mut();
        let mut nchildren: u32 = 0;
        if (xlib.XQueryTree)(
            display,
            win,
            &mut root_ret,
            &mut parent_ret,
            &mut children,
            &mut nchildren,
        ) != 0
        {
            for i in 0..nchildren {
                let child = *children.add(i as usize);
                if let Some(found) = find_pill_window(xlib, display, child, depth + 1) {
                    (xlib.XFree)(children as *mut _);
                    return Some(found);
                }
            }
            if !children.is_null() {
                (xlib.XFree)(children as *mut _);
            }
        }
    }
    None
}

fn bottom_center(ctx: &egui::Context) -> egui::Pos2 {
    let (mx, my) = ctx
        .input(|i| i.viewport().monitor_size)
        .map(|m| (m.x, m.y))
        .unwrap_or((1920.0, 1080.0));
    egui::pos2((mx - PILL_W) / 2.0, my - PILL_H - BOTTOM_GAP)
}

impl eframe::App for PillApp {
    // keep everything outside the rounded card fully transparent
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // mark the window non-focusable while it is still hidden, then map it.
        // mapping a focusable window makes the window manager hand it the
        // keyboard, which swallows the text meant for the focused app.
        if !self.configured {
            make_non_focusable();
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(1.0, 1.0)));
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(bottom_center(ctx)));
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            self.configured = true;
        }

        self.state = send_cmd("state");
        let show = self.state == "listening";

        if show != self.shown {
            if show {
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                    PILL_W, PILL_H,
                )));
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(bottom_center(ctx)));
                ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                    egui::WindowLevel::AlwaysOnTop,
                ));
            } else {
                // 1x1 transparent, parked at the bottom center. never unmapped,
                // so it never steals focus from the app being typed into.
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(1.0, 1.0)));
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(bottom_center(ctx)));
            }
            self.shown = show;
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(150));

        if !show {
            return;
        }

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
            .with_inner_size([1.0, 1.0])
            .with_always_on_top()
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(true)
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
                shown: false,
                configured: false,
            }))
        }),
    )
}
