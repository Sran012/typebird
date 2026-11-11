// entrypoint. `typebird daemon` runs the daemon. anything else is a client
// command sent over the unix socket.
mod daemon;
mod doctor;
mod hotkey;
mod inject;
mod models;
mod paths;
mod seam;
mod transcribe;

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

fn socket_path() -> String {
    let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    format!("{}/typebird.sock", runtime)
}

fn send_cmd(cmd: &str) -> String {
    let path = socket_path();
    let mut stream = match UnixStream::connect(&path) {
        Ok(s) => s,
        Err(e) => return format!("daemon not running: {}", e),
    };
    let _ = writeln!(stream, "{}", cmd);
    let _ = stream.flush();
    let mut reader = BufReader::new(stream);
    let mut resp = String::new();
    let _ = reader.read_line(&mut resp);
    resp.trim().to_string()
}

// one-time setup: make the daemon (and the pill it spawns) come up with the
// desktop session, so the shortcut works without ever opening a terminal.
fn install_autostart() {
    let exe = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("typebird"));
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let dir = format!("{}/.config/autostart", home);
    let _ = std::fs::create_dir_all(&dir);
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName=Typebird\nComment=voice typing\nExec={} daemon\nTerminal=false\nNoDisplay=true\nX-GNOME-Autostart-enabled=true\n",
        exe.display()
    );
    let path = format!("{}/typebird.desktop", dir);
    match std::fs::write(&path, entry) {
        Ok(_) => {
            println!("autostart installed: {}", path);
            println!("next login starts the daemon and pill automatically");
            println!("hotkey: Ctrl+Shift+D toggles recording");
        }
        Err(e) => eprintln!("failed to write {}: {}", path, e),
    }
}

fn usage() {
    println!("typebird {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("voice typing for linux");
    println!();
    println!("usage: typebird <command>");
    println!();
    println!("commands:");
    println!("  install        autostart the daemon and pill with the desktop session");
    println!("  daemon         run the daemon in the foreground");
    println!("  toggle         start or stop recording");
    println!("  start          start recording");
    println!("  stop           stop, transcribe, type");
    println!("  cancel         stop without transcribing");
    println!("  state          print ready, listening, or transcribing");
    println!("  info           print engine, recorder, injector");
    println!("  doctor         check audio, model, uinput, daemon");
    println!("  models         list models");
    println!("  models get X   download model X");
    println!("  hotkey <key>   extra binding for GNOME");
    println!("  type <text>    type text through the virtual keyboard");
    println!();
    println!("  -h, --help     show this");
    println!("  -V, --version  show the version");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match cmd {
        "daemon" => daemon::run(),
        "help" | "-h" | "--help" => usage(),
        "-V" | "--version" => println!("typebird {}", env!("CARGO_PKG_VERSION")),
        "install" | "setup" => install_autostart(),
        "toggle" => println!("{}", send_cmd("toggle")),
        "start" => println!("{}", send_cmd("start")),
        "stop" => println!("{}", send_cmd("stop")),
        "cancel" => println!("{}", send_cmd("cancel")),
        "type" => {
            let text = args[2..].join(" ");
            inject::type_text(&text);
            println!("typed");
        }
        "state" => println!("{}", send_cmd("state")),
        "info" => println!("{}", send_cmd("info")),
        "hotkey" => hotkey::setup_gnome(args.get(2).map(|s| s.as_str()).unwrap_or("<Super>h")),
        "doctor" => doctor::run(),
        "models" => match args.get(2).map(|s| s.as_str()) {
            Some("get") => models::get(args.get(3).map(|s| s.as_str()).unwrap_or("base.en")),
            _ => models::list(),
        },
        _ => usage(),
    }
}
