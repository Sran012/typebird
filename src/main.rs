// entrypoint. `typebird daemon` runs the daemon. anything else is a client
// command sent over the unix socket.
mod audio;
mod daemon;
mod inject;
mod hotkey;
mod doctor;
mod models;
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

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match cmd {
        "daemon" => daemon::run(),
        "toggle" => println!("{}", send_cmd("toggle")),
        "start" => println!("{}", send_cmd("start")),
        "stop" => println!("{}", send_cmd("stop")),
        "cancel" => println!("{}", send_cmd("cancel")),
        "state" => println!("{}", send_cmd("state")),
        "info" => println!("{}", send_cmd("info")),
        "hotkey" => hotkey::setup_gnome(args.get(2).map(|s| s.as_str()).unwrap_or("<Super>h")),
        "doctor" => doctor::run(),
        "models" => match args.get(2).map(|s| s.as_str()) {
            Some("get") => models::get(args.get(3).map(|s| s.as_str()).unwrap_or("base.en")),
            _ => models::list(),
        },
        _ => {
            println!("typebird: usage: daemon | toggle | start | stop | cancel | state | info");
        }
    }
}
