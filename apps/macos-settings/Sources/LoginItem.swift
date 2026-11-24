import ServiceManagement

/// Starts VNKey.app at login through the system login items.
@MainActor
final class LoginItem: ObservableObject {
    @Published private(set) var enabled = SMAppService.mainApp.status == .enabled
    @Published var error: String?

    func set(_ enabled: Bool) {
        do {
            if enabled {
                try SMAppService.mainApp.register()
            } else {
                try SMAppService.mainApp.unregister()
            }
            error = nil
        } catch {
            self.error = error.localizedDescription
        }
        self.enabled = SMAppService.mainApp.status == .enabled
    }
}
