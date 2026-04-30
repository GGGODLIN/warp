//! Rust binding for the `WarpStatusItem` ObjC class defined in
//! `objc/status_item.m`. Provides a single menu-bar status item with
//! "Show Warp" and "Quit Warp" entries; clicks invoke Rust closures.
//!
//! macOS-only. The struct is wired up from the cross-platform layer when
//! the `close_to_menu_bar` setting is on (V4 spec).

use cocoa::base::{id, nil};
use objc::{class, msg_send, sel, sel_impl};
use std::ffi::c_void;

type ActionCallback = Box<dyn Fn() + Send + 'static>;

extern "C" {
    fn warp_status_item_create(image_data: id, show_ctx: *mut c_void, quit_ctx: *mut c_void)
        -> id;
    fn warp_status_item_destroy(item: id);
}

/// A retained handle to the macOS menu-bar status item plus the Rust-owned
/// callbacks the ObjC layer dispatches into. Drop the handle to remove the
/// status item from the menu bar; the callbacks are freed when the ObjC
/// instance deallocs.
pub struct StatusItem {
    handle: id,
}

impl StatusItem {
    /// Installs the status item in the system menu bar.
    ///
    /// `image_png` is optional template-image PNG bytes. If `None` the status
    /// item falls back to a "Warp" text label so the user can still find it.
    pub fn install(
        image_png: Option<&[u8]>,
        show_warp: ActionCallback,
        quit_warp: ActionCallback,
    ) -> Self {
        let image_data = make_nsdata(image_png);
        let show_ctx = Box::into_raw(Box::new(show_warp)) as *mut c_void;
        let quit_ctx = Box::into_raw(Box::new(quit_warp)) as *mut c_void;
        let handle = unsafe { warp_status_item_create(image_data, show_ctx, quit_ctx) };
        StatusItem { handle }
    }
}

impl Drop for StatusItem {
    fn drop(&mut self) {
        if self.handle != nil {
            unsafe { warp_status_item_destroy(self.handle) };
            self.handle = nil;
        }
    }
}

fn make_nsdata(bytes: Option<&[u8]>) -> id {
    let Some(bytes) = bytes else {
        return nil;
    };
    if bytes.is_empty() {
        return nil;
    }
    unsafe {
        let cls = class!(NSData);
        let data: id = msg_send![cls, dataWithBytes:bytes.as_ptr() as *const c_void
                                            length:bytes.len()];
        data
    }
}

#[no_mangle]
extern "C-unwind" fn warp_status_item_action_invoked(ctx: *mut c_void) {
    if ctx.is_null() {
        return;
    }
    // SAFETY: ctx was produced from `Box::into_raw(Box::new(callback))` in
    // `StatusItem::install` and stays valid for the lifetime of the ObjC
    // WarpStatusItem instance (until `warp_status_item_context_freed` is
    // called from -dealloc).
    let callback = unsafe { &*(ctx as *const ActionCallback) };
    callback();
}

#[no_mangle]
extern "C-unwind" fn warp_status_item_context_freed(ctx: *mut c_void) {
    if ctx.is_null() {
        return;
    }
    // SAFETY: balances the `Box::into_raw` in `StatusItem::install`. ObjC only
    // frees each context once, when the WarpStatusItem instance deallocates.
    unsafe {
        std::mem::drop(Box::from_raw(ctx as *mut ActionCallback));
    }
}
