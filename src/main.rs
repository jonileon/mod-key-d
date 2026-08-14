use std::io::Write;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread;
use std::path::Path;
use std::os::unix::net::UnixListener;

use evdev::{Device, InputEventKind, Key};

const KEYD_KEYBOARD_PATH: &str = "/dev/input/event15";
const SOCKET_PATH: &str = "/tmp/modifier.sock";

#[derive(Clone, Debug)]
struct ModifierState {
    alt: bool,
    meta: bool,
    shift: bool,
    ctrl: bool,
}


fn main() {
    let (tx, rx) = channel::<ModifierState>();

    thread::spawn(move || {
        socket_server(rx);
    });

    keyboard_daemon(tx);
}


fn keyboard_daemon(tx: Sender<ModifierState>) {
    let mut keyboard =
        Device::open(KEYD_KEYBOARD_PATH)
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
            "{{\"ctrl\":{},\"shift\":{},\"alt\":{},\"super\":{}}}\n",
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
                "{{\"ctrl\":{},\"shift\":{},\"alt\":{},\"super\":{}}}\n",
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
