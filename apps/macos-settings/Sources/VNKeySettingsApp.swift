import AppKit
import SwiftUI

@main
struct VNKeySettingsApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var delegate
    @StateObject private var settings = SettingsStore()
    @StateObject private var macros = MacroStore()
    @StateObject private var loginItem = LoginItem()
    @AppStorage("settingsTab") private var tab = "input"

    var body: some Scene {
        Window("Cài đặt VNKey", id: "settings") {
            TabView(selection: $tab) {
                InputTab()
                    .tabItem { Label("Bộ gõ", systemImage: "keyboard") }
                    .tag("input")
                SpellingTab()
                    .tabItem { Label("Chính tả", systemImage: "textformat.abc") }
                    .tag("spelling")
                ShortcutsTab()
                    .tabItem { Label("Gõ tắt", systemImage: "text.badge.plus") }
                    .tag("shortcuts")
                SystemTab()
                    .tabItem { Label("Hệ thống", systemImage: "gearshape") }
                    .tag("system")
            }
            .padding()
            .frame(width: 560, height: 560)
            .environmentObject(settings)
            .environmentObject(macros)
            .environmentObject(loginItem)
        }
        .windowResizability(.contentSize)
    }
}

final class AppDelegate: NSObject, NSApplicationDelegate {
    private var showSignal: DispatchSourceSignal?

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.regular)
        NSApp.activate(ignoringOtherApps: true)
        // The menu bar sends SIGUSR1 when "Cài đặt…" is chosen again.
        signal(SIGUSR1, SIG_IGN)
        let source = DispatchSource.makeSignalSource(signal: SIGUSR1, queue: .main)
        source.setEventHandler {
            NSApp.activate(ignoringOtherApps: true)
            NSApp.windows.first { $0.canBecomeKey }?.makeKeyAndOrderFront(nil)
        }
        source.resume()
        showSignal = source
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        true
    }
}
