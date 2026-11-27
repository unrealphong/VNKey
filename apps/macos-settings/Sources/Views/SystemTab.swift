import ApplicationServices
import SwiftUI

struct SystemTab: View {
    @EnvironmentObject private var store: SettingsStore
    @EnvironmentObject private var loginItem: LoginItem
    @State private var trusted = AXIsProcessTrusted()

    private let accessibilitySettings = URL(
        string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")!
    private let refresh = Timer.publish(every: 2, on: .main, in: .common).autoconnect()

    var body: some View {
        Form {
            Section {
                Toggle(
                    "Khởi động cùng máy",
                    isOn: Binding(get: { loginItem.enabled }, set: { loginItem.set($0) }))
                if let error = loginItem.error {
                    Text(error).font(.caption).foregroundStyle(.red)
                }
                Toggle("Tự chuyển chế độ theo ứng dụng", isOn: $store.settings.smartSwitch)
            } footer: {
                Text("VNKey nhớ mỗi ứng dụng đang gõ tiếng Việt hay tiếng Anh và tự đổi khi bạn chuyển ứng dụng.")
                    .foregroundStyle(.secondary)
            }
            Section("Quyền Accessibility") {
                LabeledContent("Trạng thái") {
                    Label(
                        trusted ? "Đã cấp quyền" : "Chưa cấp quyền",
                        systemImage: trusted ? "checkmark.circle.fill" : "exclamationmark.triangle.fill"
                    )
                    .foregroundStyle(trusted ? .green : .orange)
                }
                Button("Mở cài đặt Accessibility…") {
                    NSWorkspace.shared.open(accessibilitySettings)
                }
            }
            Section {
                LabeledContent("Phiên bản", value: version)
            }
        }
        .formStyle(.grouped)
        .onReceive(refresh) { _ in trusted = AXIsProcessTrusted() }
    }

    private var version: String {
        guard let version = Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString")
            as? String else { return "Không rõ" }
        if let build = Bundle.main.object(forInfoDictionaryKey: "CFBundleVersion") as? String {
            return "\(version) (bản dựng \(build))"
        }
        return version
    }
}
