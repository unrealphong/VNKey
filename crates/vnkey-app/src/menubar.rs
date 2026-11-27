use std::{
    cell::RefCell,
    env,
    process::{self, Child, Command, ExitCode},
    sync::{Arc, Mutex},
};

use dispatch2::DispatchQueue;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
use tray_icon::{
    TrayIcon, TrayIconBuilder,
    menu::{
        CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem,
        accelerator::{Accelerator, Code, Modifiers},
    },
};
use vnkey_engine::InputType;

use crate::session::{CaptureStatus, Session};

const CAPTURE_STATUS_ID: &str = "capture-status";
const TOGGLE_ID: &str = "toggle";
const TELEX_ID: &str = "telex";
const VNI_ID: &str = "vni";
const SETTINGS_ID: &str = "settings";
const QUIT_ID: &str = "quit";
const SETTINGS_PROGRAM: &str = "vnkey-settings";
const ACCESSIBILITY_SETTINGS: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";

struct Ui {
    tray: TrayIcon,
    capture_status: MenuItem,
    toggle: CheckMenuItem,
    telex: CheckMenuItem,
    vni: CheckMenuItem,
}

impl Ui {
    fn refresh(&self, enabled: bool, method: InputType, capture: CaptureStatus) {
        self.tray.set_title(Some(title(enabled, capture)));
        if let Err(error) = self
            .tray
            .set_tooltip(Some(tooltip(enabled, method, capture)))
        {
            eprintln!("Cannot update the menu bar tooltip: {error}");
        }
        self.capture_status.set_text(capture_label(capture));
        self.capture_status
            .set_enabled(capture == CaptureStatus::NeedsAccessibility);
        self.toggle.set_checked(enabled);
        self.telex.set_checked(method == InputType::Telex);
        self.vni.set_checked(method == InputType::Vni);
    }
}

thread_local! {
    // Menu items and the status item are main-thread objects; menu events are
    // delivered on the main thread, so they are only reached from there.
    static UI: RefCell<Option<Ui>> = const { RefCell::new(None) };
}

static SETTINGS_WINDOW: Mutex<Option<Child>> = Mutex::new(None);

pub fn run(session: Arc<Mutex<Session>>) -> ExitCode {
    let Some(mtm) = MainThreadMarker::new() else {
        eprintln!("The menu bar must run on the main thread.");
        return ExitCode::FAILURE;
    };
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

    let (enabled, method, capture) = current(&session);
    let capture_status = MenuItem::with_id(
        CAPTURE_STATUS_ID,
        capture_label(capture),
        capture == CaptureStatus::NeedsAccessibility,
        None,
    );
    let toggle = CheckMenuItem::with_id(TOGGLE_ID, "Gõ tiếng Việt", true, enabled, None);
    let telex = CheckMenuItem::with_id(TELEX_ID, "Telex", true, method == InputType::Telex, None);
    let vni = CheckMenuItem::with_id(VNI_ID, "VNI", true, method == InputType::Vni, None);
    let settings = MenuItem::with_id(
        SETTINGS_ID,
        "Cài đặt…",
        true,
        Some(Accelerator::new(Modifiers::META, Code::Comma)),
    );
    let quit = MenuItem::with_id(QUIT_ID, "Thoát VNKey", true, None);
    let menu = Menu::new();
    if let Err(error) = menu.append_items(&[
        &capture_status,
        &toggle,
        &PredefinedMenuItem::separator(),
        &telex,
        &vni,
        &PredefinedMenuItem::separator(),
        &settings,
        &quit,
    ]) {
        eprintln!("Cannot build the menu bar menu: {error}");
        return ExitCode::FAILURE;
    }

    let tray = match TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_title(title(enabled, capture))
        .with_tooltip(tooltip(enabled, method, capture))
        .build()
    {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Cannot create the menu bar item: {error}");
            return ExitCode::FAILURE;
        }
    };
    UI.with_borrow_mut(|ui| {
        *ui = Some(Ui {
            tray,
            capture_status,
            toggle,
            telex,
            vni,
        });
    });

    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        handle_menu(&event, &session);
    }));
    app.run();
    ExitCode::SUCCESS
}

/// Refreshes the menu bar from any thread.
pub fn refresh_later(session: Arc<Mutex<Session>>) {
    DispatchQueue::main().exec_async(move || refresh(&session));
}

fn refresh(session: &Mutex<Session>) {
    let (enabled, method, capture) = current(session);
    UI.with_borrow(|ui| {
        if let Some(ui) = ui {
            ui.refresh(enabled, method, capture);
        }
    });
}

fn handle_menu(event: &MenuEvent, session: &Mutex<Session>) {
    if event.id == CAPTURE_STATUS_ID {
        if current(session).2 == CaptureStatus::NeedsAccessibility {
            if let Err(error) = Command::new("/usr/bin/open")
                .arg(ACCESSIBILITY_SETTINGS)
                .status()
            {
                eprintln!("Cannot open Accessibility settings: {error}");
            }
        }
        return;
    }
    if event.id == QUIT_ID {
        process::exit(0);
    }
    if event.id == SETTINGS_ID {
        open_settings();
        return;
    }
    if let Ok(mut session) = session.lock() {
        if event.id == TOGGLE_ID {
            session.toggle_vietnamese();
        } else if event.id == TELEX_ID {
            session.set_input_type(InputType::Telex);
        } else if event.id == VNI_ID {
            session.set_input_type(InputType::Vni);
        }
    }
    refresh(session);
}

/// Starts the settings window, or brings the running one to the front.
fn open_settings() {
    let Ok(mut window) = SETTINGS_WINDOW.lock() else {
        return;
    };
    if let Some(child) = window.as_mut() {
        if matches!(child.try_wait(), Ok(None)) {
            if let Err(error) = Command::new("/bin/kill")
                .args(["-USR1", &child.id().to_string()])
                .status()
            {
                eprintln!("Cannot reach the settings window: {error}");
            }
            return;
        }
    }
    let program = env::current_exe()
        .ok()
        .and_then(|exe| Some(exe.parent()?.join(SETTINGS_PROGRAM)));
    let Some(program) = program else {
        eprintln!("Cannot locate {SETTINGS_PROGRAM}.");
        return;
    };
    match Command::new(&program).spawn() {
        Ok(child) => *window = Some(child),
        Err(error) => eprintln!("Cannot start {}: {error}", program.display()),
    }
}

fn current(session: &Mutex<Session>) -> (bool, InputType, CaptureStatus) {
    session.lock().map_or(
        (true, InputType::Telex, CaptureStatus::Restarting),
        |session| {
            (
                session.is_vietnamese(),
                session.input_type(),
                session.capture_status(),
            )
        },
    )
}

fn method_name(method: InputType) -> &'static str {
    match method {
        InputType::Telex => "Telex",
        InputType::Vni => "VNI",
        InputType::SimpleTelex1 => "Simple Telex 1",
        InputType::SimpleTelex2 => "Simple Telex 2",
    }
}

fn title(enabled: bool, capture: CaptureStatus) -> &'static str {
    match (enabled, capture) {
        (true, CaptureStatus::Active) => "V",
        (false, CaptureStatus::Active) => "E",
        (true, CaptureStatus::NeedsAccessibility) => "V!",
        (false, CaptureStatus::NeedsAccessibility) => "E!",
        (true, CaptureStatus::Starting | CaptureStatus::Restarting) => "V…",
        (false, CaptureStatus::Starting | CaptureStatus::Restarting) => "E…",
    }
}

fn capture_label(capture: CaptureStatus) -> &'static str {
    match capture {
        CaptureStatus::Starting => "Đang khởi động bộ gõ…",
        CaptureStatus::Active => "Bộ gõ đang hoạt động",
        CaptureStatus::NeedsAccessibility => "Thiếu quyền Accessibility…",
        CaptureStatus::Restarting => "Đang khởi động lại bộ gõ…",
    }
}

fn tooltip(enabled: bool, method: InputType, capture: CaptureStatus) -> String {
    let mode = if enabled {
        format!("Tiếng Việt ({})", method_name(method))
    } else {
        "English".to_owned()
    };
    format!("VNKey: {mode}\n{}", capture_label(capture))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_bar_title_shows_capture_health_in_each_mode() {
        assert_eq!(title(true, CaptureStatus::Active), "V");
        assert_eq!(title(false, CaptureStatus::Active), "E");
        assert_eq!(title(true, CaptureStatus::NeedsAccessibility), "V!");
        assert_eq!(title(false, CaptureStatus::NeedsAccessibility), "E!");
        assert_eq!(title(true, CaptureStatus::Starting), "V…");
        assert_eq!(title(false, CaptureStatus::Restarting), "E…");
    }
}
