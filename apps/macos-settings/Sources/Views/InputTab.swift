import SwiftUI

struct InputTab: View {
    @EnvironmentObject private var store: SettingsStore
    @State private var sample = ""

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
            Section {
                HStack(alignment: .firstTextBaseline) {
                    TextField("Nhập bằng bàn phím", text: $sample, axis: .vertical)
                        .autocorrectionDisabled()
                        .lineLimit(2...4)
                    Button("Xóa") { sample = "" }
                        .disabled(sample.isEmpty)
                }
                Text("\(sampleKeys) → ư ơ ươ rượu")
                    .font(.system(.callout, design: .monospaced))
                    .foregroundStyle(.secondary)
            } header: {
                Text("Gõ thử · \(store.settings.inputType.label)")
            } footer: {
                Text("Chọn chế độ tiếng Việt trên menu bar rồi gõ chuỗi mẫu.")
            }
        }
        .formStyle(.grouped)
    }

    private var sampleKeys: String {
        switch store.settings.inputType {
        case .vni: "u7 o7 uo7 ru7ou5"
        case .simpleTelex1: "uw ow uow ruwouj"
        case .telex, .simpleTelex2: "w ow uow ruwouj"
        }
    }
}
