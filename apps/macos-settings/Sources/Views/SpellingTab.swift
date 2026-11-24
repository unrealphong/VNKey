import SwiftUI

struct SpellingTab: View {
    @EnvironmentObject private var store: SettingsStore

    var body: some View {
        Form {
            Section {
                Toggle("Kiểm tra chính tả", isOn: $store.settings.checkSpelling)
                Toggle("Khôi phục phím đã gõ khi từ sai chính tả", isOn: $store.settings.restoreIfWrongSpelling)
                    .disabled(!store.settings.checkSpelling)
            }
            Section {
                Picker("Đặt dấu", selection: $store.settings.modernOrthography) {
                    Text("Kiểu mới (oà, uý)").tag(true)
                    Text("Kiểu cũ (òa, úy)").tag(false)
                }
                Toggle("Gõ dấu tự do (không tự dời dấu)", isOn: $store.settings.freeMark)
            }
            Section {
                Toggle("Viết hoa chữ đầu câu", isOn: $store.settings.upperCaseFirstChar)
                Toggle("Cho phép từ bắt đầu bằng z, f, w, j", isOn: $store.settings.allowConsonantZfwj)
            }
        }
        .formStyle(.grouped)
    }
}
