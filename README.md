# VNKey

VNKey is a GPL-3.0 Vietnamese input method written in Rust for macOS and
Linux.

## Features

- Pure Rust engine with Telex, VNI and Simple Telex input
- Modern or classic tone placement, free tone marking, and edits anywhere in
  the current word (type `tieng` then `e` to get `tiêng`)
- Spell checking that restores the typed keys for non-Vietnamese words
- Quick Telex and quick consonant shortcuts
- Unicode, TCVN3, VNI Windows, Unicode Compound and CP1258 output
- User-defined text macros through the engine API
- Native global keyboard interception on macOS and Linux
- Linux support at the `evdev`/`uinput` layer, independent of X11 or Wayland
- Small command-line converter for testing and scripting

## Build

Install the stable Rust toolchain, then run:

```sh
cargo build --release
cargo test --workspace
```

The binary is written to `target/release/vnkey`.

Build a signed local macOS application bundle:

```sh
./packaging/macos/build-app.sh
open dist/VNKey.app
```

The local bundle uses ad-hoc signing for development. A public release should
use an Apple Developer ID certificate and notarization.

## Use

Verify Telex conversion without installing the keyboard daemon:

```sh
vnkey convert telex 'Tooi ye ** Vieetj Nam'
```

Start the input daemon:

```sh
vnkey daemon telex
# or
vnkey daemon vni
# or simple1 / simple2 for Simple Telex
```

### macOS permission

The daemon requests Accessibility permission when it starts. If needed, enable
the terminal or packaged VNKey application in **System Settings -> Privacy &
Security -> Accessibility**, then restart VNKey.

Check the current state with:

```sh
vnkey permissions
```

### Linux permission

VNKey reads keyboard devices from `/dev/input` and creates a virtual keyboard
through `/dev/uinput`. Grant those permissions with your distribution's
`input`/`uinput` group or a narrowly scoped udev rule; do not run the daemon as
root. Log out and back in after changing group membership.

## Security

The daemon necessarily sees keyboard events in order to implement an input
method. Events are processed in memory only. VNKey does not log keystrokes,
write them to disk, or send them over the network. Capture failures are
fail-open so the physical keyboard remains usable.

## License and attribution

VNKey is licensed under GPL-3.0-only; see `LICENSE` and `NOTICE`.
