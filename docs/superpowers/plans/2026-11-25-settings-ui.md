# Settings UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A native macOS settings window for VNKey whose changes apply to the running input daemon immediately, plus the switch hotkey, per-app mode memory and launch at login.

**Architecture:** The Rust menu bar daemon (`vnkey`) and a SwiftUI program (`vnkey-settings`) live in `VNKey.app/Contents/MacOS` and share `settings.json`, `macros.txt` and `state.json` in the data directory. The daemon polls the two user files and reloads the engine; a `Session` type owns the engine plus hotkey and per-app state so the listener, menu bar and reloader share one lock.

**Tech Stack:** Rust 2024 (`serde`, `serde_json`, `dispatch2`, `objc2-app-kit` `NSWorkspace`), Swift 5 language mode with SwiftUI and ServiceManagement, macOS 13+.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-11-25-settings-ui-design.md`.
- Rust: `unsafe_code = "forbid"`, clippy `all` + `pedantic` clean with `-D warnings`, `cargo fmt`.
- Data directory: `~/Library/Application Support/VNKey/` (macOS), `$XDG_CONFIG_HOME/vnkey/` or `~/.config/vnkey/` (Linux).
- JSON field names exactly as in the spec; missing fields take defaults; unknown fields ignored.
- Writes are atomic (temp file in the same directory, then rename).
- Macro file header: `;Compatible VNKey Macro Data file for UniKey*** version=1 ***`.
- Commits: signed via `git commit-tree -S`, no trailer.

## Files

| Path | Responsibility |
| --- | --- |
| `crates/vnkey-engine/Cargo.toml`, `src/config.rs` | Optional `serde` feature: `Config`, `InputType`, `CodeTable` (snake_case, `#[serde(default)]`) |
| `crates/vnkey-app/src/settings.rs` | `Paths`, `Settings { config, switch_hotkey, smart_switch }`, `load`, `save`, `write_atomic` |
| `crates/vnkey-app/src/state.rs` | `AppState { vietnamese, apps }`, `load`, `save` |
| `crates/vnkey-app/src/hotkey.rs` | `Hotkey`, `HotkeyDetector`, `HotkeyAction` |
| `crates/vnkey-app/src/smart_switch.rs` | `SmartSwitch` |
| `crates/vnkey-app/src/session.rs` | `Session`: engine + detector + smart switch + state + paths |
| `crates/vnkey-app/src/reload.rs` | Poll thread reloading settings and macros |
| `crates/vnkey-app/src/frontmost.rs` | macOS frontmost bundle id |
| `crates/vnkey-app/src/menubar.rs` | "Cài đặt…" item, `refresh_later`, launching `vnkey-settings` |
| `crates/vnkey-app/src/main.rs` | Wiring |
| `apps/macos-settings/Sources/*.swift` | Settings window |
| `packaging/macos/build-app.sh` | Compile and bundle `vnkey-settings` |

---

### Task 1: Settings and state files

**Files:** engine `Cargo.toml`, `config.rs`; app `Cargo.toml`, `settings.rs`, `state.rs`, `main.rs` (module declarations).

**Interfaces (produces):**
- `vnkey_engine` feature `serde`; `Config: Serialize + Deserialize` with `#[serde(default)]`.
- `settings::Paths { settings: PathBuf, macros: PathBuf, state: PathBuf }`, `Paths::in_dir(dir: &Path) -> Paths`, `Paths::default_location() -> Option<Paths>`.
- `settings::Settings { #[serde(flatten)] config: Config, switch_hotkey: Hotkey, smart_switch: bool }` with `Default` (`smart_switch = true`), `Settings::load(path: &Path) -> Settings`, `Settings::save(&self, path: &Path) -> io::Result<()>`.
- `settings::write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()>` (creates the parent directory).
- `state::AppState { vietnamese: bool (default true), apps: BTreeMap<String, bool> }`, `AppState::load(path) -> AppState`, `AppState::save(&self, path) -> io::Result<()>`.
- `hotkey::Hotkey` is declared in Task 2 but its serde shape is needed here, so Task 1 creates `hotkey.rs` with only the struct: `Hotkey { control, option, command, shift: bool, key: Option<String> }`, `Default` = control + shift, no key.

- [ ] Tests (`settings.rs`): default round trip; `{}` gives defaults; `{"input_type":"vni","bogus":1}` gives VNI and ignores `bogus`; invalid JSON gives defaults; missing file gives defaults; every engine field name parses (`code_table":"vni_windows"`, `"simple_telex2"`); `save` then `load` equal.
- [ ] Tests (`state.rs`): missing file gives `vietnamese: true` and no apps; round trip with two apps; invalid JSON gives default.
- [ ] Implement; run `cargo test -p vnkey`; clippy; commit "Load settings and state from the data directory".

### Task 2: Hotkey detector and smart switch

**Files:** `hotkey.rs`, `smart_switch.rs`.

**Interfaces (produces):**
- `HotkeyAction { Pass, Toggle, ToggleAndSuppress }`.
- `HotkeyDetector::new(hotkey: &Hotkey) -> Self`, `set_hotkey(&mut self, hotkey: &Hotkey)`, `on_key(&mut self, code: KeyCode, pressed: bool) -> HotkeyAction`.
  - Modifier keys (`Control*`, `Alt*`, `Meta*`, `Shift*`) update a held set (left/right merged per kind). A press adds the kind to `held` and to `peak`. A release removes it; when `held` becomes empty, return `Toggle` if `peak == wanted && hotkey.key.is_none() && !interrupted`, then clear `peak` and `interrupted`.
  - Any other key press: if `hotkey.key` names this key and `held == wanted`, return `ToggleAndSuppress`; otherwise, if `held` is not empty, set `interrupted`.
  - Key names: `a`–`z`, `0`–`9`, `space`, `backquote`.
- `SmartSwitch::default()`, `app_changed(&mut self, app: &str, vietnamese: bool, state: &mut AppState) -> Option<bool>` (stores the old app's mode, returns the remembered mode of the new app, `None` when unchanged or unknown), `remember(&self, vietnamese: bool, state: &mut AppState)`.

- [ ] Tests: ⌃⇧ press/release toggles once; ⌃⇧ + `a` does not toggle on release; ⌃⌥⇧ does not toggle; release order shift-then-control still toggles; hotkey `⌘ + space` returns `ToggleAndSuppress` for space with ⌘ held and `Pass` without; smart switch: unknown app returns `None` and remembers the old app, known app returns its mode, same app returns `None`.
- [ ] Implement; test; clippy; commit "Add the switch hotkey and per-app mode memory".

### Task 3: Session, reload and menu bar

**Files:** `session.rs`, `reload.rs`, `frontmost.rs`, `menubar.rs`, `main.rs`, app `Cargo.toml` (`serde`, `serde_json`, `dispatch2`, `NSWorkspace` feature).

**Interfaces (produces):**
- `Session::new(paths: Option<Paths>) -> Session` loads settings, macros, state.
- `Session::handle_key(&mut self, event: &KeyEvent, input: Option<KeyInput>, app: Option<&str>) -> KeyOutcome` where `KeyOutcome { output: Option<Output>, suppress: bool, mode_changed: bool }`; order: smart switch, hotkey, engine.
- `Session::handle_mouse_down`, `toggle_vietnamese`, `is_vietnamese`, `input_type`, `set_input_type` (also saves `settings.json`), `apply_settings(Settings)`, `load_macros(&str)`.
- `reload::spawn(session: Arc<Mutex<Session>>, paths: Paths, on_change: impl Fn() + Send + 'static)`.
- `frontmost::bundle_id() -> Option<String>` (macOS; `None` elsewhere).
- `menubar::refresh_later(session: Arc<Mutex<Session>>)` dispatches a UI refresh to the main queue.

- [ ] Tests (`session.rs`, with a temp directory): hotkey toggles mode and saves `state.json`; app switch restores the remembered mode; `set_input_type` persists to `settings.json`; `apply_settings` changes engine behaviour (`aa` gives `â` in Telex, stays `aa` in VNI).
- [ ] Wire `main.rs` and `menubar.rs` (add "Cài đặt…" with ⌘, that launches `vnkey-settings` next to the current executable, or sends `SIGUSR1` via `kill` when it is still running).
- [ ] Test; clippy; commit "Share a session between the listener, menu bar and settings files".

### Task 4: SwiftUI settings window

**Files:** `apps/macos-settings/Sources/{VNKeySettingsApp,Settings,SettingsStore,MacroStore,HotkeyRecorder,LoginItem}.swift`, `apps/macos-settings/Sources/Views/{InputTab,SpellingTab,ShortcutsTab,SystemTab}.swift`, `packaging/macos/build-app.sh`.

- `Settings` mirrors the JSON with defaults for missing keys (custom `init(from:)`).
- `SettingsStore` (`@MainActor`, `ObservableObject`): publishes `settings`; saving writes atomically; watches the data directory with `DispatchSource` and reloads when the file content differs.
- `MacroStore`: `[Macro]`, parse/serialize with the header above, add/edit/delete, import (`NSOpenPanel`) and export (`NSSavePanel`).
- `HotkeyRecorder`: records with a local `NSEvent` monitor for `.keyDown` and `.flagsChanged`; Escape cancels; requires at least one modifier.
- `LoginItem`: `SMAppService.mainApp` status, register, unregister.
- App: one `Window`, regular activation policy while open, quits after the window closes, `SIGUSR1` brings the window to front.
- Build: `swiftc -O -swift-version 5 -parse-as-library -target "$(uname -m)-apple-macos13.0" ... -o "$app_dir/Contents/MacOS/vnkey-settings"` before signing.

- [ ] Build the app; open settings from the menu bar; walk the spec's manual checklist.
- [ ] Commit "Add the macOS settings window".

### Task 5: Docs, install, push

- [ ] README: settings window, data directory, hotkey.
- [ ] Build, install to `/Applications`, reset the Accessibility grant, ask the user to re-grant and run the manual checklist.
- [ ] Commit, `git push`.
