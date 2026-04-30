# Tech Plan: Sidebar Folder Workspaces

> **Spec phase**: Phase 2 (PLAN) — awaiting human review before Phase 3 (TASKS)
> **Companion**: [PRODUCT.md](file:///Users/linhancheng/Desktop/projects/warp-fork/specs/sidebar-folder-workspaces/PRODUCT.md)

## Major Components

| # | Component | Path | Purpose |
|---|---|---|---|
| 1 | Feature flag variant | [`crates/warp_features/`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/warp_features/) | `FolderWorkspacesEnabled` enum value，用來 gate 整個新行為 |
| 2 | DB migration | [`crates/persistence/migrations/<ts>_folder_workspaces/{up,down}.sql`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/persistence/migrations/) | 建 `folder_workspaces` 表 + tab → workspace 關聯 column / junction table |
| 3 | Diesel schema regen | [`crates/persistence/src/schema.rs`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/persistence/src/schema.rs) | `diesel print-schema` 後加進去（搭 `schema.patch`） |
| 4 | Data model | `app/src/folder_workspace/model.rs` | `FolderWorkspace { id, name, path, display_order, collapsed, created_ts }` Diesel `Queryable` / `Insertable` |
| 5 | Manager | `app/src/folder_workspace/manager.rs` | CRUD + folder `fileExists` 檢查 + bootstrap migration（既有 tab → Default workspace） |
| 6 | View | `app/src/folder_workspace/view.rs` | WarpUI collapsible header + tabs container（fluent API） |
| 7 | Sidebar integration | [`app/src/workspace/view/vertical_tabs.rs:1640-1665`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/workspace/view/vertical_tabs.rs) | feature-flag gated `if/else` 分支 |
| 8 | UI 按鈕 + folder picker | sidebar toolbar | "+" button → macOS `NSOpenPanel`（或 WarpUI helper） |
| 9 | Folder missing handler | manager + view | warning icon + tooltip + new-tab cwd fallback `$HOME` + 一次性 toast |
| 10 | Tests | inline `#[cfg(test)]` + [`crates/integration/`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/integration/) | unit + integration |

## Dependency Graph

```
1 (Feature flag) ─────────────────────────────────┐
                                                  │
2 (Migration) ──→ 3 (Schema) ──→ 4 (Model) ──→ 5 (Manager) ──┐
                                                              │
                                                  6 (View) ───┤
                                                              │
                                                  7 (Sidebar integration) ──┐
                                                                            │
                                                  8 (UI button) ────────────┤
                                                                            │
                                                  9 (Folder missing) ───────┘
                                                                            │
                                                              10 (Tests, 邊做邊寫)
```

## Implementation Order

| Step | 動作 | 驗證 checkpoint |
|---|---|---|
| **1** | 加 `FolderWorkspacesEnabled` enum variant 到 `warp_features` | `cargo build --bin warp-oss --features gui` 過 + Settings → Developer / Features 看到新 toggle（用 `runtime_flags_menu_items()` 機制）|
| **2** | 寫 migration `up.sql` / `down.sql`（建 `folder_workspaces` 表 + tab 關聯） | `diesel migration run` + `diesel migration revert` 來回過 |
| **3** | 重 gen `schema.rs` + 對照 `schema.patch` | `cargo build` 過、schema diff 只增不減 |
| **4** | 寫 `FolderWorkspace` struct + Diesel derives | `cargo build` 過 |
| **5** | 寫 manager（CRUD / fileExists / bootstrap） | unit tests 全綠 |
| **6** | 寫 view（先用 hardcoded 假資料 render） | `./script/run --dont-open && open …` 不 panic、view 元件單獨 render OK |
| **7** | Sidebar integration：`vertical_tabs.rs` 加 feature-flag gated 分支 | flag on → 看到 grouping render（hardcoded）；flag off → vanilla flat list（diff vs upstream 為 0） |
| **8** | "+" 按鈕 + macOS folder picker → call `manager.create()` | 點按鈕 → folder picker → 選資料夾 → sidebar 出現 workspace + SQLite 有 row |
| **9** | Folder missing：sidebar render 時 `fileExists` check + new-tab cwd fallback + toast | `rm -rf` folder → warning icon + tooltip；開新 tab → cwd `$HOME` + toast；`mkdir` 同名 folder → warning 自動消失 |
| **10** | Integration tests via `warp-integration-test` | 全綠 |
| **11** | Final pass | `./script/presubmit` 全綠 + manual smoke |

## Parallel Opportunities

| 平行 | 可不可以 | 備註 |
|---|---|---|
| Step 1 (FF) ‖ Step 2 (migration) | ✅ | 完全獨立，可同時做 |
| Step 6 (view 假資料) ‖ Step 4-5 (model/manager) | ✅ | view 先用 hardcoded 原型，最後接 manager；好處：早期暴露 WarpUI pattern 學習問題 |
| Step 8 (UI button) ‖ Step 9 (folder missing) | ⚠️ | 都要 manager 完成；但內部邏輯不重疊，可 parallel |
| Tests | 🔄 | 邊做邊寫（TDD-lite），不另闢 step |

其他都 sequential，因為 downstream 依賴 upstream。

## Risks & Mitigation

| 風險 | 影響 | 緩解 |
|---|---|---|
| **WarpUI Entity-Handle 學習曲線**（無社群文件） | 進度不可預測 | 先讀 [`pane_group/mod.rs:832-881`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/pane_group/mod.rs) 跟 [`text.rs:56-74`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/warpui_core/src/ui_components/text.rs) 既有 view 範例；step 6 view 先做 hardcoded 假資料原型，pattern 通了再接 manager |
| **`vertical_tabs.rs` (1500+ 行) 動到 break 既有 sidebar** | 既有使用者炸鍋 | 改動限縮在 line 1640-1665，**只加** `if FeatureFlag::is_enabled() { new } else { original }` 單一分支，default off |
| **Diesel migration 寫錯 break DB** | dev 環境 DB 壞、難 recover | down.sql 寫完整、先 dev DB 來回 `migration run`/`revert` 測；確認 `schema.rs` regen 結果只增不減 |
| **跟 cloud [`workspaces/`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/workspaces/)（複數）命名 / 概念混淆** | code review / merge 衝突 | 嚴格用 `folder_workspace` 命名，不 import `workspaces/`；commit 前 grep 確認 |
| **Upstream rebase 衝突**（Warp daily merge 大量 PR） | fork 維護成本高 | 改動限縮在新檔；`vertical_tabs.rs` 只加單一 if 分支不重排既有 line；feature flag enum 加在尾端不動順序 |
| **macOS `NSOpenPanel` 與 WarpUI 整合**（不確定有無 helper） | step 8 卡住 | 先 grep `NSOpenPanel` / `OpenPanel` 找既有 usage；若無，用 `objc` crate 直接 wrap |
| **Bootstrap migration race**（既有 tab vs Default workspace 建立順序） | 啟動時 tab 顯示在沒 workspace 的狀態 | migration 放在 app init phase（非 lazy），確保所有 tab 載入前 Default workspace 已存在 |
| **Tab → Workspace association schema 選擇**（add column vs junction table） | 影響既有 tabs 表 | 看現行 tabs 表 structure 決定（step 2 前先 grep schema.rs `tabs` 表）。**預設**：tabs 表加 `folder_workspace_id` nullable column（簡單，bootstrap migration 改一次就完成） |

## Open Tech Questions（T5 已解答 — 2026-04-29）

### Q1：Tabs 表 structure

[`crates/persistence/src/schema.rs:358-364`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/persistence/src/schema.rs)：

```
tabs (id) {
    id -> Integer,
    window_id -> Integer,
    custom_title -> Nullable<Text>,
    color -> Nullable<Text>,
}
```

FK convention：`<table>_id Integer NOT NULL` referencing PK Integer。`joinable!(tabs -> windows (window_id))` 在 schema.rs:510 下方。

**T2 設計**：
- `up.sql`：`ALTER TABLE tabs ADD COLUMN folder_workspace_id INTEGER NULL REFERENCES folder_workspaces(id)`
- schema.rs 加 `folder_workspace_id -> Nullable<Integer>` 到 `tabs!` macro + `joinable!(tabs -> folder_workspaces (folder_workspace_id))`

### Q2：Migration init callsite

- [`crates/persistence/src/lib.rs:5-6`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/persistence/src/lib.rs)：`pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");`
- [`app/src/persistence/sqlite.rs:406`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/persistence/sqlite.rs)：`conn.run_pending_migrations(persistence::MIGRATIONS)`

**結論**：Migration auto-run 在 sqlite.rs 啟動。寫好 SQL 放 [`crates/persistence/migrations/<ts>_folder_workspaces/`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/persistence/migrations/) `embed_migrations!` 自動拾取，**不用**改其他 init code。

### Q3：FeatureFlag → Settings UI mechanism

- [`app/src/app_menus.rs:8,891`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/app_menus.rs)：`debug_menu_items.extend(runtime_flags_menu_items())`
- [`crates/warp_core/src/features.rs:19-24`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/warp_core/src/features.rs)：`runtime_flags_menu_items()` iter `RUNTIME_FEATURE_FLAGS` 自動生成 menu items

**結論**：T1 已加 `FolderWorkspacesEnabled` 到 `RUNTIME_FEATURE_FLAGS` → debug menu 自動出現 toggle，**不用**改 `app_menus.rs`。前提：`RuntimeFeatureFlags` 自己 enable（dev build 預設 on，see DEBUG_FLAGS line 855）。

### Q4：Folder picker

兩個 pattern：
- (a) [`crates/warpui/src/windowing/winit/delegate.rs:333+`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/warpui/src/windowing/winit/delegate.rs)：`native_dialog::FileDialog`（純 Rust crate，cross-platform，已用於 file open / save）
- (b) [`crates/warpui/src/platform/mac/objc/window.m:847`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/warpui/src/platform/mac/objc/window.m)：直接 ObjC `NSOpenPanel`

**T9 採用 (a)**：`native_dialog::FileDialog::new().set_can_select_folders(true).show_open_single_dir()`。理由：純 Rust + 既有依賴 + delegate.rs 333-369 有 spawn-blocking pattern 可抄。

## Out of Scope (v2+)

- Cross-workspace tab drag
- Workspace 自訂 icon / color / emoji
- 跨 workspace cmd palette / search
- Cloud sync（GraphQL server 閉源 + spike 範圍）
- Upstream PR（屆時看 Issue [#2314](https://github.com/warpdotdev/Warp/issues/2314) 的 mocks 進展再決定）

---

**Phase 2 done. Awaiting human review before Phase 3 (TASKS)。**

Phase 3 用 [`agent-skills:planning-and-task-breakdown`](file:///Users/linhancheng/.claude/plugins/cache/addy-agent-skills/agent-skills/1.0.0/skills/planning-and-task-breakdown) skill 把這份 plan 拆成 ~10-15 個 acceptance-tested tasks。

---

# V2 增量技術規劃（2026-04-30 後補）

> 對齊 [PRODUCT.md V2 增量規格](file:///Users/linhancheng/Desktop/projects/warp-fork/specs/sidebar-folder-workspaces/PRODUCT.md)。本段描述「已交付」的實作 + 「待做」的設計。

## V2 已交付的組件 / 變更

### 1. `app/src/folder_workspace/entity.rs` — FolderWorkspaceModel 新方法

```rust
toggle_collapsed(id, ctx) -> Result<()>
rename_workspace(id, new_name, ctx) -> Result<()>
move_workspace(id, delta, ctx) -> Result<()>   // delta = -1 up, +1 down
delete_workspace(id, ctx) -> Result<()>        // tabs reassign to first remaining
last_active_id() -> Option<i32>                // V9 cmd+T fallback
set_last_active(id)
```

每個 mutation 都用 `establish_rw_connection` 寫 DB（同 spike pattern；待 D1 改 ModelEvent path）。

### 2. `app/src/folder_workspace/manager.rs` — 新 free fns

```rust
rename(conn, id, new_name) -> QueryResult<usize>
set_display_order(conn, id, new_order) -> QueryResult<usize>
delete_with_tab_reassignment(conn, id, fallback_id: Option<i32>) -> QueryResult<()>
```

### 3. `app/src/folder_workspace/view.rs` — FolderWorkspaceHeader 簽章變更

```rust
FolderWorkspaceHeader::new(name)
    .with_path(path)              // V6: 第二行 path
    .with_tab_count(count)        // V6: title 後綴 (N)
    .with_collapsed(bool)
    .with_folder_missing(bool)
    .with_title_color(ColorU)     // 14pt main_text_color
    .with_path_color(ColorU)      // 11pt sub_text_color
    .with_style(UiComponentStyles)
```

兩 line layout：title 14pt + path 11pt (在 collapsed/expanded 都顯示)。

### 4. `app/src/workspace/action.rs` — 新 actions

```rust
RenameFolderWorkspace { id }
MoveFolderWorkspaceUp { id }
MoveFolderWorkspaceDown { id }
DeleteFolderWorkspace { id }
ToggleFolderWorkspaceCollapsed { id }   // V3 spike 已加
AddTabToFolderWorkspace { folder_workspace_id, path }
AddFolderWorkspace { name, path }
```

### 5. `app/src/workspace/view/vertical_tabs.rs` — render path 重寫

- Header 包 `EventHandler` 同時掛 `on_left_mouse_down` (toggle collapse) + `on_right_mouse_down` (osascript "choose from list" menu)
- Header 旁 always-visible icon row：4 個 unicode icon button (`✎ ↑ ↓ ✕`)
- Tab grouping：unassigned flat → 每 ws header → indented children → `+ New Tab` → 底部 `+ Add Folder Workspace`
- **`render_terminal_row_content`** + **`render_pane_row`** 加 `folder_minimal = FolderWorkspacesEnabled.is_enabled()` 開關，drop 第二/三行 + subtitle（V10 minimal mode）

### 6. `app/src/workspace/view.rs` — action handlers + on_tab_drag constraint

- 6 個 action handler（rename / move / delete / toggle / add-tab / add-ws）
- `on_tab_drag` 加同 `folder_workspace_id` 檢查（V8）— 跨 ws drop no-op
- `assign_default_folder_workspace_to_active_tab` 改用 `last_active_id()`（V9）

## V2 待做組件（對應 PRODUCT.md V2 polish + deferred）

### P1 Drag workspace header reorder

**設計**：

- VerticalTabsPanelState 新增 `folder_workspace_drag_states: RefCell<HashMap<i32, DraggableState>>`
- VerticalTabsPanelState 新增 `folder_workspace_header_positions: RefCell<HashMap<i32, RectF>>`（透過 `SavePosition` 在 render 時 capture 每個 header 的 Y rect）
- Header `EventHandler` 外層 wrap `Draggable::new(state, element).with_drag_axis(DragAxis::VerticalOnly)`
- 對應 actions：`StartFolderWorkspaceDrag { id }` / `DragFolderWorkspace { id, position }` / `DropFolderWorkspace`
- on-drag 動作：依 drag rect.y 比對 captured positions，找最近的 neighbor，dispatch `MoveFolderWorkspaceUp/Down` 直到 swap 完成
- DB write：`set_display_order` 多次（或加 batch reorder method `reorder_all(orders: Vec<(i32, i32)>)`）

**Trade-off**：drag-and-drop UX 比 Move Up/Down 順手但需要 sidebar Y-position infra。寫一次後 V7 也可重用 pattern。

### P2 Rename inline editor

**設計**：

- 抄 `Workspace.tab_rename_editor: ViewHandle<EditorView>` pattern
- `Workspace` 加 `folder_workspace_rename_editor: ViewHandle<EditorView>` + `folder_workspace_being_renamed: Option<i32>`
- `WorkspaceState` 加 `is_folder_workspace_being_renamed()` query
- `RenameFolderWorkspace { id }` action handler 改成「進入 rename mode」而非開 osascript
- vertical_tabs.rs header render：當 `is_folder_workspace_being_renamed == Some(id)` 時 render `folder_workspace_rename_editor` 取代 title `Text`
- editor commit (Enter / blur) → dispatch `SetFolderWorkspaceName { id, name }` action → entity.rename_workspace
- Cancel (Escape) → 退出 rename mode 不寫 DB

**Trade-off**：玻璃流暢、跟 Warp 既有 rename 一致；但要 plumb editor + WorkspaceState flag + render switch。

### P3 Hover icons

**設計**：

- icon row 用 `Hoverable::new(state, |hover_state| { ... })` 包起來
- 父 header element 也 wrap `Hoverable`（共享同一個 mouse_state？或分開）
- 簡化：icon row container 用一個 `MouseStateHandle` (per-workspace) 存在 `VerticalTabsPanelState.folder_workspace_header_hover_states`，render 時 query is_hovered → 控制 opacity 0/1 或 conditional add_child

**Trade-off**：行為穩定但 hover state 要 plumb HashMap<i32, MouseStateHandle>。或更簡單：icon row 永遠 occupy 空間，opacity 0/1 切；好處不會 layout shift。

### P4 Delete confirm

**設計**：

- 看 `app/src/workspace/delete_conversation_confirmation_dialog.rs` pattern；clone 一份 `delete_folder_workspace_confirmation_dialog.rs`
- `WorkspaceState` 加 `pending_folder_workspace_delete: Option<i32>`
- `DeleteFolderWorkspace { id }` action handler 改成「show confirm dialog」
- 加新 action `ConfirmDeleteFolderWorkspace { id }` 跟 `CancelDeleteFolderWorkspace`
- Confirm → 走原 entity.delete_workspace + tab reassignment

**Trade-off**：再加一個 modal layer 但 dest UX 安全。osascript 一次性 dialog 也是 trivial fallback。

### P5 SVG icons

**設計**：

- grep `WarpIcon::DotsVertical` / `WarpIcon::Pencil` / `WarpIcon::ArrowUp` / `WarpIcon::ArrowDown` / `WarpIcon::Close` 找既有列舉
- 沒對應的就用 `to_warpui_icon(color)` 加新變體 in `crates/.../icons.rs` 或 inline `bundled/svg/<name>.svg`
- `make_icon_button` helper 改用 svg 不用 `Text`

**Trade-off**：找不到 svg 就要新增 asset，但 visual consistency 大幅提升。

### P6 event_loop_proxy folder picker + rename dialog

**設計**：

- 看 [`crates/warpui/src/windowing/winit/delegate.rs:333+`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/warpui/src/windowing/winit/delegate.rs) 既有 picker spawn pattern
- 加 `CustomEvent::FolderPicked { path }` / `CustomEvent::FolderWorkspaceRenamed { id, new_name }` 變體
- click handler `spawn` thread → `native_dialog::FileDialog` / 自製 text input modal → 結果透過 `event_loop_proxy.send_event(…)` 回 main thread
- main thread 收到 `CustomEvent` → dispatch 對應 action

**Trade-off**：production-grade、跨平台、不會 panic（不 pump main run loop）；但要 plumb event_loop_proxy 跟 CustomEvent 變體。osascript 在 spike 階段是 acceptable。

### D1 ModelEvent path for FolderWorkspace mutations

**設計**：

- `app/src/persistence/mod.rs` ModelEvent 加 4 個變體：
    ```rust
    UpsertFolderWorkspace { workspace: FolderWorkspace }
    DeleteFolderWorkspace { id: i32, fallback_id: Option<i32> }
    UpdateFolderWorkspaceCollapsed { id: i32, collapsed: bool }
    UpdateFolderWorkspaceDisplayOrder { id: i32, display_order: i32 }
    UpdateFolderWorkspaceName { id: i32, name: String }
    ```
- `app/src/persistence/sqlite.rs:601` 區塊新 match arms
- `entity.rs` 改用 `model_event_sender.send(ModelEvent::*)` 不再 fresh RW connection
- **Tentative-id 處理**：用 `max(existing.id) + 1` 當 in-memory 暫時 id；DB 真實 id 透過 startup 重 load 校準
- 移除 `crates/persistence` `establish_rw_connection` pub re-export

**Trade-off**：production 級別、合 Warp single-writer advisory；但 tentative-id 機制有重啟前 ID 不一致 edge case。

### D2 Missing folder toast

**設計**：

- 看 `app/src/workspace/toast_stack.rs` 既有 toast 系統
- `AddTabToFolderWorkspace` handler 內，`!path.exists()` 時：
    - cwd = `$HOME`
    - `self.toast_stack.update(ctx, |view, ctx| view.add_ephemeral_toast(...))`
- 一次性：加 `RefCell<HashSet<i32>>` 或 `WorkspaceState` flag 記錄 session 內哪幾個 ws 已 toast 過

**Trade-off**：純 view-side feature，零 DB write。

### D3 Integration test

**設計**：

- 抄 `crates/integration/` 既有 test pattern（用 [`warp-integration-test`](file:///Users/linhancheng/.claude/plugins/cache/warp/warp/skills/warp-integration-test/) skill 找 sample）
- 至少 1 e2e test：build → 建 ws → 加 tab → toggle collapse → restart → assert 持久化
- 進階 test：rename / delete / reorder / cross-ws drag rejection

**Trade-off**：第一個 test 學 framework 成本高；後續 test 都 cheap。

### D4 Cleanup spike-only

**Files**：

- `git revert a418008` (default-on debug build flag)
- `app/src/persistence/mod.rs` 移除 `pub use sqlite::establish_rw_connection`（D1 完成後 entity 不需要）
- `app/src/folder_workspace/mod.rs` 移除 `#![allow(dead_code)]`

## V2 Risks & Mitigation

| 風險 | 影響 | 緩解 | 對應項目 |
|---|---|---|---|
| Drag header swap thrashing（Y-position 計算 unstable） | drag UX 卡頓 | swap threshold + DragAxis::VerticalOnly + neighbor-only swap | P1 |
| Inline editor 跟 tab rename editor 互踩 | rename 行為混亂 | 獨立 `folder_workspace_rename_editor` ViewHandle + `WorkspaceState` 獨立 flag | P2 |
| Icon row hover state HashMap memory leak（刪 ws 沒清） | memory grow over time | `delete_workspace` 同步清 hover map entry | P3 |
| Delete confirm dialog 跟 conversation delete dialog 重複 boilerplate | 維護成本 | 抽 generic `delete_confirmation_dialog<T>` helper（後續 refactor） | P4 |
| Tentative-id 跟 DB id 衝突（multi-thread create） | tab 跟 ws 對應錯亂 | 序列化 in-memory create（main thread only） + 重啟重 load 校準 | D1 |
| Toast spam（同 ws 多次 missing） | UX 干擾 | session 內 `HashSet<workspace_id>` 抑制 | D2 |
| Integration test framework 不熟 | 第一個 test 工程量大 | 先 1 個 happy path test，cover create + persist + restart | D3 |

## V2 Implementation Order

| Step | Item | Verification |
|---|---|---|
| 1 | P5 svg icons（最便宜 visual upgrade） | 4 個 icon 視覺一致 |
| 2 | P3 hover icons（純 view state） | hover header 才出現 icon row |
| 3 | P4 delete confirm dialog | 點 ✕ → confirm → 真刪 / cancel → 不動 |
| 4 | P2 rename inline editor | 點 ✎ / 右鍵 Rename → editor 出現；commit / cancel work |
| 5 | P1 drag header reorder | drag header 上下 → display_order 跟著變 + DB update |
| 6 | D2 missing folder toast | tab in missing ws → cwd $HOME + toast 一次 |
| 7 | D1 ModelEvent path | 改完所有 mutation 不走 establish_rw_connection |
| 8 | D4 cleanup（依賴 D1 完成） | revert + remove pub re-export + remove dead_code allow |
| 9 | P6 event_loop_proxy picker | folder picker 不 block UI；不 panic |
| 10 | D3 integration test | `cargo nextest run` 過 |

理論上 P5/P3 可平行；P1/P2 互不依賴可平行；D1/D2 獨立；D4 必須最後。

---

**V2 tech plan done — 2026-04-30 補。Awaiting Phase 3 TASKS.md update。**

---

# V3 增量技術規劃 — Per-folder default command（2026-04-30）

> 對齊 [PRODUCT.md V3 增量規格](file:///Users/linhancheng/Desktop/projects/warp-fork/specs/sidebar-folder-workspaces/PRODUCT.md)。

## 核心設計：借 LaunchConfig CommandTemplate

wrap 已有「新 pane spawn 時跑 commands」基礎建設：

- [`app/src/launch_configs/launch_config.rs:94-113`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/launch_configs/launch_config.rs)：`PaneTemplateType::PaneTemplate { cwd, commands: Vec<CommandTemplate>, ... }`
- [`app/src/launch_configs/launch_config.rs:218-221`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/launch_configs/launch_config.rs)：`CommandTemplate { exec: String }`（命名 `exec` 是 launch config schema 字串欄位，不是 unix exec syscall）
- [`app/src/pane_group/mod.rs:813`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/pane_group/mod.rs)：`PanesLayout::Template(PaneTemplateType)` 是 `add_tab_with_pane_layout` 接受的 layout type
- 既有 callsite：[`workspace/view.rs:3595`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/workspace/view.rs) + [`workspace/view.rs:6395`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/workspace/view.rs) 透過 LaunchConfig 開 tab 都走這條

→ V3 只要在 `AddTabToFolderWorkspace` handler 內，`default_command` 有值時改用 `PanesLayout::Template(PaneTemplateType::PaneTemplate { cwd, commands: vec![CommandTemplate { exec: cmd }], ... })`。**不用碰任何 spawn / exec / shell integration code**。

## V3 待做組件

### 1. DB schema 增量

- Migration `<ts>_folder_workspace_default_command/`：
  ```sql
  -- up.sql
  ALTER TABLE folder_workspaces ADD COLUMN default_command TEXT NULL;

  -- down.sql
  ALTER TABLE folder_workspaces DROP COLUMN default_command;
  ```
- `crates/persistence/src/schema.rs`：`folder_workspaces!` macro 加 `default_command -> Nullable<Text>`
- 注意 SQLite `ALTER TABLE DROP COLUMN` 從 SQLite 3.35（2021-03）才支援，warp `rust-toolchain.toml` SQLite 版本要先確認；若不支援要走 rebuild table pattern

### 2. Model + Manager

- `app/src/folder_workspace/model.rs`：`FolderWorkspace` struct 加 `default_command: Option<String>` field
- `app/src/folder_workspace/manager.rs`：
  ```rust
  set_default_command(conn, id, command: Option<String>) -> QueryResult<usize>
  ```
- `app/src/folder_workspace/entity.rs`：
  ```rust
  set_default_command(id, command: Option<String>, ctx) -> Result<()>
  ```

### 3. Settings 全域預設值

- `crates/warp_core/src/user_preferences.rs`（或對應 settings struct）加 `default_command_for_new_folder_workspaces: String` field，default `"claude"`
- T27 先 grep `pub struct .*Settings` / `UserPreferences` 找正確 location；wrap settings infra：
  - `warp_core::user_preferences::GetUserPreferences` trait
  - `warpui_extras::user_preferences::UserPreferences` 實作
  - `warpui_extras::user_preferences::toml_backed::TomlBackedUserPreferences` 是 disk-backed impl
- 建立 ws 流程（`AddFolderWorkspace` handler）讀此 setting → 寫進新 ws.default_command

### 4. AddTabToFolderWorkspace handler 改寫

[`app/src/workspace/view.rs:20504-20557`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/workspace/view.rs) 現況：

```rust
self.add_tab_with_pane_layout(
    PanesLayout::SingleTerminal(Box::new(NewTerminalOptions {
        initial_directory,
        ..Default::default()
    })),
    ...
);
```

V3 改寫：

```rust
let workspace = FolderWorkspaceModel::handle(ctx).as_ref(ctx).get(workspace_id);
let default_command = workspace.and_then(|ws| ws.default_command.clone());

// skip_default_command 從 action payload 來（見組件 5）
let layout = if !skip_default_command && default_command.as_ref().map_or(false, |s| !s.is_empty()) {
    PanesLayout::Template(PaneTemplateType::PaneTemplate {
        cwd: initial_directory.unwrap_or_else(|| PathBuf::from(".")),
        commands: vec![CommandTemplate { exec: default_command.unwrap() }],
        is_focused: Some(true),
        pane_mode: PaneMode::Terminal,
        shell: None,
    })
} else {
    PanesLayout::SingleTerminal(Box::new(NewTerminalOptions {
        initial_directory,
        ..Default::default()
    }))
};
self.add_tab_with_pane_layout(layout, Arc::new(HashMap::new()), None, ctx);
```

### 5. AddTabToFolderWorkspace action 加 opt-out flag

[`app/src/workspace/action.rs:151`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/workspace/action.rs)：

```rust
AddTabToFolderWorkspace {
    folder_workspace_id: i32,
    path: PathBuf,
    skip_default_command: bool,   // ← 新增；既有 callsite 全填 false
}
```

既有 dispatch site（`vertical_tabs.rs:1794, 1979`）兩處改加 `skip_default_command: false`。Modifier key / 右鍵 menu 路徑 dispatch 時填 `true`。

### 6. UI plumbing

#### 6.1 Per-ws default_command edit UI

兩條路徑（T29 實作時擇一或都做）：

- **(a) 右鍵 menu** 加「Set default command...」→ dispatch `RenameFolderWorkspaceDefaultCommand { id }` action → 進 inline editor mode（抄 P2/T18 rename inline editor pattern；獨立 `folder_workspace_default_command_editor: ViewHandle<EditorView>`）
- **(b) Settings → Folder Workspaces 子頁** 列出所有 ws 跟可編輯 default_command 欄位

v3 先做 (a)，(b) 留 v4 polish。

#### 6.2 Opt-out UI

- **Modifier key**：sidebar `+ New Tab` 按鈕的 click handler 在 dispatch 時讀 `ctx.modifiers()` → 若 alt/option 按下 → `skip_default_command: true`
- **右鍵 menu**：`+ New Tab` 按鈕 wrap `EventHandler::on_right_mouse_down` → osascript "choose from list" 跳「Open with default command / Open without default command」（osascript 仍是 spike-friendly fallback；P6/T23 完成後可改 inline menu）

### 7. Settings UI（S7）

- 找 wrap 既有 settings UI 加 string editor pattern（grep `text_input` / `string_setting` / `pref_input`）
- 設定改動寫回 user prefs file（`toml_backed` 處理）
- 改設定不需 broadcast event 到既有 ws（既有 ws 已 freeze 自己的 default_command）

## V3 Risks & Mitigation

| 風險 | 影響 | 緩解 |
|---|---|---|
| SQLite ADD/DROP COLUMN 版本問題 | migration 失敗 | up.sql 用 ADD COLUMN 沒問題；down.sql 若 SQLite 版本 < 3.35 要 rebuild table — `cargo test` migration round-trip 強制驗 |
| `PanesLayout::Template` cwd 必填、Folder missing 時 fallback `$HOME` 也要傳對 | 開 tab 在錯誤目錄 | 既有 fallback 邏輯保留（line 20510-20514），只是 layout 變數型別改 |
| `CommandTemplate.exec` 是 `exec` 命名引起混淆 | 怕 spawn 取代 shell | 確認既有 callsite 行為：`exec` 字串是「shell 執行此 command」，shell 結束 command 不取代 shell — 已在 launch_config_tests.rs 看到使用 |
| Modifier key 在 click handler 內讀不到（wrap UI 框架限制） | opt-out 失效 | T30 先 spike：找個 wrap 既有 modifier-aware click 範例參考；找不到 fallback 純右鍵 menu |
| Per-ws editor 跟 P2 rename editor view 互踩 | rename 跟 set-command 行為混亂 | 獨立 ViewHandle + 獨立 WorkspaceState flag（同 P2 自己抄 tab_rename_editor 的處理）|
| 設定 default 改成 `claude` 但 user 沒裝 claude → 新 tab 顯示 `command not found` | UX 警告不足 | 接受此行為；user 會自己看到 error 然後改 setting；不做 path validation |

## V3 Implementation Order

| Step | Item | Verification |
|---|---|---|
| 1 | DB migration + schema regen + model field | `diesel migration run/revert` 來回過、`cargo build` 過、unit test (round-trip insert/query default_command) 過 |
| 2 | Manager + entity setter (`set_default_command`) | unit test 過 |
| 3 | AddTabToFolderWorkspace.skip_default_command field 加（既有 callsite 全填 false） | `cargo build` 過、行為不變 |
| 4 | Handler 改寫（讀 ws.default_command → 走 Template path 或 SingleTerminal path） | flag on + ws 設 default_command="echo hi"+ 開 tab → tab 看到 `hi` |
| 5 | Settings 全域 `default_command_for_new_folder_workspaces`（"claude" default） | grep settings infra → 加 field → AddFolderWorkspace handler 讀 setting 寫進新 ws → restart 後 setting 持久 |
| 6 | Per-ws default_command inline editor（右鍵 menu 進 mode） | 設 "echo hi" → 開 tab → 跑 |
| 7 | Opt-out modifier key + 右鍵 menu | ⌥-click 開 tab → 純 shell；右鍵 → 看到「Open without default command」|
| 8 | Settings UI（在 Settings 看到 string field） | 改 setting → 新 ws 帶新值 |
| 9 | Tests：lifecycle + opt-out + setting inheritance | manager unit + entity unit + smoke through full UI |

Step 1-4 是核心路徑（不做完功能不 work），Step 5 是 quality of life，Step 6-8 是 UI plumb，Step 9 是 regression net。

---

**V3 tech plan done — 2026-04-30。Awaiting TASKS.md V3 update。**
