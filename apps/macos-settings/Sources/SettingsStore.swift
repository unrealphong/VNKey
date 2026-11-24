import Foundation

/// The data directory shared with the input daemon.
enum DataDirectory {
    static let url = FileManager.default
        .urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        .appendingPathComponent("VNKey", isDirectory: true)

    static func create() {
        try? FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
    }
}

/// Reads and writes `settings.json`, and follows changes made by the menu bar.
@MainActor
final class SettingsStore: ObservableObject {
    @Published var settings: Settings {
        didSet {
            if settings != oldValue && !reloading {
                save()
            }
        }
    }

    private let file = DataDirectory.url.appendingPathComponent("settings.json")
    private var reloading = false
    private var watcher: DispatchSourceFileSystemObject?

    init() {
        DataDirectory.create()
        settings = Self.read(file) ?? Settings()
        watch()
    }

    private static func read(_ file: URL) -> Settings? {
        guard let data = try? Data(contentsOf: file) else { return nil }
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        return try? decoder.decode(Settings.self, from: data)
    }

    private func save() {
        let encoder = JSONEncoder()
        encoder.keyEncodingStrategy = .convertToSnakeCase
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        do {
            var data = try encoder.encode(settings)
            data.append(0x0A)
            try data.write(to: file, options: .atomic)
        } catch {
            NSLog("VNKey: cannot save settings: \(error)")
        }
    }

    private func reload() {
        guard let latest = Self.read(file), latest != settings else { return }
        reloading = true
        settings = latest
        reloading = false
    }

    /// Atomic saves replace the file, so watch the directory instead.
    private func watch() {
        let descriptor = open(DataDirectory.url.path, O_EVTONLY)
        guard descriptor >= 0 else { return }
        let source = DispatchSource.makeFileSystemObjectSource(
            fileDescriptor: descriptor, eventMask: .write, queue: .main)
        source.setEventHandler { [weak self] in
            MainActor.assumeIsolated { self?.reload() }
        }
        source.setCancelHandler { close(descriptor) }
        source.resume()
        watcher = source
    }
}
