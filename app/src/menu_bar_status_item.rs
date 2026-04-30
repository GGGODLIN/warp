//! macOS menu-bar status item lifecycle.
//!
//! Singleton controller that mirrors the `close_to_menu_bar` setting: when on,
//! a [`StatusItem`] sits in the system menu bar with "Show Warp" / "Quit Warp"
//! entries. Toggling the setting installs or removes the item live without an
//! app restart.
//!
//! V4 spec: see [TASKS.md V4](file:///Users/linhancheng/Desktop/projects/warp-fork/specs/sidebar-folder-workspaces/TASKS.md).

use warpui::platform::mac::status_item::StatusItem;
use warpui::platform::TerminationMode;
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
                    Box::new(show_warp_action),
                    Box::new(quit_warp_action),
                ));
            }
            (false, true) => {
                me.item = None;
            }
            _ => {}
        }
    });
}

/// Reopen wrap when the user picks "Show Warp" from the status-item menu.
///
/// If any windows still exist (typically hidden by the V4 close intercept),
/// unhide and focus the first one — the existing process keeps its claude
/// sessions, scrollback, and shell state. Only when there are no windows at
/// all do we fall through to creating a new one (matching the Dock-click
/// reopen path in [`crate::lib::on_new_window_requested`]).
fn show_warp_action(ctx: &mut AppContext) {
    if let Some(window_id) = ctx.window_ids().next() {
        ctx.windows().show_window_and_focus_app(window_id);
        return;
    }
    crate::App::record_last_active_timestamp();
    ctx.dispatch_global_action("root_view:open_new", &());
    ctx.dispatch_global_action("workspace:save_app", &());
}

/// Real quit. Goes through `terminate_app(Cancellable)` so any unsaved-state
/// confirmation prompts run, matching `Cmd-Q` and the Dock-menu Quit item.
fn quit_warp_action(ctx: &mut AppContext) {
    ctx.terminate_app(TerminationMode::Cancellable, None);
}
