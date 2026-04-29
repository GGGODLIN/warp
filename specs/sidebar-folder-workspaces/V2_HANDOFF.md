# V2 Handoff — Complete the cmux-style Folder Workspaces

> **Purpose**: This doc lets a fresh Claude Code session continue from where the spike (2026-04-29/30) stopped. Spike delivered foundation + scaffolding; v2 must deliver actual usable UX.
>
> **Branch state**: [`feat/folder-workspaces`](https://github.com/GGGODLIN/warp/tree/feat/folder-workspaces) — 19 commits pushed to `origin/GGGODLIN/warp`. App bundle at [`target/debug/bundle/osx/WarpOss.app`](file:///Users/linhancheng/Desktop/projects/warp-fork/target/debug/bundle/osx/WarpOss.app).
>
> **Read first**: [PRODUCT.md](file:///Users/linhancheng/Desktop/projects/warp-fork/specs/sidebar-folder-workspaces/PRODUCT.md) · [TECH.md](file:///Users/linhancheng/Desktop/projects/warp-fork/specs/sidebar-folder-workspaces/TECH.md) · [TASKS.md](file:///Users/linhancheng/Desktop/projects/warp-fork/specs/sidebar-folder-workspaces/TASKS.md) · [memory](file:///Users/linhancheng/.claude/projects/-Users-linhancheng-Desktop-projects-warp-fork/memory/project_warp_fork_spike_2026_04_29.md)

## 目標 (per user 2026-04-30)

**完整可用版本**：sidebar 像 cmux — workspace headers 可展開/收起；展開時 tabs 縮排在 header 下面；點任何 tab 切到那個 tab；點 "+" 在 workspace 內新增 tab（cwd 預設 = workspace folder）；workspace 對應一個資料夾，folder picker 選定後綁住。

**驗收 demo**（user 真的會測這個流程）：
1. Launch WarpOss
2. 看到 `▾ Default` 包現有 tab（indented children 形式）
3. 點 "+ Add Folder Workspace" → folder picker 選 `~/code/foo`
4. Sidebar 出現 `▾ foo` 空 workspace，跟 Default 平行
5. 在 foo workspace 內點 "+ New Tab"（per-workspace）→ 新 tab 出現在 foo 之下，shell cwd = `~/code/foo`
6. 點 Default header → 收起，Default 的 tabs 隱藏
7. 重啟 Warp → workspace + tabs 全還原

## 已交付 (T1-T9 + T11 + T13)

19 commits 在 `feat/folder-workspaces`。看 TASKS.md 的「Spike Outcome」段。

**現有 wiring**：
- `crates/warp_features/src/lib.rs`: `FolderWorkspacesEnabled` enum variant + 在 `RUNTIME_FEATURE_FLAGS` 列表
- `crates/persistence/migrations/2026-04-29-224002_add_folder_workspaces/{up,down}.sql`: 表 + tabs.folder_workspace_id nullable column
- `crates/persistence/src/schema.rs`: folder_workspaces table macro + `tabs.folder_workspace_id` + joinable! + allow_tables_to_appear_in_same_query
- `crates/persistence/src/model.rs`: `Tab` struct 加了 `folder_workspace_id: Option<i32>`
- `app/src/folder_workspace/`：
    - `model.rs`: `FolderWorkspace` (Queryable/Selectable/Identifiable) + `NewFolderWorkspace` (Insertable)
    - `manager.rs`: free-fn CRUD `create / get_all / get_by_id / update_collapsed / delete / bootstrap_default_workspace_for_existing_tabs` + 8 unit tests
    - `entity.rs`: `FolderWorkspaceModel: SingletonEntity` + `FolderWorkspaceEvent` enum + `create_workspace(name, path, ctx)` mutator
    - `view.rs`: `FolderWorkspaceHeader` UiComponent (name + ▾/▸ + ⚠ if folder missing) — 不接 manager，純 hardcoded data renderer
    - `mod.rs`: re-exports
- `app/src/persistence/`：
    - `mod.rs`: `PersistedData::folder_workspaces` field + re-export `establish_rw_connection`
    - `sqlite.rs`: `pub fn establish_rw_connection` + bootstrap call in `initialize` + load folder_workspaces in PersistedData
- `app/src/lib.rs`：tuple unpacking 加 `persisted_folder_workspaces` + `ctx.add_singleton_model` 註冊 FolderWorkspaceModel + `#[cfg(debug_assertions)] FolderWorkspacesEnabled` 在 `enabled_features()`
- `app/src/workspace/`：
    - `action.rs`: `WorkspaceAction::AddFolderWorkspace { name, path }` variant
    - `view.rs`: action handler dispatches `model.create_workspace` via Singleton
    - `view/vertical_tabs.rs:1640-1700` 區塊：feature-flag gated render block — 渲染 workspace headers (僅頭，無 children) + "+ Add Folder Workspace" button (osascript folder picker)

## v2 必須做的（按優先順序）

### V1: Tab → workspace association（最大塊，先做）

**目的**: tab 真的歸屬於 workspace；render 能依此分組。

#### V1.1 TabData 帶 folder_workspace_id 欄位

- [ ] 找 `TabData` struct 定義（grep `pub struct TabData` 在 `app/src/tab/` 應該有）
- [ ] 加 `pub folder_workspace_id: Option<i32>` 欄位
- [ ] 找 TabData 從 DB 載入的點（grep `Tab` model 對 `TabData` 的轉換）— 把 DB Tab 的 `folder_workspace_id` 帶進來
- [ ] 找 `NewTab` insert 點 — 新 tab 寫入 DB 時若有 active workspace 就 set `folder_workspace_id`

**Hint**: `app/src/tab.rs` 應該有相關 conversion / constructor

#### V1.2 Active workspace 概念

- [ ] 在 `VerticalTabsPanelState` (or `Workspace` struct) 加 `active_folder_workspace_id: Option<i32>`
- [ ] Default value: `bootstrap` 時的 Default workspace id
- [ ] 點 workspace header 時 update `active_folder_workspace_id`
- [ ] New tab 時讀此值寫入 `tab.folder_workspace_id`

**Decision needed in v2**: 是「last clicked」還是「explicitly chosen via menu」？建議 last-clicked 簡單；user 點任何 workspace header 或內部 tab 都更新 active state。

#### V1.3 ModelEvent path 取代 fresh RW connection

T9 用了 `establish_rw_connection` 違反 Warp single-writer。改回 ModelEvent 路徑：

- [ ] `app/src/persistence/mod.rs` ModelEvent enum 加 `UpsertFolderWorkspace { workspace: FolderWorkspace }` + `DeleteFolderWorkspace { id: i32 }` + `UpdateFolderWorkspaceCollapsed { id: i32, collapsed: bool }`
- [ ] `app/src/persistence/sqlite.rs:601` 區塊新 match arm 處理上面 3 個 events
- [ ] `entity.rs::create_workspace` 改用 sender 送 event，不用 fresh RW connection
- [ ] **Tentative-id 處理**: 用 max(existing.id) + 1 當 in-memory 暫時 id；DB 真實 id 在下次 restart 時重 load 取代
- [ ] 移除 `establish_rw_connection` pub re-export（保留 fn private），revert `crates/warp_features` 等不必要的 spike-only changes

### V2: Render tab as workspace children（grouping 視覺實作）

**目的**: cmux-like indent visual。

- [ ] 重構 `vertical_tabs.rs:1640+` render loop：原本 `for visible_tab in visible_tabs.iter()` 改為 `for workspace in fw_model.all() { render_header; for tab in workspace.tabs { render_tab_group } }`
- [ ] 沒 `folder_workspace_id` 的 tab 怎麼處理？(spike 後 bootstrap 應已把所有 tab 灌進 Default，但邊界狀態：使用者刪 Default workspace 怎辦) → 設計：未分配 tab 自動 fallback 到 Default 或 first workspace
- [ ] Indent: render_tab_group 接 `indent_level` 參數，或 wrap with Container padding-left
- [ ] Empty workspace（沒 tab）顯示什麼？建議 placeholder text "No tabs in this workspace"

### V3: Click header → collapse / expand

- [ ] 加 click handler 到 `FolderWorkspaceHeader` 構造（或 wrap with EventHandler）— **注意 NSOpenPanel-style 同樣的 borrow conflict 不適用 click handler 本身（沒 modal pump）**
- [ ] 點擊時 dispatch `WorkspaceAction::ToggleFolderWorkspaceCollapsed { id }`
- [ ] action handler 在 `view.rs` 做 `model.toggle_collapsed(id, ctx)` → 觸發 ModelEvent::UpdateFolderWorkspaceCollapsed
- [ ] render 時 if `workspace.collapsed` 跳過 children rendering
- [ ] Persistence: collapsed state 透過 V1.3 的 ModelEvent 寫 DB

### V4: Per-workspace "New Tab" button

- [ ] 每個展開的 workspace header 下方加 "+ New Tab" button
- [ ] 點擊時 dispatch action 建新 tab + 設 cwd = workspace.path + folder_workspace_id = workspace.id
- [ ] 看 `WorkspaceAction::AddTerminalTab` 等既有 add tab actions 怎麼建立 new TabData，complete the cwd / folder_workspace_id 寫入

### V5: Folder picker thread + event_loop_proxy（取代 osascript）

T9 用 osascript 是 macOS only + 阻塞 main thread。Production 路徑：

- [ ] 看 `crates/warpui/src/windowing/winit/delegate.rs:333+` Warp 既有 picker 模式
- [ ] 在 click handler spawn thread 跑 native_dialog
- [ ] 結果透過 `event_loop_proxy.send_event(CustomEvent::FolderPicked { path })` 送回 main thread
- [ ] 收到 event 後 dispatch `WorkspaceAction::AddFolderWorkspace`

### V6: Delete / Rename workspace（lifecycle）

- [ ] Right-click workspace header → context menu「Delete」「Rename」
- [ ] Delete: confirmation? spike 直接刪；tabs reassign 到 first remaining workspace 或 Default
- [ ] Rename: inline input or dialog
- [ ] 兩者透過 ModelEvent 持久化

### V7: Folder missing UX 完整化

T11 已加 ⚠ icon。補上：

- [ ] 新開 tab 在 missing-folder workspace → cwd fallback `$HOME` + 一次性 toast
- [ ] 找 toast 系統 (grep `toast_stack` 或 `notification_toast`)

### V8: Integration test

- [ ] 看 [`crates/integration`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/integration/) Builder/TestStep framework + [`warp-integration-test`](file:///Users/linhancheng/.claude/plugins/cache/warp/warp/skills/warp-integration-test/) skill
- [ ] 寫一個 e2e test：建 workspace → 加 tab → 切 workspace → 重啟 → 持久化驗證

### V9: Cleanup spike-only changes

- [ ] revert `chore(folder-workspaces): default-on in debug builds` (commit `a418008`)
- [ ] revert / replace osascript folder picker with V5 thread+event_loop_proxy
- [ ] revert `establish_rw_connection` pub re-export（V1.3 已改為 ModelEvent path）
- [ ] 確認 module-level `#![allow(dead_code)]` in `app/src/folder_workspace/mod.rs` 可移除

## Spike-time architectural notes（避免重踩坑）

1. **Render 不能讀 DB**：必須走 SingletonEntity + memory cache。原 spec 「manager.get_all() from render」**錯**。
2. **Diesel SQLite RETURNING 不支援**：`as_returning()` panics with `DoesNotSupportReturningClause` for `SelectBy<T, Sqlite>`. Fallback `last_insert_rowid()` query.
3. **Single-writer-thread 強制**：`establish_ro_connection` public、RW 是 private 因為 Warp 強制 1 writer 經 ModelEvent。**不能 spike 用 fresh RW conn 走 production**。
4. **Feature flag init 在 persistence init 之後**（lib.rs:2329 vs 1057）→ sqlite::initialize 內**不能** `is_enabled()` 查；bootstrap 必須 unconditional + idempotent。
5. **`MouseStateHandle::default()` in render 會壞**：要在 panel state struct 持有穩定 handle。
6. **NSOpenPanel modal 在 click handler 內panic**：modal 跑期間 Cocoa run loop pump async tasks，撞外層 `borrow_mut(AppContext)` → 'RefCell already borrowed'。要 spawn thread + event_loop_proxy（Warp 既有 pattern in `delegate.rs:336+`）。
7. **`vertical_tabs.rs` 動到 break sidebar 風險**：永遠包 `if FeatureFlag::is_enabled() { 新 } else { 原 }`，default off (release)，limit changes to 單一 if/else 分支。
8. **Tab struct 動 schema 後一起改**：`Queryable` 嚴格按 column order 匹配。
9. **`Diesel print-schema` 不需要**：手 edit `schema.rs` 在這 codebase 內合法（`diesel.toml schema.patch` convention）。

## Build / test commands

```bash
. "$HOME/.cargo/env"
cargo build --bin warp-oss --features gui                                          # build
cd /Users/linhancheng/Desktop/projects/warp-fork/app && cargo test --features gui --lib -- folder_workspace   # unit tests
cargo clippy --bin warp-oss --features gui --lib -- -D warnings                    # lint
export PATH="$HOME/.cargo/bin:$PATH" && cd /Users/linhancheng/Desktop/projects/warp-fork && ./script/run --dont-open  # bundle .app
osascript -e 'tell app "WarpOss" to quit' 2>/dev/null
open /Users/linhancheng/Desktop/projects/warp-fork/target/debug/bundle/osx/WarpOss.app   # launch demo
```

## Spike-only items in current branch（v2 收尾務必處理）

- 主分支 `chore(folder-workspaces): default-on in debug builds` (`a418008`) — revert（換成 user 透過 settings 開關，前提是 RuntimeFeatureFlags 自身 enabled）
- `establish_rw_connection` 為 spike 而 public — V1.3 改回 private + ModelEvent
- osascript folder picker — V5 改 thread + event_loop_proxy
- `#![allow(dead_code)]` in `app/src/folder_workspace/mod.rs` — V1/V2/V3 完成後拿掉

---

**Final session goal**: 過完 V1+V2+V3+V4，user demo 跑得到 7 步驗收流程。V5-V9 視時間補。
