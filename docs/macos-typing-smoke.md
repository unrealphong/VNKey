# macOS typing smoke test

Use a physical keyboard with the macOS ABC or U.S. input source. In VNKey,
select Telex and Unicode dựng sẵn, enable spelling restoration, and turn on
Vietnamese for the app you are testing. Start with an empty field in each app.

| App | Test field |
| --- | --- |
| TextEdit | New plain text document (Shift–Command–T to switch to plain text) |
| Chrome | Open `data:text/html,%3Ctextarea%20autofocus%3E%3C%2Ftextarea%3E` and click the textarea |
| VS Code | New untitled file in Plain Text mode |
| Terminal | Run `read -r vnkey_sample && printf '%s\n' "$vnkey_sample"` and type into the waiting input |
| VNKey | Cài đặt → Bộ gõ → Gõ thử |

| Action | Expected result |
| --- | --- |
| Type `w ww ow uwo uow uwow` | `ư w ơ ươ ươ ươ` |
| Type `ruwouj ngwoif dduwocj thuowr THUOWR` at normal speed, then again quickly | `rượu người được thuở THUỞ` both times |
| Type `uwo`, press Backspace, then type `ow` | `ươ`, then `ư`, then `ươ` |
| Type `uo` in one field, click another empty field, and press `w` | The first field keeps `uo` and the second gets `ư` |
| Type `uo`, switch to a different app that is already in Vietnamese mode, and press `w` in an empty field | The first app keeps `uo` and the second gets `ư` |
| Switch to VNI and type `u7 o7 uo7 ru7ou5` | `ư ơ ươ rượu` |

In the VNKey test field, change the input method and check that the sample
updates. Clear the field and check that clearing the sample leaves settings
as they were. In the menu bar, Vietnamese and English show `V` and `E` while
capture is active. Without Accessibility permission, the title ends in `!`
and the status item opens Accessibility settings. After granting permission,
the title returns to `V` or `E` when capture starts.

Record the app version, macOS version, input source, key sequence, and actual
text for any discrepancy. The Rust event-handler tests use an in-memory text
field and can run without a macOS Accessibility grant.
