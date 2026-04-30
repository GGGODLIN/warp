//! Rust binding for the `WarpStatusItem` ObjC class defined in
//! `objc/status_item.m`. Provides a single menu-bar status item with
//! "Show Warp" and "Quit Warp" entries; clicks invoke Rust closures with
//! access to the wrap [`AppContext`].
//!
//! macOS-only. Click callbacks always run on the main thread inside the
//! `callback_dispatcher().menu_item_triggered` scope, mirroring how regular
//! NSMenuItem actions enter wrap (see [`super::menus`]).

use cocoa::base::{id, nil};
use objc::{class, msg_send, sel, sel_impl};
use std::ffi::c_void;

use warpui_core::AppContext;

use super::app::callback_dispatcher;

/// Click action invoked with the wrap [`AppContext`] after the click is
/// re-dispatched onto the main UI thread.
pub type StatusItemAction = Box<dyn Fn(&mut AppContext) + 'static>;

/// Internal callback the ObjC layer drives directly. Wraps a
/// [`StatusItemAction`] so we can re-enter wrap via the callback dispatcher.
type RawCallback = Box<dyn Fn() + 'static>;

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
        show_warp: StatusItemAction,
        quit_warp: StatusItemAction,
    ) -> Self {
        let image_data = make_nsdata(image_png);
        let show_ctx = Box::into_raw(Box::new(wrap_for_dispatch(show_warp))) as *mut c_void;
        let quit_ctx = Box::into_raw(Box::new(wrap_for_dispatch(quit_warp))) as *mut c_void;
        let handle = unsafe { warp_status_item_create(image_data, show_ctx, quit_ctx) };
        StatusItem { handle }
    }
}

fn wrap_for_dispatch(action: StatusItemAction) -> RawCallback {
    Box::new(move || {
        callback_dispatcher().menu_item_triggered(|ctx| action(ctx));
    })
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
    let callback = unsafe { &*(ctx as *const RawCallback) };
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
        std::mem::drop(Box::from_raw(ctx as *mut RawCallback));
    }
}
