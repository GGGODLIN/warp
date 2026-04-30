# Warp Fork — daily-driver additions

This is a personal fork of [warpdotdev/warp](https://github.com/warpdotdev/warp)
that ports the [cmux](https://github.com/get-convex/cmux) workflow into Warp:
folder-grouped tabs, auto-launching CLI agents, and an "app survives window
close" mode so claude / shell sessions don't get torn down every time the
last window goes away.

**Platform**: macOS only for the V4 menu-bar features. V1–V3 run on every
platform Warp itself supports.

**Branch**: `master` is fast-forwarded to `feat/folder-workspaces` so the
fork repo shows the daily-driver version at the root. The original
upstream `master` is reachable via the `upstream` remote.

---

## What changed (V1 → V4)

### V1–V2 · Sidebar folder workspaces

Group tabs into named "folder workspaces" in a cmux-style left sidebar. Each
workspace is bound to a directory; new tabs in that workspace inherit the
path. Drag headers to reorder, click to collapse, right-click for rename /
delete / set-default-command, double-click the path to edit it inline.

- New `folder_workspaces` SQLite table + `tabs.folder_workspace_id` FK
- `FolderWorkspaceModel` singleton + `FolderWorkspaceManager` CRUD
- New WarpUI view component for the sidebar
- 17 unit tests (manager + model lifecycle, fallback on delete, etc.)
- Specs under [`specs/sidebar-folder-workspaces/`](specs/sidebar-folder-workspaces/)
  ([PRODUCT](specs/sidebar-folder-workspaces/PRODUCT.md) ·
  [TECH](specs/sidebar-folder-workspaces/TECH.md) ·
  [TASKS](specs/sidebar-folder-workspaces/TASKS.md))

### V3 · Per-folder default command

Each workspace can declare a command that runs automatically in every new
tab. Defaults to `claude` so a fresh tab in a code workspace lands you
straight in a Claude Code session. Empty string = plain shell.

- `folder_workspaces.default_command` column + entity / manager setters
- New global `FolderWorkspaceSettings.default_command_for_new_workspaces`
  setting (defaults to `claude`)
- Routes through the existing `LaunchConfig::Template` path so the command
  runs *inside* the spawned shell, not as a replacement — `/quit` returns
  to a normal prompt
- Inline editor in the workspace header for editing the default command
- Right-click "Open without default command" on `+ New Tab` to one-shot
  skip the auto-launch for a single tab

### V4 · Close to menu bar (macOS)

Closing the last window hides it instead of quitting the app. The wrap
process keeps running, claude sessions / shell / scrollback survive in
memory, and a menu-bar status item lets you reopen. Mirrors cmux's
"app survives window close" flow.

- New `WindowSettings.close_to_menu_bar` toggle (default **on**, macOS only)
- `NSStatusItem` via direct ObjC FFI (`crates/warpui/src/platform/mac/objc/status_item.{h,m}`
  + Rust binding `crates/warpui/src/platform/mac/status_item.rs`); no
  `tray-icon` crate dependency
- Setting watcher (`app/src/menu_bar_status_item.rs`) installs / removes
  the icon live when the toggle changes
- `on_should_close_window` intercept hides instead of close when the
  setting is on; `on_new_window_requested` (Dock-click reopen) unhides
  the existing window instead of opening a new one
- ⌘Q (`terminate_app`) is unaffected — it still really quits

### Other niceties

- **Codesign helper** ([`script/setup-local-codesign.sh`](script/setup-local-codesign.sh)):
  one-shot bootstrap of a stable self-signed identity for local builds
  (handles OpenSSL 3.x PKCS#12 + `add-trusted-cert` sudo trap)

---

## Build & run

Same as upstream:

```sh
./script/run
```

First time on macOS, set up a stable codesign identity so subsequent rebuilds
don't re-prompt:

```sh
./script/setup-local-codesign.sh
```

Then `./script/run` builds, codesigns, and launches `WarpOss.app`.

For presubmit:

```sh
./script/presubmit
```

(needs `clang-format` for the V4 ObjC files; `brew install clang-format`)

---

## Settings

| Setting | Default | Where |
|---|---|---|
| `appearance.window.close_to_menu_bar` | `true` (macOS) | Settings → Appearance → Window |
| `folder_workspaces.default_command_for_new_workspaces` | `"claude"` | Settings (TOML) |

Per-workspace overrides for `default_command` live on each row in
`folder_workspaces.default_command` (set via the right-click menu in the
sidebar header).

---

## Layout

```
app/src/
  folder_workspace/         V1–V3 model + manager + view
  menu_bar_status_item.rs   V4 status-item lifecycle controller
  settings/folder_workspace.rs
  window_settings.rs        V4 close_to_menu_bar field
  workspace/view.rs         sidebar render + right-click menus
  workspace/view/vertical_tabs.rs

crates/warpui/src/platform/mac/
  status_item.rs            V4 NSStatusItem Rust binding
  objc/status_item.{h,m}    V4 ObjC FFI

specs/sidebar-folder-workspaces/
  PRODUCT.md  TECH.md  TASKS.md
```
