# Giao diện cài đặt VNKey trên macOS

## Mục tiêu

Người dùng chỉnh mọi tuỳ chọn của bộ gõ trong một cửa sổ macOS native, giống
các bộ gõ tiếng Việt quen thuộc. Thay đổi có hiệu lực ngay, không cần nút Lưu
hay khởi động lại.

Ngoài phạm vi đợt này: giao diện trên Linux (daemon Linux chỉ đọc chung file
cấu hình), âm báo khi chuyển chế độ, công cụ chuyển mã văn bản.

## Kiến trúc

`VNKey.app` chứa hai chương trình trong `Contents/MacOS`:

| Chương trình | Ngôn ngữ | Vai trò |
| --- | --- | --- |
| `vnkey` | Rust | Chạy nền trên menu bar, bắt phím, gõ tiếng Việt, phím tắt chuyển chế độ, tự chuyển theo app |
| `vnkey-settings` | SwiftUI | Cửa sổ cài đặt, chỉ chạy khi người dùng mở |

Cả hai dùng chung `Info.plist` của bundle (`vn.vnkey.app`), nên
`vnkey-settings` có cùng icon, và `SMAppService.mainApp` đăng ký chính
`VNKey.app` khi bật "Khởi động cùng máy".

### Dữ liệu dùng chung

Thư mục: `~/Library/Application Support/VNKey/` trên macOS,
`$XDG_CONFIG_HOME/vnkey/` (mặc định `~/.config/vnkey/`) trên Linux.

| File | Bên ghi | Nội dung |
| --- | --- | --- |
| `settings.json` | Swift; Rust chỉ khi đổi kiểu gõ từ menu bar | Mọi tuỳ chọn |
| `macros.txt` | Swift | Danh sách macro, định dạng của `MacroTable::to_text` |
| `state.json` | Rust | Chế độ Việt/Anh hiện tại và chế độ đã nhớ của từng app |

Mọi lần ghi đều ghi ra file tạm cùng thư mục rồi đổi tên, để bên đọc không
bao giờ thấy file dở dang.

`settings.json`:

```json
{
  "input_type": "telex",
  "code_table": "unicode",
  "switch_hotkey": { "control": true, "option": false, "command": false, "shift": true, "key": null },
  "check_spelling": true,
  "restore_if_wrong_spelling": true,
  "modern_orthography": true,
  "free_mark": false,
  "upper_case_first_char": false,
  "allow_consonant_zfwj": false,
  "quick_telex": false,
  "quick_start_consonant": false,
  "quick_end_consonant": false,
  "use_macro": false,
  "use_macro_in_english_mode": false,
  "auto_caps_macro": false,
  "smart_switch": true
}
```

- `input_type`: `telex`, `vni`, `simple_telex1`, `simple_telex2`.
- `code_table`: `unicode`, `tcvn3`, `vni_windows`, `unicode_compound`, `cp1258`.
- `switch_hotkey.key`: `null` (chỉ phím bổ trợ), hoặc tên phím `a`–`z`,
  `0`–`9`, `space`, `backquote`. Phải có ít nhất một phím bổ trợ.
- Trường thiếu lấy giá trị mặc định ở trên; trường lạ bị bỏ qua.
- "Khởi động cùng máy" không lưu ở đây: trạng thái thật nằm ở
  `SMAppService`.

`state.json`:

```json
{ "vietnamese": true, "apps": { "com.apple.TextEdit": true, "com.googlecode.iterm2": false } }
```

### Áp dụng thay đổi

- Một luồng của `vnkey` kiểm tra thời gian sửa đổi của `settings.json` và
  `macros.txt` mỗi 0,5 giây. Khi đổi, nó đọc lại, chuyển thành
  `vnkey_engine::Config` và gọi `Engine::set_config` / nạp lại `MacroTable`.
- File không có, hỏng hoặc không đọc được: dùng mặc định cho phần hỏng, ghi
  lỗi ra stderr, không ghi đè file của người dùng.
- `vnkey-settings` theo dõi `settings.json` (DispatchSource) để hiển thị đúng
  khi kiểu gõ bị đổi từ menu bar.

## Phần Rust (`crates/vnkey-app`)

Tách thành các module nhỏ, mỗi module một việc và test được riêng:

| Module | Việc |
| --- | --- |
| `settings.rs` | `Settings` (serde), đọc/ghi nguyên tử, đổi sang `Config`, đường dẫn thư mục dữ liệu |
| `state.rs` | `AppState` (chế độ hiện tại, chế độ theo app), đọc/ghi |
| `hotkey.rs` | `Hotkey` và bộ nhận diện phím tắt từ chuỗi sự kiện phím |
| `smart_switch.rs` | Logic tự chuyển: nhớ chế độ app cũ, trả về chế độ của app mới |
| `frontmost.rs` (macOS) | Bundle id của app đang ở phía trước (`NSWorkspace`, không cần `unsafe`) |
| `reload.rs` | Luồng theo dõi file và nạp lại cấu hình |
| `menubar.rs` | Thêm mục "Cài đặt…" (⌘,), cập nhật tiêu đề V/E khi chế độ đổi từ luồng khác |
| `main.rs` | Ghép các phần; chọn phím tắt trước khi đưa phím cho engine |

Phím tắt chuyển chế độ:

- Chỉ phím bổ trợ (mặc định ⌃⇧): khi tất cả phím bổ trợ đã nhả, nếu tập phím
  từng giữ đúng bằng tập cấu hình và không có phím thường nào được bấm ở giữa
  thì đổi chế độ.
- Có phím thường: bấm phím đó khi đang giữ đúng tập phím bổ trợ thì đổi chế độ
  và nuốt phím đó.

Tự chuyển theo app (khi `smart_switch` bật): mỗi lần nhấn phím, `vnkey` hỏi
bundle id của app phía trước. Nếu khác lần trước, nó lưu chế độ hiện tại cho
app cũ vào `state.json`, rồi áp chế độ đã nhớ của app mới (app chưa gặp thì
giữ nguyên chế độ hiện tại). Khi người dùng đổi chế độ, chế độ của app hiện
tại được ghi nhớ ngay.

Đổi chế độ từ luồng bắt phím: tiêu đề menu bar chỉ được sửa trên luồng chính,
nên việc cập nhật được đẩy sang hàng đợi chính bằng crate `dispatch2`.

Mục "Cài đặt…": chạy `vnkey-settings` nằm cạnh file chạy hiện tại. Nếu tiến
trình đó vẫn đang chạy, gửi `SIGUSR1` (qua lệnh `kill`) để nó đưa cửa sổ lên
trước, thay vì mở thêm cửa sổ thứ hai.

## Phần Swift (`apps/macos-settings`)

| File | Việc |
| --- | --- |
| `VNKeySettingsApp.swift` | Điểm vào, một cửa sổ "Cài đặt VNKey", hiện icon Dock khi mở, thoát khi đóng cửa sổ, nhận `SIGUSR1` để đưa cửa sổ lên trước |
| `SettingsStore.swift` | `ObservableObject` đọc/ghi `settings.json`, theo dõi file |
| `MacroStore.swift` | Đọc/ghi `macros.txt`, thêm/sửa/xoá, nhập/xuất file |
| `HotkeyRecorder.swift` | Ô bấm để ghi phím tắt |
| `LoginItem.swift` | Bật/tắt khởi động cùng máy bằng `SMAppService.mainApp` |
| `Views/*.swift` | Bốn tab |

Bốn tab:

1. **Bộ gõ**: kiểu gõ, bảng mã, phím tắt chuyển Việt/Anh.
2. **Chính tả**: kiểm tra chính tả, khôi phục phím khi sai, đặt dấu kiểu mới,
   gõ dấu tự do, viết hoa chữ đầu câu, cho phép z/f/w/j.
3. **Gõ tắt**: Quick Telex, gõ nhanh phụ âm đầu, gõ nhanh phụ âm cuối, bật
   macro, macro khi ở chế độ tiếng Anh, tự viết hoa macro, bảng macro
   (thêm, sửa, xoá, nhập/xuất file).
4. **Hệ thống**: khởi động cùng máy, tự chuyển chế độ theo app, trạng thái quyền
   Accessibility và nút mở System Settings.

Ô phụ thuộc bị làm mờ khi ô cha tắt (ví dụ "Khôi phục phím khi sai" khi tắt
kiểm tra chính tả; các ô macro khi tắt macro).

`packaging/macos/build-app.sh` biên dịch bằng
`swiftc -O -parse-as-library -target <arch>-apple-macos13.0` và chép
`vnkey-settings` vào `Contents/MacOS` trước khi ký.

## Kiểm thử

Rust (unit test):

- `settings`: mặc định, thiếu trường, trường lạ, file hỏng, ghi rồi đọc lại,
  đổi sang `Config` đúng từng trường.
- `state`: ghi rồi đọc lại, file hỏng.
- `hotkey`: ⌃⇧ nhả ra thì đổi; ⌃⇧ kèm phím khác thì không; thêm phím bổ trợ
  thứ ba thì không; phím tắt có phím thường thì đổi và nuốt phím.
- `smart_switch`: app mới chưa gặp, app đã nhớ, quay lại app cũ.

Thủ công trên máy:

- Mở "Cài đặt…" từ menu bar; mở lần hai chỉ đưa cửa sổ lên trước.
- Đổi kiểu gõ, bảng mã, từng ô chính tả và gõ tắt, kiểm tra có hiệu lực ngay
  trong TextEdit.
- Thêm macro `vn` → `Việt Nam`, gõ `vn ` ra `Việt Nam `.
- Ghi phím tắt mới, dùng được ngay; ⌃⇧ đổi V/E trên menu bar.
- Bật tự chuyển: TextEdit để Việt, Terminal để Anh, chuyển qua lại đúng.
- Bật khởi động cùng máy, thấy VNKey trong System Settings → General → Login
  Items.
