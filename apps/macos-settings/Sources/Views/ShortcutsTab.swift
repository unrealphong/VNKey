import SwiftUI

struct ShortcutsTab: View {
    @EnvironmentObject private var store: SettingsStore
    @EnvironmentObject private var macros: MacroStore
    @State private var selection = Set<Macro.ID>()
    @State private var name = ""
    @State private var content = ""

    var body: some View {
        Form {
            Section("Gõ nhanh") {
                Toggle("Quick Telex (cc → ch, gg → gi, nn → ng…)", isOn: $store.settings.quickTelex)
                Toggle("Phụ âm đầu (f → ph, j → gi, w → qu)", isOn: $store.settings.quickStartConsonant)
                Toggle("Phụ âm cuối (g → ng, h → nh, k → ch)", isOn: $store.settings.quickEndConsonant)
            }
            Section("Macro") {
                Toggle("Bật macro", isOn: $store.settings.useMacro)
                Toggle("Dùng macro cả khi gõ tiếng Anh", isOn: $store.settings.useMacroInEnglishMode)
                    .disabled(!store.settings.useMacro)
                Toggle("Tự viết hoa theo từ tắt (Btw → By the way)", isOn: $store.settings.autoCapsMacro)
                    .disabled(!store.settings.useMacro)
                macroTable
                macroEditor
            }
        }
        .formStyle(.grouped)
    }

    private var editing: Macro.ID? {
        selection.count == 1 ? selection.first : nil
    }

    private var macroTable: some View {
        Table(macros.macros, selection: $selection) {
            TableColumn("Từ tắt", value: \.name).width(min: 60, ideal: 90, max: 140)
            TableColumn("Nội dung", value: \.content)
        }
        .frame(height: 150)
        .onChange(of: selection) { _ in
            if let id = editing, let macro = macros.macros.first(where: { $0.id == id }) {
                name = macro.name
                content = macro.content
            }
        }
    }

    private var macroEditor: some View {
        let problem = macros.problem(name: name, content: content, replacing: editing)
        return VStack(alignment: .leading, spacing: 8) {
            HStack {
                TextField("Từ tắt", text: $name)
                    .frame(width: 110)
                TextField("Nội dung", text: $content)
            }
            HStack {
                Button(editing == nil ? "Thêm" : "Cập nhật") {
                    macros.save(name: name, content: content, replacing: editing)
                    name = ""
                    content = ""
                    selection = []
                }
                .disabled(problem != nil)
                Button("Xoá") {
                    macros.remove(selection)
                    selection = []
                    name = ""
                    content = ""
                }
                .disabled(selection.isEmpty)
                Spacer()
                Button("Nhập…") { macros.importFile() }
                Button("Xuất…") { macros.exportFile() }
            }
            if let problem, !name.isEmpty || !content.isEmpty {
                Text(problem).font(.caption).foregroundStyle(.secondary)
            }
        }
    }
}
