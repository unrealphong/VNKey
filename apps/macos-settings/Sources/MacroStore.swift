import AppKit
import UniformTypeIdentifiers

struct Macro: Identifiable, Equatable {
    let id = UUID()
    var name: String
    var content: String
}

/// Reads and writes `macros.txt` in the UniKey-compatible text format: a
/// header line, then one `name:content` per line.
@MainActor
final class MacroStore: ObservableObject {
    @Published private(set) var macros: [Macro] = []

    private static let header = ";Compatible VNKey Macro Data file for UniKey*** version=1 ***"
    private let file = DataDirectory.url.appendingPathComponent("macros.txt")

    init() {
        DataDirectory.create()
        if let text = try? String(contentsOf: file, encoding: .utf8) {
            macros = Self.parse(text)
        }
    }

    /// Why `name` cannot be used, or nil when it can.
    func problem(name: String, content: String, replacing id: Macro.ID?) -> String? {
        if name.isEmpty { return "Nhập từ tắt." }
        if name.contains(where: { $0 == ":" || $0.isWhitespace }) {
            return "Từ tắt không được chứa dấu cách hoặc dấu hai chấm."
        }
        if content.isEmpty { return "Nhập nội dung." }
        if macros.contains(where: { $0.name == name && $0.id != id }) {
            return "Từ tắt \"\(name)\" đã có."
        }
        return nil
    }

    func save(name: String, content: String, replacing id: Macro.ID?) {
        let content = content.replacingOccurrences(of: "\n", with: " ")
        if let id, let index = macros.firstIndex(where: { $0.id == id }) {
            macros[index].name = name
            macros[index].content = content
        } else {
            macros.append(Macro(name: name, content: content))
        }
        write()
    }

    func remove(_ ids: Set<Macro.ID>) {
        macros.removeAll { ids.contains($0.id) }
        write()
    }

    /// Adds the macros from a file; names already present are kept.
    func importFile() {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [.plainText, .text]
        guard panel.runModal() == .OK, let url = panel.url,
            let text = try? String(contentsOf: url, encoding: .utf8)
        else { return }
        for macro in Self.parse(text) where !macros.contains(where: { $0.name == macro.name }) {
            macros.append(macro)
        }
        write()
    }

    func exportFile() {
        let panel = NSSavePanel()
        panel.nameFieldStringValue = "vnkey-macros.txt"
        panel.allowedContentTypes = [.plainText]
        guard panel.runModal() == .OK, let url = panel.url else { return }
        do {
            try serialized().write(to: url, atomically: true, encoding: .utf8)
        } catch {
            NSAlert(error: error).runModal()
        }
    }

    private func write() {
        do {
            try serialized().write(to: file, atomically: true, encoding: .utf8)
        } catch {
            NSLog("VNKey: cannot save macros: \(error)")
        }
    }

    private func serialized() -> String {
        ([Self.header] + macros.map { "\($0.name):\($0.content)" }).joined(separator: "\n") + "\n"
    }

    private static func parse(_ text: String) -> [Macro] {
        var result: [Macro] = []
        for line in text.split(whereSeparator: \.isNewline).dropFirst() {
            guard let colon = line.firstIndex(of: ":") else { continue }
            var name = String(line[..<colon])
            var content = String(line[line.index(after: colon)...])
            while name.isEmpty, let next = content.firstIndex(of: ":") {
                name = ":" + content[..<next]
                content = String(content[content.index(after: next)...])
            }
            if !name.isEmpty && !result.contains(where: { $0.name == name }) {
                result.append(Macro(name: name, content: content))
            }
        }
        return result
    }
}
