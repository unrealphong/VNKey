#[cfg(target_os = "macos")]
mod frontmost;
mod hotkey;
#[cfg(target_os = "macos")]
mod menubar;
mod reload;
mod session;
mod settings;
mod smart_switch;
mod state;

use std::{
    env,
    process::ExitCode,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use enigo::{Direction, Enigo, Keyboard};
use inputs::{
    Event, EventDisposition, EventKind, GrabMode, InjectionSignature, KeyCode, KeyEvent, Listener,
    PressState,
};
use vnkey_engine::{Config, Engine, InputType, Key, KeyInput, Output};

#[cfg(target_os = "macos")]
use crate::session::CaptureStatus;
use crate::{session::Session, settings::Paths};

/// Called after a change that affects the menu bar.
type Notify = Arc<dyn Fn() + Send + Sync>;

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

const SYMBOL_KEYS: [(KeyCode, char); 11] = [
    (KeyCode::Backquote, '`'),
    (KeyCode::Minus, '-'),
    (KeyCode::Equal, '='),
    (KeyCode::BracketLeft, '['),
    (KeyCode::BracketRight, ']'),
    (KeyCode::Backslash, '\\'),
    (KeyCode::Semicolon, ';'),
    (KeyCode::Quote, '\''),
    (KeyCode::Comma, ','),
    (KeyCode::Period, '.'),
    (KeyCode::Slash, '/'),
];

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("--version" | "-V") => {
            print_version();
            ExitCode::SUCCESS
        }
        Some("convert") => {
            let input_type = parse_input_type(args.next().as_deref()).unwrap_or_default();
            let input = args.collect::<Vec<_>>().join(" ");
            println!("{}", convert(input_type, &input));
            ExitCode::SUCCESS
        }
        Some("daemon") => run_daemon(parse_input_type(args.next().as_deref())),
        Some("permissions") => {
            println!("{:?}", inputs::permission_status());
            ExitCode::SUCCESS
        }
        #[cfg(target_os = "macos")]
        None => run_menu_bar(),
        _ => print_help(),
    }
}

fn parse_input_type(name: Option<&str>) -> Option<InputType> {
    Some(match name? {
        "vni" => InputType::Vni,
        "simple1" => InputType::SimpleTelex1,
        "simple2" => InputType::SimpleTelex2,
        _ => InputType::Telex,
    })
}

fn new_session() -> Arc<Mutex<Session>> {
    let paths = Paths::default_location();
    if paths.is_none() {
        eprintln!("No home directory; settings will not be saved.");
    }
    Arc::new(Mutex::new(Session::new(paths)))
}

fn watch_settings(session: &Arc<Mutex<Session>>, notify: &Notify) {
    if let Some(paths) = Paths::default_location() {
        let notify = Arc::clone(notify);
        reload::spawn(Arc::clone(session), paths, move || notify());
    }
}

fn run_daemon(input_type: Option<InputType>) -> ExitCode {
    if cfg!(target_os = "macos") {
        inputs::request_permission();
    }
    let session = new_session();
    if let Some(input_type) = input_type {
        if let Ok(mut session) = session.lock() {
            session.use_input_type(input_type);
        }
    }
    let notify: Notify = Arc::new(|| {});
    watch_settings(&session, &notify);
    let listener = match start_listener(&session, &notify) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            eprintln!("Permission status: {:?}", inputs::permission_status());
            return ExitCode::FAILURE;
        }
    };

    println!("VNKey is active. Press Ctrl+C to stop.");
    wait_while_healthy(&listener);
    eprintln!("The keyboard listener stopped unexpectedly.");
    ExitCode::FAILURE
}

/// Runs the menu bar UI on the main thread while keyboard capture starts in
/// the background as soon as Accessibility permission is granted.
#[cfg(target_os = "macos")]
fn run_menu_bar() -> ExitCode {
    let session = new_session();
    let refresh_session = Arc::clone(&session);
    let notify: Notify = Arc::new(move || menubar::refresh_later(Arc::clone(&refresh_session)));
    watch_settings(&session, &notify);
    watch_front_app(&session, &notify);
    let input_session = Arc::clone(&session);
    let input_notify = Arc::clone(&notify);
    thread::spawn(move || serve_input(&input_session, &input_notify));
    menubar::run(session)
}

/// Applies the remembered mode as soon as another application comes to the
/// front. `NSWorkspace` only reports the front application reliably on the
/// main thread, so the check is queued there.
#[cfg(target_os = "macos")]
fn watch_front_app(session: &Arc<Mutex<Session>>, notify: &Notify) {
    const INTERVAL: Duration = Duration::from_millis(100);
    let session = Arc::clone(session);
    let notify = Arc::clone(notify);
    thread::spawn(move || {
        loop {
            thread::sleep(INTERVAL);
            let session = Arc::clone(&session);
            let notify = Arc::clone(&notify);
            dispatch2::DispatchQueue::main().exec_async(move || {
                let Some(app) = frontmost::bundle_id() else {
                    return;
                };
                let changed = session
                    .lock()
                    .is_ok_and(|mut session| session.app_in_front(&app));
                if changed {
                    notify();
                }
            });
        }
    });
}

#[cfg(target_os = "macos")]
fn serve_input(session: &Arc<Mutex<Session>>, notify: &Notify) {
    let report = |status| {
        if session
            .lock()
            .is_ok_and(|mut session| session.set_capture_status(status))
        {
            notify();
        }
    };

    if !inputs::permission_status().is_usable() {
        report(CaptureStatus::NeedsAccessibility);
        inputs::request_permission();
    }

    loop {
        if inputs::permission_status().is_usable() {
            match start_listener(session, notify) {
                Ok(listener) => {
                    report(CaptureStatus::Active);
                    wait_while_healthy(&listener);
                    eprintln!("The keyboard listener stopped; restarting.");
                }
                Err(error) => eprintln!("{error}"),
            }
            report(if inputs::permission_status().is_usable() {
                CaptureStatus::Restarting
            } else {
                CaptureStatus::NeedsAccessibility
            });
        } else {
            report(CaptureStatus::NeedsAccessibility);
        }
        thread::sleep(Duration::from_secs(1));
    }
}

fn start_listener(session: &Arc<Mutex<Session>>, notify: &Notify) -> Result<Listener, String> {
    let signature = InjectionSignature::default();
    let keyboard = Enigo::new(&enigo::Settings {
        event_source_user_data: Some(i64::from_ne_bytes(signature.0.to_ne_bytes())),
        ..enigo::Settings::default()
    })
    .map_err(|error| format!("Cannot initialize keyboard output: {error}"))?;
    let keyboard = Mutex::new(keyboard);
    let session = Arc::clone(session);
    let notify = Arc::clone(notify);
    inputs::Listener::builder()
        .grab_mode(GrabMode::Force)
        .injection_signature(signature)
        .start(move |event: &Event| handle_event(event, &session, &keyboard, &notify))
        .map_err(|error| format!("Cannot capture keyboard input: {error}"))
}

fn wait_while_healthy(listener: &Listener) {
    while listener.is_healthy() {
        thread::sleep(Duration::from_secs(1));
    }
}

fn print_version() {
    println!("VNKey {}", env!("CARGO_PKG_VERSION"));
}

fn print_help() -> ExitCode {
    print_version();
    println!("Usage: vnkey convert [telex|vni|simple1|simple2] <text>");
    println!("       vnkey daemon [telex|vni|simple1|simple2]");
    println!("       vnkey permissions");
    ExitCode::SUCCESS
}

fn handle_event(
    event: &Event,
    session: &Mutex<Session>,
    keyboard: &Mutex<impl Keyboard>,
    notify: &Notify,
) -> EventDisposition {
    if event.injected {
        return EventDisposition::Pass;
    }
    let key = match &event.kind {
        EventKind::Key(key) => key,
        EventKind::Button(button) if button.state == PressState::Pressed => {
            if let Ok(mut session) = session.lock() {
                session.handle_mouse_down();
            }
            return EventDisposition::Pass;
        }
        _ => return EventDisposition::Pass,
    };
    let pressed = key.state == PressState::Pressed;
    let input = if pressed { key_input(key) } else { None };
    let Ok(mut locked) = session.lock() else {
        return EventDisposition::Pass;
    };
    let outcome = locked.handle_key(key.code(), pressed, input, None);
    drop(locked);
    if outcome.mode_changed {
        notify();
    }
    if outcome.suppress {
        return EventDisposition::Suppress;
    }

    let Output::Replace {
        backspaces,
        text,
        then,
    } = outcome.output
    else {
        return EventDisposition::Pass;
    };
    let Ok(mut keyboard) = keyboard.lock() else {
        return EventDisposition::Pass;
    };
    if let Err(error) = send(&mut *keyboard, backspaces, &text, then) {
        eprintln!("Cannot send keys: {error}");
    }
    EventDisposition::Suppress
}

fn send(
    keyboard: &mut impl Keyboard,
    backspaces: usize,
    text: &str,
    then: Option<Key>,
) -> Result<(), enigo::InputError> {
    for _ in 0..backspaces {
        keyboard.key(enigo::Key::Backspace, Direction::Click)?;
    }
    if !text.is_empty() {
        keyboard.text(text)?;
    }
    if let Some(key) = then.and_then(control_key) {
        keyboard.key(key, Direction::Click)?;
    }
    Ok(())
}

fn control_key(key: Key) -> Option<enigo::Key> {
    Some(match key {
        Key::Backspace => enigo::Key::Backspace,
        Key::Enter => enigo::Key::Return,
        Key::Tab => enigo::Key::Tab,
        Key::Escape => enigo::Key::Escape,
        Key::Left => enigo::Key::LeftArrow,
        Key::Right => enigo::Key::RightArrow,
        Key::Up => enigo::Key::UpArrow,
        Key::Down => enigo::Key::DownArrow,
        _ => return None,
    })
}

/// The engine input for a key press, read as a US QWERTY layout. Modifier
/// keys on their own are not reported.
fn key_input(event: &KeyEvent) -> Option<KeyInput> {
    let key = match event.code() {
        None => Key::Other,
        Some(code) => physical_key(code)?,
    };
    let modifiers = event.modifiers;
    Some(KeyInput {
        key,
        shift: modifiers.shift,
        caps_lock: modifiers.caps_lock,
        other_modifier: modifiers.control || modifiers.alt || modifiers.meta,
    })
}

fn physical_key(code: KeyCode) -> Option<Key> {
    if let Some(index) = LETTER_KEYS.iter().position(|&letter| letter == code) {
        let letter = char::from_digit(u32::try_from(index).ok()? + 10, 36)?;
        return Some(Key::Letter(letter));
    }
    if let Some(digit) = DIGIT_KEYS.iter().position(|&digit| digit == code) {
        return Some(Key::Digit(u8::try_from(digit).ok()?));
    }
    if let Some(&(_, symbol)) = SYMBOL_KEYS.iter().find(|entry| entry.0 == code) {
        return Some(Key::Symbol(symbol));
    }
    Some(match code {
        KeyCode::Space => Key::Space,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Enter | KeyCode::NumpadEnter => Key::Enter,
        KeyCode::Tab => Key::Tab,
        KeyCode::Escape => Key::Escape,
        KeyCode::ArrowLeft => Key::Left,
        KeyCode::ArrowRight => Key::Right,
        KeyCode::ArrowUp => Key::Up,
        KeyCode::ArrowDown => Key::Down,
        KeyCode::ShiftLeft
        | KeyCode::ShiftRight
        | KeyCode::ControlLeft
        | KeyCode::ControlRight
        | KeyCode::AltLeft
        | KeyCode::AltRight
        | KeyCode::MetaLeft
        | KeyCode::MetaRight
        | KeyCode::CapsLock
        | KeyCode::Fn => return None,
        _ => Key::Other,
    })
}

/// Types `input` into an empty text field, one key per character.
fn convert(input_type: InputType, input: &str) -> String {
    let mut engine = Engine::new(Config {
        input_type,
        ..Config::default()
    });
    let mut output = String::new();
    for value in input.chars() {
        let Some(input) = KeyInput::from_char(value) else {
            engine.handle_key(KeyInput::new(Key::Other));
            output.push(value);
            continue;
        };
        match engine.handle_key(input) {
            Output::Pass => output.push(value),
            Output::Replace {
                backspaces,
                text,
                then: _,
            } => {
                for _ in 0..backspaces {
                    output.pop();
                }
                output.push_str(&text);
            }
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use inputs::{Button, ButtonEvent, Modifiers, PhysicalKey};

    #[derive(Default)]
    struct TestField(String);

    impl TestField {
        fn apply(&mut self, character: char) {
            if character == '\u{8}' {
                self.0.pop().expect("a character to delete");
            } else {
                self.0.push(character);
            }
        }
    }

    impl Keyboard for TestField {
        fn fast_text(&mut self, text: &str) -> enigo::InputResult<Option<()>> {
            self.0.push_str(text);
            Ok(Some(()))
        }

        fn key(&mut self, key: enigo::Key, direction: Direction) -> enigo::InputResult<()> {
            assert_eq!(direction, Direction::Click);
            match key {
                enigo::Key::Backspace => self.apply('\u{8}'),
                enigo::Key::Return => self.apply('\n'),
                _ => panic!("unexpected injected key: {key:?}"),
            }
            Ok(())
        }

        fn raw(&mut self, keycode: u16, _: Direction) -> enigo::InputResult<()> {
            panic!("unexpected raw key: {keycode}");
        }
    }

    #[test]
    fn converts_sentences() {
        assert_eq!(convert(InputType::Telex, "Tooi ye **"), "Tôi ye **");
        assert_eq!(convert(InputType::Telex, "tienge"), "tiêng");
        assert_eq!(convert(InputType::Vni, "Vie65t Nam"), "Việt Nam");
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

    fn type_in_field(session: &Mutex<Session>, field: &Mutex<TestField>, sequence: &str) {
        let notify: Notify = Arc::new(|| {});
        for character in sequence.chars() {
            let code = match character.to_ascii_lowercase() {
                letter @ 'a'..='z' => LETTER_KEYS[(letter as usize) - ('a' as usize)],
                digit @ '0'..='9' => DIGIT_KEYS[(digit as usize) - ('0' as usize)],
                ' ' => KeyCode::Space,
                '\u{8}' => KeyCode::Backspace,
                '\n' => KeyCode::Enter,
                _ => panic!("unsupported test key: {character:?}"),
            };
            let mut stroke = key(
                code,
                Modifiers {
                    shift: character.is_ascii_uppercase(),
                    ..Modifiers::default()
                },
            );
            let press = Event::new(EventKind::Key(stroke.clone()), None, false);
            if handle_event(&press, session, field, &notify) == EventDisposition::Pass {
                field.lock().unwrap().apply(character);
            }
            stroke.state = PressState::Released;
            let release = Event::new(EventKind::Key(stroke), None, false);
            assert_eq!(
                handle_event(&release, session, field, &notify),
                EventDisposition::Pass
            );
        }
    }

    #[test]
    fn types_and_edits_vietnamese_through_the_event_handler() {
        for (method, sequence, expected) in [
            (
                InputType::Telex,
                "w ow uwo\u{8}ow ruwouj ngwoif dduwocj thuowr THUOWR\n",
                "ư ơ ươ rượu người được thuở THUỞ\n",
            ),
            (InputType::Vni, "u7 o7 uo7 ru7ou5\n", "ư ơ ươ rượu\n"),
        ] {
            let mut session = Session::new(None);
            session.use_input_type(method);
            let field = Mutex::default();
            type_in_field(&Mutex::new(session), &field, sequence);
            assert_eq!(field.into_inner().unwrap().0, expected);
        }
    }

    #[test]
    fn types_in_the_new_field_after_a_click_or_app_change() {
        let session = Mutex::new(Session::new(None));
        session.lock().unwrap().app_in_front("editor");
        let first = Mutex::default();
        let second = Mutex::default();
        let third = Mutex::default();
        type_in_field(&session, &first, "uo");

        let click = Event::new(
            EventKind::Button(ButtonEvent {
                button: Button::Left,
                state: PressState::Pressed,
            }),
            None,
            false,
        );
        let notify: Notify = Arc::new(|| {});
        assert_eq!(
            handle_event(&click, &session, &second, &notify),
            EventDisposition::Pass
        );
        type_in_field(&session, &second, "wuo");

        session.lock().unwrap().app_in_front("terminal");
        type_in_field(&session, &third, "w");
        assert_eq!(first.into_inner().unwrap().0, "uo");
        assert_eq!(second.into_inner().unwrap().0, "ưuo");
        assert_eq!(third.into_inner().unwrap().0, "ư");
    }

    fn plain(code: KeyCode) -> Option<KeyInput> {
        key_input(&key(code, Modifiers::default()))
    }

    #[test]
    fn maps_text_keys() {
        assert_eq!(plain(KeyCode::KeyA), Some(KeyInput::new(Key::Letter('a'))));
        assert_eq!(plain(KeyCode::KeyZ), Some(KeyInput::new(Key::Letter('z'))));
        assert_eq!(plain(KeyCode::Digit0), Some(KeyInput::new(Key::Digit(0))));
        assert_eq!(plain(KeyCode::Digit9), Some(KeyInput::new(Key::Digit(9))));
        assert_eq!(
            plain(KeyCode::BracketLeft),
            Some(KeyInput::new(Key::Symbol('[')))
        );
        assert_eq!(plain(KeyCode::Space), Some(KeyInput::new(Key::Space)));
        assert_eq!(plain(KeyCode::Enter), Some(KeyInput::new(Key::Enter)));
        assert_eq!(plain(KeyCode::F5), Some(KeyInput::new(Key::Other)));
    }

    #[test]
    fn ignores_lone_modifier_keys() {
        assert_eq!(plain(KeyCode::ShiftLeft), None);
        assert_eq!(plain(KeyCode::MetaRight), None);
    }

    #[test]
    fn reports_modifiers() {
        let modifiers = Modifiers {
            shift: true,
            caps_lock: true,
            meta: true,
            ..Modifiers::default()
        };
        let input = key_input(&key(KeyCode::KeyD, modifiers)).expect("key");
        assert!(input.shift && input.caps_lock && input.other_modifier);
    }
}
