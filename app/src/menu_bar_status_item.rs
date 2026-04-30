//! macOS menu-bar status item lifecycle.
//!
//! Singleton controller that mirrors the `close_to_menu_bar` setting: when on,
//! a [`StatusItem`] sits in the system menu bar with "Show Warp" / "Quit Warp"
//! entries. Toggling the setting installs or removes the item live without an
//! app restart.
//!
//! V4 spec: see [TASKS.md `T35`](file:///Users/linhancheng/Desktop/projects/warp-fork/specs/sidebar-folder-workspaces/TASKS.md).
//! T35 wires lifecycle only; menu-item callbacks log placeholders that T36
//! will replace with real Show/Quit dispatch via the winit event loop proxy.

use warpui::platform::mac::status_item::StatusItem;
use warpui::{AppContext, Entity, ModelContext, SingletonEntity};

use crate::window_settings::{WindowSettings, WindowSettingsChangedEvent};

pub struct MenuBarStatusItemController {
    item: Option<StatusItem>,
}

impl Entity for MenuBarStatusItemController {
    type Event = ();
}

impl SingletonEntity for MenuBarStatusItemController {}

impl MenuBarStatusItemController {
    fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self { item: None }
    }
}

/// Registers the singleton, applies the current setting, and subscribes to
/// future `close_to_menu_bar` toggles. Call once during app init.
pub fn init(ctx: &mut AppContext) {
    ctx.add_singleton_model(MenuBarStatusItemController::new);
    refresh(ctx);
    ctx.subscribe_to_model(&WindowSettings::handle(ctx), |_, event, ctx| {
        if matches!(event, WindowSettingsChangedEvent::CloseToMenuBar { .. }) {
            refresh(ctx);
        }
    });
}

fn refresh(ctx: &mut AppContext) {
    let enabled = *WindowSettings::as_ref(ctx).close_to_menu_bar;
    MenuBarStatusItemController::handle(ctx).update(ctx, |me, _ctx| {
        match (enabled, me.item.is_some()) {
            (true, false) => {
                me.item = Some(StatusItem::install(
                    None,
                    Box::new(|| {
                        log::info!(
                            "[menu-bar-status-item] Show Warp clicked (T36 will dispatch reopen)"
                        );
                    }),
                    Box::new(|| {
                        log::info!(
                            "[menu-bar-status-item] Quit Warp clicked (T36 will dispatch quit)"
                        );
                    }),
                ));
            }
            (false, true) => {
                me.item = None;
            }
            _ => {}
        }
    });
}
