import AppKit
import SwiftUI

/// A button that records the next shortcut the user presses.
struct HotkeyRecorder: View {
    @Binding var hotkey: Hotkey
    @State private var recording = false
    @State private var monitor: Any?
    @State private var peak: NSEvent.ModifierFlags = []

    private static let modifierKeys: NSEvent.ModifierFlags = [.control, .option, .command, .shift]
    private static let escape: UInt16 = 53

    var body: some View {
        Button(recording ? "Nhấn phím tắt… (Esc để huỷ)" : hotkey.label) {
            recording ? stop() : start()
        }
        .onDisappear(perform: stop)
    }

    private func start() {
        peak = []
        recording = true
        monitor = NSEvent.addLocalMonitorForEvents(matching: [.keyDown, .flagsChanged]) { event in
            handle(event)
            return nil
        }
    }

    private func stop() {
        if let monitor {
            NSEvent.removeMonitor(monitor)
        }
        monitor = nil
        recording = false
    }

    private func handle(_ event: NSEvent) {
        let modifiers = event.modifierFlags.intersection(Self.modifierKeys)
        if event.type == .flagsChanged {
            if modifiers.isEmpty {
                if !peak.isEmpty {
                    hotkey = Hotkey(modifiers: peak, key: nil)
                    stop()
                }
            } else {
                peak.formUnion(modifiers)
            }
            return
        }
        if event.keyCode == Self.escape && modifiers.isEmpty {
            stop()
            return
        }
        guard !modifiers.isEmpty, let name = Hotkey.keyNames[event.keyCode] else {
            NSSound.beep()
            return
        }
        hotkey = Hotkey(modifiers: modifiers, key: name)
        stop()
    }
}
