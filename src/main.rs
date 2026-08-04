use std::io::Write;
use std::thread;
use std::sync::{Arc, Mutex};
use std::path::Path;
use std::os::unix::net::UnixListener;

use evdev::{Device, InputEventKind, Key};

struct ModifierState {
    alt: bool,
    meta: bool,
    shift: bool,
    ctrl: bool,
}


//[TODO] hardcoded for testing, should be dynamically computed later
const KEYD_KEYBOARD_PATH: &str = "/dev/input/event15";
const SOCKET_PATH: &str = "/tmp/modifier.sock";
fn main() {
    let state = ModifierState {
        alt: false,
        meta: false,
        shift: false,
        ctrl: false,
    };
    let state_arc = Arc::new(Mutex::new(state));

    let socket_state = state_arc.clone();
    thread::spawn(move || {
        socket_server(socket_state);
    });

    keyboard_daemon(state_arc);

}

fn keyboard_daemon(state: Arc<Mutex<ModifierState>>) {
    loop {
        let mut keyboard = Device::open(KEYD_KEYBOARD_PATH).expect("Failed to get ahold of keyd virtual keyboard");
        let events = keyboard.fetch_events().unwrap();

        for event in events {
            match event.kind() {
                InputEventKind::Key(key) => {
                    match key {
                        Key::KEY_LEFTSHIFT | Key::KEY_RIGHTSHIFT => {
                            state.lock().unwrap().shift = event.value() != 0;
                        },
                        Key::KEY_LEFTCTRL | Key::KEY_RIGHTCTRL => {
                            state.lock().unwrap().ctrl = event.value() != 0;
                        }
                        Key::KEY_RIGHTMETA | Key::KEY_LEFTMETA => {
                            state.lock().unwrap().meta = event.value() != 0;
                        }
                        Key::KEY_RIGHTALT | Key::KEY_LEFTALT => {
                            state.lock().unwrap().alt = event.value() != 0;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
}

fn socket_server(state: Arc<Mutex<ModifierState>>) {

    if Path::new(SOCKET_PATH).exists() {
        std::fs::remove_file(SOCKET_PATH).unwrap();
    }
    let listener = UnixListener::bind(SOCKET_PATH).unwrap();
    for connection in listener.incoming() {
        let mut stream = connection.unwrap();
        let state = state.lock().unwrap();
        let response = format!(
            "{{\"ctrl\":{},\"shift\":{},\"alt\":{},\"super\":{}}}",
            state.ctrl,
            state.shift,
            state.alt,
            state.meta
        );

        stream.write_all(response.as_bytes()).unwrap();
    }

}
