use std::{
    env,
    process::ExitCode,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use inputs::{Event, EventDisposition, EventKind, GrabMode, KeyCode, LogicalKey, PressState};
use vnkey_engine::{Engine, InputMethod};

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
        _ => {
            println!("VNKey {}", env!("CARGO_PKG_VERSION"));
            println!("Usage: vnkey convert [telex|vni] <text>");
            println!("       vnkey daemon [telex|vni]");
            println!("       vnkey permissions");
            ExitCode::SUCCESS
        }
    }
}

struct Runtime {
    engine: Engine,
    keyboard: Enigo,
}

fn run_daemon(method: InputMethod) -> ExitCode {
    if cfg!(target_os = "macos") {
        inputs::request_permission();
    }
    let keyboard = match Enigo::new(&Settings::default()) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Cannot initialize keyboard output: {error}");
            return ExitCode::FAILURE;
        }
    };
    let runtime = Arc::new(Mutex::new(Runtime {
        engine: Engine::new(method),
        keyboard,
    }));
    let callback_runtime = Arc::clone(&runtime);
    let listener = match inputs::Listener::builder()
        .grab_mode(GrabMode::Force)
        .start(move |event: &Event| handle_event(event, &callback_runtime))
    {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Cannot capture keyboard input: {error}");
            eprintln!("Permission status: {:?}", inputs::permission_status());
            return ExitCode::FAILURE;
        }
    };

    println!("VNKey is active ({method:?}). Press Ctrl+C to stop.");
    while listener.is_healthy() {
        thread::sleep(Duration::from_secs(1));
    }
    eprintln!("The keyboard listener stopped unexpectedly.");
    ExitCode::FAILURE
}

fn handle_event(event: &Event, runtime: &Mutex<Runtime>) -> EventDisposition {
    if event.injected {
        return EventDisposition::Pass;
    }
    let EventKind::Key(key) = &event.kind else {
        return EventDisposition::Pass;
    };
    if key.state != PressState::Pressed {
        return EventDisposition::Pass;
    }
    if key.modifiers.control || key.modifiers.alt || key.modifiers.meta {
        if let Ok(mut state) = runtime.try_lock() {
            state.engine.reset();
        }
        return EventDisposition::Pass;
    }

    let edit = if key.code() == Some(KeyCode::Backspace) {
        runtime
            .try_lock()
            .ok()
            .map(|mut state| state.engine.backspace())
    } else if let Some(LogicalKey::Character(text)) = &key.logical {
        let mut chars = text.chars();
        match (chars.next(), chars.next()) {
            (Some(value), None) => runtime
                .try_lock()
                .ok()
                .map(|mut state| state.engine.push(value)),
            _ => None,
        }
    } else {
        if let Ok(mut state) = runtime.try_lock() {
            state.engine.reset();
        }
        None
    };

    let Some(edit) = edit else {
        return EventDisposition::Pass;
    };
    let Ok(mut state) = runtime.try_lock() else {
        return EventDisposition::Pass;
    };
    for _ in 0..edit.delete_chars {
        if state
            .keyboard
            .key(Key::Backspace, Direction::Click)
            .is_err()
        {
            return EventDisposition::Pass;
        }
    }
    if !edit.insert.is_empty() && state.keyboard.text(&edit.insert).is_err() {
        return EventDisposition::Pass;
    }
    EventDisposition::Suppress
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

    #[test]
    fn converts_a_sentence() {
        assert_eq!(convert(InputMethod::Telex, "Tooi ye **"), "Tôi ye **");
    }
}
