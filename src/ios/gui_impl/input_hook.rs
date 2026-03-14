//! iOS input hook — intercepts UIWindow touch events and translates them
//! into egui pointer events.
//!
//! Strategy:
//!   Hook  `-[UIWindow sendEvent:]`  via an Objective-C method swizzle or an
//!   inline hook on the concrete IMP, then forward UITouch data to
//!   `crate::core::Gui::input`.
//!
//! # Status
//! This module is a scaffold.  The UIWindow IMP address lookup and the
//! actual hook installation will be implemented once the ObjC2 bindings
//! are confirmed to build for the game's UIWindow subclass.

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::core::{Error, Gui, Hachimi};

/// IMP of the original `-[UIWindow sendEvent:]` stored after hooking.
pub static mut SEND_EVENT_ORIG: AtomicUsize = AtomicUsize::new(0);

/// Called from the hooked `-[UIWindow sendEvent:]`.
/// `window_ptr` and `event_ptr` are raw ObjC object pointers.
pub fn on_send_event(window_ptr: usize, event_ptr: usize) {
    // When the gui is consuming input, we translate the touch event and
    // block it from reaching the game.  Otherwise we pass it through.
    if !Gui::is_consuming_input_atomic() {
        // Check for the FAB tap zone (handled entirely in egui via run_fab).
        // No additional hit-testing needed here.
        return; // let the original IMP run (called by the hook trampoline)
    }

    // TODO: extract UITouch coordinates from `event_ptr` via ObjC2
    // and push egui::Event::PointerButton / PointerMoved into gui.input.
    //
    // Skeleton:
    //   let gui = Gui::instance()?.lock().unwrap();
    //   gui.input.events.push(egui::Event::PointerButton { pos, button, pressed, modifiers });
}

fn init_internal() -> Result<(), Error> {
    // Locate the UIWindow class and `sendEvent:` selector IMP.
    // Using libc::dlsym on the ObjC runtime is the most reliable approach
    // since objc2 provides typed wrappers but we need the raw hook address.
    //
    // Pseudocode (full implementation in Phase 3):
    //   let uiwindow_class = objc2::runtime::AnyClass::get(c"UIWindow").unwrap();
    //   let sel = objc2::sel!(sendEvent:);
    //   let imp = uiwindow_class.method_implementation(sel).unwrap();
    //   Hachimi::instance().interceptor.hook(imp as usize, hooked_send_event as usize)?;

    info!("iOS: input_hook module loaded (UIWindow sendEvent: hook — Phase 3)");
    Ok(())
}

pub fn init() {
    init_internal().unwrap_or_else(|e| {
        error!("iOS input_hook init failed: {}", e);
    });
}
