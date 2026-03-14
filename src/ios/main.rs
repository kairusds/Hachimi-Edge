use crate::core::Hachimi;
use super::hook;

/// iOS entry point via the `#[ctor]` constructor attribute.
///
/// This function is called by dyld **before** the app's `main()` runs,
/// giving Hachimi a chance to install its hooks early.
///
/// Requirements for this to work without crashing:
/// - The dylib must be signed with `com.apple.security.cs.allow-jit`
///   and `com.apple.security.cs.allow-unsigned-executable-memory`
///   entitlements (see tools/ios/inject_ipa.sh).
/// - The `__RESTRICT` segment must be stripped from the host binary
///   (done automatically by inject_ipa.sh via optool).
#[ctor::ctor]
fn hachimi_ios_init() {
    if !Hachimi::init() {
        return;
    }
    hook::init();
    info!("Hachimi iOS: constructor finished");
}
