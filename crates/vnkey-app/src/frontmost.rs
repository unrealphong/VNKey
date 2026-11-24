//! The application that receives typed text.

/// Bundle id of the frontmost application. Call on the main thread.
pub fn bundle_id() -> Option<String> {
    use objc2_app_kit::NSWorkspace;

    objc2::rc::autoreleasepool(|_| {
        let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;
        Some(app.bundleIdentifier()?.to_string())
    })
}
