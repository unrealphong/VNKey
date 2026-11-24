import SwiftUI

struct InputTab: View {
    @EnvironmentObject private var store: SettingsStore

    var body: some View {
        Form {
            Section {
                Picker("Kiểu gõ", selection: $store.settings.inputType) {
                    ForEach(InputType.allCases) { Text($0.label).tag($0) }
                }
                Picker("Bảng mã", selection: $store.settings.codeTable) {
                    ForEach(CodeTable.allCases) { Text($0.label).tag($0) }
                }
            }
            Section {
                LabeledContent("Chuyển Việt/Anh") {
                    HotkeyRecorder(hotkey: $store.settings.switchHotkey)
                }
            } footer: {
                Text("Bấm nút rồi nhấn tổ hợp phím mới. Chỉ dùng phím bổ trợ (như ⌃⇧) thì chế độ đổi khi nhả phím.")
                    .foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
    }
}
