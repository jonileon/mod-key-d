use std::io::Write;
use std::fs::read_to_string;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread;
use std::path::Path;
use std::os::unix::net::UnixListener;

use evdev::{Device, InputEventKind, Key};

const INPUT_DEVICE_PATHS: &str = "/proc/bus/input/devices";
const SOCKET_PATH: &str = "/tmp/modifier.sock";

#[derive(Clone, Debug)]
struct ModifierState {
    alt: bool,
    meta: bool,
    shift: bool,
    ctrl: bool,
}


fn main() {
    let keyd_keyboard_path = get_keyboard_path();

    let (tx, rx) = channel::<ModifierState>();

    thread::spawn(move || {
        socket_server(rx);
    });

    keyboard_daemon(tx, &keyd_keyboard_path);
}

fn get_keyboard_path() -> String {

    let file = read_to_string(INPUT_DEVICE_PATHS).expect(&format!("Failed to open devices on: {}", INPUT_DEVICE_PATHS));
    let mut iter = file.lines();
    let mut line = iter.next().expect("Failed to read devices");

    while !line.contains("keyd virtual keyboard") {
        line = iter.next().expect("Failed to read devices");
    }

    while !(line.starts_with("H")){
        line = iter.next().expect("Couldnt find event information for keyd virtual keyboard");
    }

    let start = line.find("event").expect("Couldnt find evdev Handle for keyd virtual keyboard");
    let mut end = start;
    let chars: Vec<char> = line.chars().collect();
    while end < line.len() {
        if chars[end] == ' ' {
            break
        }
        end = end + 1;
    }

    let event = line[start..end].to_string();
    return format!("/dev/input/{}", event)
}

fn keyboard_daemon(tx: Sender<ModifierState>, path : &str) {
    let mut keyboard =
        Device::open(path)
        .expect("Failed to open keyboard");


    let mut state = ModifierState {
        alt: false,
        meta: false,
        shift: false,
        ctrl: false,
    };

    loop {
        let events = keyboard.fetch_events().unwrap();

        for event in events {
            match event.kind() {
                InputEventKind::Key(key) => {

                    let value = match event.value() {
                        0 => false,
                        1 => true,
                        _ => continue, // ignore repeat events
                    };
                    match key {
                        Key::KEY_LEFTSHIFT |
                        Key::KEY_RIGHTSHIFT => {
                            state.shift = value;
                        }

                        Key::KEY_LEFTCTRL |
                        Key::KEY_RIGHTCTRL => {
                            state.ctrl = value;
                        }

                        Key::KEY_LEFTMETA |
                        Key::KEY_RIGHTMETA => {
                            state.meta = value;
                        }

                        Key::KEY_LEFTALT |
                        Key::KEY_RIGHTALT => {
                            state.alt = value;
                        }

                        _ => continue,
                    }
                    tx.send(state.clone()).unwrap();
                }
                _ => {}
            }
        }
    }
}

fn socket_server(rx: Receiver<ModifierState>) {

    if Path::new(SOCKET_PATH).exists() {
        std::fs::remove_file(SOCKET_PATH).unwrap();
    }
    let listener = UnixListener::bind(SOCKET_PATH).unwrap();
    println!("Listening on {}", SOCKET_PATH);

    let mut last_state = ModifierState {
        alt: false,
        meta: false,
        shift: false,
        ctrl: false,
    };

    for connection in listener.incoming() {
        let mut stream = connection.unwrap();

        println!("client connected");

        let response = format!(
            "{{\"ctrl\":{},\"shift\":{},\"alt\":{},\"meta\":{}}}\n",
            last_state.ctrl,
            last_state.shift,
            last_state.alt,
            last_state.meta
        );
        if stream.write_all(response.as_bytes()).is_err() {
            continue;
        }

        while let Ok(state) = rx.recv() {
            last_state = state.clone();

            let response = format!(
                "{{\"ctrl\":{},\"shift\":{},\"alt\":{},\"meta\":{}}}\n",
                state.ctrl,
                state.shift,
                state.alt,
                state.meta
            );

            match stream.write_all(response.as_bytes()) {
                Ok(_) => {},
                Err(e) => {
                    println!("client disconnected: {}", e);
                    break;
                }
            }
        }
    }
}
