#[cfg(target_os = "macos")]
mod menubar;

use std::{
    env,
    process::ExitCode,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use inputs::{
    Event, EventDisposition, EventKind, GrabMode, KeyCode, KeyEvent, Listener, LogicalKey,
    PressState,
};
use vnkey_engine::legacy::{Engine, InputMethod};

const LETTER_KEYS: [KeyCode; 26] = [
    KeyCode::KeyA,
    KeyCode::KeyB,
    KeyCode::KeyC,
    KeyCode::KeyD,
    KeyCode::KeyE,
    KeyCode::KeyF,
    KeyCode::KeyG,
    KeyCode::KeyH,
    KeyCode::KeyI,
    KeyCode::KeyJ,
    KeyCode::KeyK,
    KeyCode::KeyL,
    KeyCode::KeyM,
    KeyCode::KeyN,
    KeyCode::KeyO,
    KeyCode::KeyP,
    KeyCode::KeyQ,
    KeyCode::KeyR,
    KeyCode::KeyS,
    KeyCode::KeyT,
    KeyCode::KeyU,
    KeyCode::KeyV,
    KeyCode::KeyW,
    KeyCode::KeyX,
    KeyCode::KeyY,
    KeyCode::KeyZ,
];

const DIGIT_KEYS: [KeyCode; 10] = [
    KeyCode::Digit0,
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
];

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("convert") => {
            let method = match args.next().as_deref() {
                Some("vni") => InputMethod::Vni,
                _ => InputMethod::Telex,
            };
            let input = args.collect::<Vec<_>>().join(" ");
            println!("{}", convert(method, &input));
            ExitCode::SUCCESS
        }
        Some("daemon") => {
            let method = match args.next().as_deref() {
                Some("vni") => InputMethod::Vni,
                _ => InputMethod::Telex,
            };
            run_daemon(method)
        }
        Some("permissions") => {
            println!("{:?}", inputs::permission_status());
            ExitCode::SUCCESS
        }
        #[cfg(target_os = "macos")]
        None => run_menu_bar(InputMethod::Telex),
        _ => print_help(),
    }
}

fn run_daemon(method: InputMethod) -> ExitCode {
    if cfg!(target_os = "macos") {
        inputs::request_permission();
    }
    let engine = Arc::new(Mutex::new(Engine::new(method)));
    let listener = match start_listener(&engine) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            eprintln!("Permission status: {:?}", inputs::permission_status());
            return ExitCode::FAILURE;
        }
    };

    println!("VNKey is active ({method:?}). Press Ctrl+C to stop.");
    wait_while_healthy(&listener);
    eprintln!("The keyboard listener stopped unexpectedly.");
    ExitCode::FAILURE
}

/// Runs the menu bar UI on the main thread while keyboard capture starts in
/// the background as soon as Accessibility permission is granted.
#[cfg(target_os = "macos")]
fn run_menu_bar(method: InputMethod) -> ExitCode {
    let engine = Arc::new(Mutex::new(Engine::new(method)));
    let input_engine = Arc::clone(&engine);
    thread::spawn(move || serve_input(&input_engine));
    menubar::run(engine)
}

#[cfg(target_os = "macos")]
fn serve_input(engine: &Arc<Mutex<Engine>>) {
    inputs::request_permission();
    loop {
        if inputs::permission_status() == inputs::PermissionStatus::Granted {
            match start_listener(engine) {
                Ok(listener) => {
                    wait_while_healthy(&listener);
                    eprintln!("The keyboard listener stopped; restarting.");
                }
                Err(error) => eprintln!("{error}"),
            }
        }
        thread::sleep(Duration::from_secs(1));
    }
}

fn start_listener(engine: &Arc<Mutex<Engine>>) -> Result<Listener, String> {
    let keyboard = Enigo::new(&Settings::default())
        .map_err(|error| format!("Cannot initialize keyboard output: {error}"))?;
    let keyboard = Mutex::new(keyboard);
    let engine = Arc::clone(engine);
    inputs::Listener::builder()
        .grab_mode(GrabMode::Force)
        .start(move |event: &Event| handle_event(event, &engine, &keyboard))
        .map_err(|error| format!("Cannot capture keyboard input: {error}"))
}

fn wait_while_healthy(listener: &Listener) {
    while listener.is_healthy() {
        thread::sleep(Duration::from_secs(1));
    }
}

fn print_help() -> ExitCode {
    println!("VNKey {}", env!("CARGO_PKG_VERSION"));
    println!("Usage: vnkey convert [telex|vni] <text>");
    println!("       vnkey daemon [telex|vni]");
    println!("       vnkey permissions");
    ExitCode::SUCCESS
}

fn handle_event(
    event: &Event,
    engine: &Mutex<Engine>,
    keyboard: &Mutex<Enigo>,
) -> EventDisposition {
    if event.injected {
        return EventDisposition::Pass;
    }
    let EventKind::Key(key) = &event.kind else {
        return EventDisposition::Pass;
    };
    if key.state != PressState::Pressed {
        return EventDisposition::Pass;
    }
    let Ok(mut engine) = engine.try_lock() else {
        return EventDisposition::Pass;
    };
    if !engine.is_enabled() {
        return EventDisposition::Pass;
    }

    let edit = if key.modifiers.control || key.modifiers.alt || key.modifiers.meta {
        None
    } else if key.code() == Some(KeyCode::Backspace) {
        Some(engine.backspace())
    } else {
        typed_char(key).map(|value| engine.push(value))
    };
    let Some(edit) = edit else {
        engine.reset();
        return EventDisposition::Pass;
    };
    drop(engine);

    let Ok(mut keyboard) = keyboard.try_lock() else {
        return EventDisposition::Pass;
    };
    for _ in 0..edit.delete_chars {
        if keyboard.key(Key::Backspace, Direction::Click).is_err() {
            return EventDisposition::Pass;
        }
    }
    if !edit.insert.is_empty() && keyboard.text(&edit.insert).is_err() {
        return EventDisposition::Pass;
    }
    EventDisposition::Suppress
}

/// The character a key press types. Backends that do not resolve the layout
/// (`logical` is `None`) are read as a US QWERTY layout.
fn typed_char(key: &KeyEvent) -> Option<char> {
    if let Some(LogicalKey::Character(text)) = &key.logical {
        let mut chars = text.chars();
        return match (chars.next(), chars.next()) {
            (Some(value), None) => Some(value),
            _ => None,
        };
    }
    let code = key.code()?;
    if let Some(index) = LETTER_KEYS.iter().position(|&letter| letter == code) {
        let letter = char::from_digit(u32::try_from(index).ok()? + 10, 36)?;
        let uppercase = key.modifiers.shift != key.modifiers.caps_lock;
        return Some(if uppercase {
            letter.to_ascii_uppercase()
        } else {
            letter
        });
    }
    if key.modifiers.shift {
        return None;
    }
    let index = DIGIT_KEYS.iter().position(|&digit| digit == code)?;
    char::from_digit(u32::try_from(index).ok()?, 10)
}

fn convert(method: InputMethod, input: &str) -> String {
    let mut engine = Engine::new(method);
    let mut output = String::new();
    for value in input.chars() {
        let edit = engine.push(value);
        for _ in 0..edit.delete_chars {
            output.pop();
        }
        output.push_str(&edit.insert);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use inputs::{Modifiers, PhysicalKey};

    #[test]
    fn converts_a_sentence() {
        assert_eq!(convert(InputMethod::Telex, "Tooi ye **"), "Tôi ye **");
    }

    fn key(code: KeyCode, modifiers: Modifiers) -> KeyEvent {
        KeyEvent {
            physical: PhysicalKey::Code(code),
            logical: None,
            state: PressState::Pressed,
            repeat: false,
            modifiers,
        }
    }

    #[test]
    fn derives_letters_from_physical_keys() {
        let shift = Modifiers {
            shift: true,
            ..Modifiers::default()
        };
        let caps = Modifiers {
            caps_lock: true,
            ..Modifiers::default()
        };
        let both = Modifiers {
            shift: true,
            caps_lock: true,
            ..Modifiers::default()
        };
        assert_eq!(
            typed_char(&key(KeyCode::KeyA, Modifiers::default())),
            Some('a')
        );
        assert_eq!(
            typed_char(&key(KeyCode::KeyW, Modifiers::default())),
            Some('w')
        );
        assert_eq!(typed_char(&key(KeyCode::KeyZ, shift)), Some('Z'));
        assert_eq!(typed_char(&key(KeyCode::KeyD, caps)), Some('D'));
        assert_eq!(typed_char(&key(KeyCode::KeyD, both)), Some('d'));
    }

    #[test]
    fn derives_unshifted_digits_only() {
        let shift = Modifiers {
            shift: true,
            ..Modifiers::default()
        };
        assert_eq!(
            typed_char(&key(KeyCode::Digit0, Modifiers::default())),
            Some('0')
        );
        assert_eq!(
            typed_char(&key(KeyCode::Digit9, Modifiers::default())),
            Some('9')
        );
        assert_eq!(typed_char(&key(KeyCode::Digit9, shift)), None);
    }

    #[test]
    fn ignores_non_text_keys() {
        assert_eq!(typed_char(&key(KeyCode::Space, Modifiers::default())), None);
        assert_eq!(typed_char(&key(KeyCode::Enter, Modifiers::default())), None);
    }

    #[test]
    fn prefers_the_logical_character() {
        let mut event = key(KeyCode::KeyA, Modifiers::default());
        event.logical = Some(LogicalKey::Character("q".into()));
        assert_eq!(typed_char(&event), Some('q'));
    }
}
