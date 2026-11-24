import AppKit

/// Mirrors `settings.json`, which the input daemon reads.
struct Settings: Codable, Equatable {
    var inputType = InputType.telex
    var codeTable = CodeTable.unicode
    var switchHotkey = Hotkey()
    var checkSpelling = true
    var restoreIfWrongSpelling = true
    var modernOrthography = true
    var freeMark = false
    var upperCaseFirstChar = false
    var allowConsonantZfwj = false
    var quickTelex = false
    var quickStartConsonant = false
    var quickEndConsonant = false
    var useMacro = false
    var useMacroInEnglishMode = false
    var autoCapsMacro = false
    var smartSwitch = true

    init() {}

    init(from decoder: Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        let defaults = Settings()
        func value<T: Decodable>(_ key: CodingKeys, _ fallback: T) -> T {
            (try? values.decodeIfPresent(T.self, forKey: key)) ?? fallback
        }
        inputType = value(.inputType, defaults.inputType)
        codeTable = value(.codeTable, defaults.codeTable)
        switchHotkey = value(.switchHotkey, defaults.switchHotkey)
        checkSpelling = value(.checkSpelling, defaults.checkSpelling)
        restoreIfWrongSpelling = value(.restoreIfWrongSpelling, defaults.restoreIfWrongSpelling)
        modernOrthography = value(.modernOrthography, defaults.modernOrthography)
        freeMark = value(.freeMark, defaults.freeMark)
        upperCaseFirstChar = value(.upperCaseFirstChar, defaults.upperCaseFirstChar)
        allowConsonantZfwj = value(.allowConsonantZfwj, defaults.allowConsonantZfwj)
        quickTelex = value(.quickTelex, defaults.quickTelex)
        quickStartConsonant = value(.quickStartConsonant, defaults.quickStartConsonant)
        quickEndConsonant = value(.quickEndConsonant, defaults.quickEndConsonant)
        useMacro = value(.useMacro, defaults.useMacro)
        useMacroInEnglishMode = value(.useMacroInEnglishMode, defaults.useMacroInEnglishMode)
        autoCapsMacro = value(.autoCapsMacro, defaults.autoCapsMacro)
        smartSwitch = value(.smartSwitch, defaults.smartSwitch)
    }
}

enum InputType: String, Codable, CaseIterable, Identifiable {
    case telex
    case vni
    case simpleTelex1 = "simple_telex1"
    case simpleTelex2 = "simple_telex2"

    var id: String { rawValue }

    var label: String {
        switch self {
        case .telex: "Telex"
        case .vni: "VNI"
        case .simpleTelex1: "Simple Telex 1"
        case .simpleTelex2: "Simple Telex 2"
        }
    }
}

enum CodeTable: String, Codable, CaseIterable, Identifiable {
    case unicode
    case tcvn3
    case vniWindows = "vni_windows"
    case unicodeCompound = "unicode_compound"
    case cp1258

    var id: String { rawValue }

    var label: String {
        switch self {
        case .unicode: "Unicode dựng sẵn"
        case .tcvn3: "TCVN3 (ABC)"
        case .vniWindows: "VNI Windows"
        case .unicodeCompound: "Unicode tổ hợp"
        case .cp1258: "Vietnamese locale CP 1258"
        }
    }
}

/// Modifiers plus an optional key. Without a key the shortcut fires when the
/// modifiers are released.
struct Hotkey: Codable, Equatable {
    var control = true
    var option = false
    var command = false
    var shift = true
    var key: String?

    init() {}

    init(modifiers: NSEvent.ModifierFlags, key: String?) {
        control = modifiers.contains(.control)
        option = modifiers.contains(.option)
        command = modifiers.contains(.command)
        shift = modifiers.contains(.shift)
        self.key = key
    }

    init(from decoder: Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        let defaults = Hotkey()
        func flag(_ key: CodingKeys, _ fallback: Bool) -> Bool {
            (try? values.decodeIfPresent(Bool.self, forKey: key)) ?? fallback
        }
        control = flag(.control, defaults.control)
        option = flag(.option, defaults.option)
        command = flag(.command, defaults.command)
        shift = flag(.shift, defaults.shift)
        key = (try? values.decodeIfPresent(String.self, forKey: .key)) ?? nil
    }

    var label: String {
        var text = ""
        if control { text += "⌃" }
        if option { text += "⌥" }
        if command { text += "⌘" }
        if shift { text += "⇧" }
        switch key {
        case nil: break
        case "space": text += " Space"
        case "backquote": text += "`"
        case let key?: text += key.uppercased()
        }
        return text.isEmpty ? "Chưa đặt" : text
    }

    /// Names of the keys the daemon understands, by macOS virtual key code.
    static let keyNames: [UInt16: String] = [
        0: "a", 11: "b", 8: "c", 2: "d", 14: "e", 3: "f", 5: "g", 4: "h", 34: "i",
        38: "j", 40: "k", 37: "l", 46: "m", 45: "n", 31: "o", 35: "p", 12: "q",
        15: "r", 1: "s", 17: "t", 32: "u", 9: "v", 13: "w", 7: "x", 16: "y", 6: "z",
        29: "0", 18: "1", 19: "2", 20: "3", 21: "4", 23: "5", 22: "6", 26: "7",
        28: "8", 25: "9", 49: "space", 50: "backquote",
    ]
}
