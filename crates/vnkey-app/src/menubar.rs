use std::{
    cell::RefCell,
    process::{self, ExitCode},
    sync::{Arc, Mutex},
};

use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
use tray_icon::{
    TrayIcon, TrayIconBuilder,
    menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};
use vnkey_engine::{Engine, InputMethod};

const TOGGLE_ID: &str = "toggle";
const TELEX_ID: &str = "telex";
const VNI_ID: &str = "vni";
const QUIT_ID: &str = "quit";

struct Ui {
    tray: TrayIcon,
    toggle: CheckMenuItem,
    telex: CheckMenuItem,
    vni: CheckMenuItem,
}

impl Ui {
    fn refresh(&self, enabled: bool, method: InputMethod) {
        self.tray.set_title(Some(title(enabled)));
        if let Err(error) = self.tray.set_tooltip(Some(tooltip(enabled, method))) {
            eprintln!("Cannot update the menu bar tooltip: {error}");
        }
        self.toggle.set_checked(enabled);
        self.telex.set_checked(method == InputMethod::Telex);
        self.vni.set_checked(method == InputMethod::Vni);
    }
}

thread_local! {
    // Menu items and the status item are main-thread objects; menu events are
    // delivered on the main thread, so they are only reached from there.
    static UI: RefCell<Option<Ui>> = const { RefCell::new(None) };
}

pub fn run(engine: Arc<Mutex<Engine>>) -> ExitCode {
    let Some(mtm) = MainThreadMarker::new() else {
        eprintln!("The menu bar must run on the main thread.");
        return ExitCode::FAILURE;
    };
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

    let (enabled, method) = settings(&engine);
    let toggle = CheckMenuItem::with_id(TOGGLE_ID, "Gõ tiếng Việt", true, enabled, None);
    let telex = CheckMenuItem::with_id(TELEX_ID, "Telex", true, method == InputMethod::Telex, None);
    let vni = CheckMenuItem::with_id(VNI_ID, "VNI", true, method == InputMethod::Vni, None);
    let quit = MenuItem::with_id(QUIT_ID, "Thoát VNKey", true, None);
    let menu = Menu::new();
    if let Err(error) = menu.append_items(&[
        &toggle,
        &PredefinedMenuItem::separator(),
        &telex,
        &vni,
        &PredefinedMenuItem::separator(),
        &quit,
    ]) {
        eprintln!("Cannot build the menu bar menu: {error}");
        return ExitCode::FAILURE;
    }

    let tray = match TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_title(title(enabled))
        .with_tooltip(tooltip(enabled, method))
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
            toggle,
            telex,
            vni,
        });
    });

    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        handle_menu(&event, &engine);
    }));
    app.run();
    ExitCode::SUCCESS
}

fn handle_menu(event: &MenuEvent, engine: &Mutex<Engine>) {
    if event.id == QUIT_ID {
        process::exit(0);
    }
    {
        let Ok(mut engine) = engine.lock() else {
            return;
        };
        if event.id == TOGGLE_ID {
            let enabled = !engine.is_enabled();
            engine.set_enabled(enabled);
        } else if event.id == TELEX_ID {
            engine.set_method(InputMethod::Telex);
        } else if event.id == VNI_ID {
            engine.set_method(InputMethod::Vni);
        }
    }
    let (enabled, method) = settings(engine);
    UI.with_borrow(|ui| {
        if let Some(ui) = ui {
            ui.refresh(enabled, method);
        }
    });
}

fn settings(engine: &Mutex<Engine>) -> (bool, InputMethod) {
    engine.lock().map_or((true, InputMethod::Telex), |engine| {
        (engine.is_enabled(), engine.method())
    })
}

fn title(enabled: bool) -> &'static str {
    if enabled { "V" } else { "E" }
}

fn tooltip(enabled: bool, method: InputMethod) -> String {
    if enabled {
        format!("VNKey: Tiếng Việt ({method:?})")
    } else {
        "VNKey: English".to_owned()
    }
}
