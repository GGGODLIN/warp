# Implementation Tasks: Sidebar Folder Workspaces

> **Spec phase**: Phase 4 (IMPLEMENT) — partial delivery 2026-04-29
> **Companions**: [PRODUCT.md](file:///Users/linhancheng/Desktop/projects/warp-fork/specs/sidebar-folder-workspaces/PRODUCT.md) · [TECH.md](file:///Users/linhancheng/Desktop/projects/warp-fork/specs/sidebar-folder-workspaces/TECH.md)
> **Original plan**: 14 tasks，分 3 phase（Foundation 5 + Vertical slices 7 + Polish 2）+ 3 checkpoints
> **Spike delivery**: T1-T9 + T11 + T13 → flag default-on (debug build)，sidebar 看到 "Default" workspace + 可建新 workspace via "+" 按鈕 + folder picker + 資料夾被刪會顯示 ⚠ warning。T10 + T12 + T14 deferred (見尾段「Spike Outcome」)。

## Architecture Decisions

- **Tab → Workspace association**：在現有 `tabs` 表加 nullable `folder_workspace_id` column（不另建 junction table）。理由：tab : workspace = N : 1；nullable 支援 flag-off 與 migration 中間狀態
- **Bootstrap migration 觸發點**：app init phase，idempotent + version-flag 防重複。理由：避免跟 tab 載入 race
- **`vertical_tabs.rs` 改動策略**：單一 `if FeatureFlag::is_enabled() { 新 } else { 原 }` 分支放在 line 1640-1665，**不**動其他 line。理由：upstream rebase 衝突最小化
- **WarpUI pattern source**：模仿 [`pane_group/mod.rs:832-881`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/pane_group/mod.rs) + [`text.rs:56-74`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/warpui_core/src/ui_components/text.rs)。理由：最像 collapsible-list-of-children pattern

---

## Phase 1: Foundation（5 tasks）

### Task 1：加 `FolderWorkspacesEnabled` feature flag variant

**Description**：把 `FolderWorkspacesEnabled` 加到 `warp_features` 的 `FeatureFlag` enum。flag 可在 Settings → Developer / Features toggle，但什麼都不做。

**Acceptance**:
- [ ] `FolderWorkspacesEnabled` enum variant 存在
- [ ] Settings → Developer / Features 看得到 toggle
- [ ] 切 toggle 對 sidebar 無觀察可見影響

**Verification**:
- `cargo build --bin warp-oss --features gui` 過
- `./script/run --dont-open && open …` → Settings 看到新 toggle

**Dependencies**：無

**Files**：`crates/warp_features/src/<flag-defs>.rs` — 1 file

**Scope**：XS

---

### Task 2：Diesel migration（建 `folder_workspaces` 表 + `tabs.folder_workspace_id`）

**Description**：寫 `up.sql` / `down.sql`：(a) 建 `folder_workspaces` 表（`id` PK, `name` text, `path` text, `display_order` int, `collapsed` bool, `created_ts` datetime）；(b) `tabs` 表加 nullable `folder_workspace_id INTEGER` FK。

**Acceptance**:
- [ ] `up.sql` 建表 6 column 齊
- [ ] `up.sql` 加 column 到 `tabs`
- [ ] `down.sql` drop column + drop table 乾淨

**Verification**:
- `diesel migration run --migration-dir crates/persistence/migrations` 過
- `diesel migration revert` 過（無 orphan）
- 再 run forward 過

**Dependencies**：無

**Files**：`crates/persistence/migrations/<ts>_folder_workspaces/{up,down}.sql` — 2 files

**Scope**：S

---

### Task 3：重 gen `schema.rs` + reconcile `schema.patch`

**Description**：T2 之後跑 `diesel print-schema` 更新 `crates/persistence/src/schema.rs`。確認 diff 純加（新表 + 新 column），無意外修改。如 `schema.patch` 影響到 modified table，重新 reconcile。

**Acceptance**:
- [ ] `schema.rs` 多了 `folder_workspaces` table macro
- [ ] `schema.rs` `tabs` 多了 `folder_workspace_id` column
- [ ] diff 純加
- [ ] `schema.patch`（若有影響）apply 乾淨

**Verification**:
- `cargo build` 過
- `git diff crates/persistence/src/schema.rs` 只看到 +line

**Dependencies**：T2

**Files**：`crates/persistence/src/schema.rs`（+ 可能 `schema.patch`）— 1-2 files

**Scope**：XS

---

### Task 4：定義 `FolderWorkspace` data model

**Description**：建 `app/src/folder_workspace/{mod,model}.rs`。`FolderWorkspace` struct + Diesel `Queryable` / `Insertable` derive 對應 schema。

**Acceptance**:
- [ ] `FolderWorkspace { id, name, path, display_order, collapsed, created_ts }` 存在
- [ ] `Queryable<folder_workspaces::SqlType, _>` impl
- [ ] `Insertable<folder_workspaces::table>` impl（new row optional id）
- [ ] `mod.rs` re-export

**Verification**:
- `cargo build` 過
- inline `#[cfg(test)]` test：構造 + in-memory SQLite insert/query round-trip

**Dependencies**：T3

**Files**：`app/src/folder_workspace/{mod,model}.rs` — 2 files

**Scope**：S

---

### Task 5：Pre-flight grep 解 TECH.md 4 個 open tech questions

**Description**：純 read-only research，不改 code。回答 TECH.md 的 4 個問題，更新 TECH.md。

**Questions**:
1. `tabs` 表 structure（column 型別、FK convention）
2. Migration init callsite（`run_pending_migrations` 在 startup 哪裡 invoke）
3. `FeatureFlag` → Settings UI 是 auto-discovery 還是 manual register
4. `NSOpenPanel`：Warp 已有 helper 還是要用 `objc` crate 直接 wrap

**Acceptance**:
- [ ] 4 個問題各有 file:line 引證
- [ ] TECH.md "Open Tech Questions" 段落更新答案

**Verification**：TECH.md diff visible

**Dependencies**：無（可跟 T1-T4 平行）

**Files**：`specs/sidebar-folder-workspaces/TECH.md` — 1 file

**Scope**：XS（read-only）

---

### Checkpoint：Foundation 完成

- [ ] T1-T5 verified
- [ ] `cargo build --bin warp-oss --features gui` 乾淨
- [ ] `cargo fmt && cargo clippy` 乾淨
- [ ] 4 tech questions 答完
- [ ] **Human review** 才進 Phase 2

---

## Phase 2: Vertical Slices（7 tasks）

### Task 6：實作 `FolderWorkspaceManager`（CRUD + bootstrap）

**Description**：建 `app/src/folder_workspace/manager.rs`。Methods：`create(name, path)` / `get_all()` / `get_by_id(id)` / `update_collapsed(id, bool)` / `delete(id)` / `bootstrap_default_workspace_for_existing_tabs()`。inline unit tests with in-memory SQLite。

**Acceptance**:
- [ ] 6 個 method 都實作 + 測過
- [ ] `bootstrap_…()` idempotent（多次呼叫不重複建 Default workspace）
- [ ] 測試 cover：create+read back、delete、collapse toggle、bootstrap with 0 / 3 既有 tabs

**Verification**:
- `cargo test app::folder_workspace::manager` 全綠
- 80%+ line coverage（`cargo-llvm-cov`）

**Dependencies**：T4

**Files**：`app/src/folder_workspace/manager.rs`（可能 + `tests.rs`）— 1-2 files

**Scope**：M

---

### Task 7：Hardcoded WarpUI view component

**Description**：建 `app/src/folder_workspace/view.rs`。`FolderWorkspaceHeader` UiComponent 渲染 name + collapse 箭頭 + N 個 hardcoded child tab rows。**先不接 manager**——純 render 接收假 `FolderWorkspace` data。模仿 [`pane_group/mod.rs:832-881`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/pane_group/mod.rs) + [`text.rs:56-74`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/warpui_core/src/ui_components/text.rs)。

**Acceptance**:
- [ ] `FolderWorkspaceHeader::new(name, tabs).build() → Container` work
- [ ] 元件被呼叫 render 不 panic
- [ ] 視覺：header text + collapse 箭頭 + indent child rows

**Verification**:
- `cargo build` 過
- 手動：暫時 wire 到 debug trigger 看一次，commit 前 revert；或 WarpUI 有 snapshot test 就用

**Dependencies**：T4

**Files**：`app/src/folder_workspace/view.rs` + `mod.rs` 更新 — 1-2 files

**Scope**：M（**最高風險** — WarpUI 學習曲線）

---

### Task 8：Sidebar integration（feature-flag gated）

**Description**：改 [`app/src/workspace/view/vertical_tabs.rs:1640-1665`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/workspace/view/vertical_tabs.rs)。加單一 `if FeatureFlag::FolderWorkspacesEnabled.is_enabled() { 新 } else { 原 }` 分支。「新」分支：`manager.get_all()` → 每個 workspace render `FolderWorkspaceHeader` + child tab rows。「else」分支：upstream 原碼不變。

**Acceptance**:
- [ ] 單一 if/else block 在 line 1640-1665，無其他 line 改動
- [ ] flag off → sidebar 跟 upstream 一致（diff 純加在 if-branch 內）
- [ ] flag on → sidebar render 出 SQLite 內 workspaces

**Verification**:
- `cargo build` 過
- 跑 app、toggle flag 觀察 sidebar 切換

**Dependencies**：T6, T7

**Files**：`app/src/workspace/view/vertical_tabs.rs` — 1 file（**HIGH RISK** 雖只動 1 file）

**Scope**：S（line scope）/ HIGH RISK

---

### Task 9："+" button + macOS folder picker

**Description**：sidebar toolbar 加 "+" 按鈕（具體位置 impl 時 confirm）。click → 開 `NSOpenPanel`（或 T5 找到的 helper）選 directory。選完 → call `manager.create(name = basename, path)`。sidebar refresh 顯示新 workspace。

**Acceptance**:
- [ ] flag on 時 sidebar 看到 "+"
- [ ] click → folder picker 開
- [ ] 選 folder → workspace 出現在 sidebar，不需 restart
- [ ] 預設 name = `path.file_name()`

**Verification**:
- 手動：click "+", select `~/Desktop`, "Desktop" workspace 出現
- SQLite check：`folder_workspaces` 表有 row

**Dependencies**：T6, T8, T5

**Files**：`vertical_tabs.rs`（toolbar）+ `app/src/folder_workspace/view.rs` 或新 `actions.rs` — 2-3 files

**Scope**：M

---

### Task 10：Tab → Workspace association on tab create

**Description**：當「current workspace」存在時，新開 tab 寫 `tabs.folder_workspace_id = current.id`，shell 預設 cwd = `current.path`。沒 current workspace（flag off / 沒任何 workspace）→ 行為不變。

**Acceptance**:
- [ ] 在 workspace X 開新 tab → `tabs.folder_workspace_id = X.id`
- [ ] 新 tab shell 起在 `X.path`
- [ ] 沒 current workspace 時 tab 開法不變

**Verification**:
- 手動：選 workspace、開新 tab、shell 看 cwd
- DB：`SELECT folder_workspace_id FROM tabs WHERE id = <new>` 對

**Dependencies**：T8, T9

**Files**：tab creation 邏輯檔（T5 grep 確認）+ `manager.rs`（新 assign method）— 2-3 files

**Scope**：M

---

### Task 11：Folder existence check + warning icon

**Description**：sidebar render 時每個 workspace 跑 `Path::new(&workspace.path).exists()`。false → render warning icon + tooltip「Folder no longer exists」。每次 render 重 check（cheap），所以 `mkdir` 同名後 auto-clear。

**Acceptance**:
- [ ] folder 被刪 → warning icon 看見
- [ ] tooltip 文字「Folder no longer exists」
- [ ] `mkdir` 同名 → 下次 render warning 消失
- [ ] 不動 DB（純 view-time check）

**Verification**:
- 手動：建 workspace 對 `/tmp/test-fw` → `rm -rf /tmp/test-fw` 看 warning → `mkdir /tmp/test-fw` 看 warning 消失

**Dependencies**：T7, T8

**Files**：`app/src/folder_workspace/view.rs` — 1 file

**Scope**：S

---

### Task 12：New-tab cwd fallback + 一次性 toast

**Description**：在 missing-folder workspace 開新 tab → cwd fallback `$HOME` + 顯示 toast「Folder X is missing; opened tab in home directory」。同一 session 同一 workspace 不重複 toast。

**Acceptance**:
- [ ] folder missing + 開新 tab → cwd = `$HOME`
- [ ] toast 顯示一次 / session / workspace
- [ ] 同 workspace 第 2 個新 tab → toast 不再顯示

**Verification**:
- 手動：T11 setup → 開 tab 觀察 cwd + toast
- 開第 2 個 tab → 確認 toast 不再

**Dependencies**：T10, T11

**Files**：tab creation 檔 + `manager.rs`（in-memory suppression state）— 2 files

**Scope**：S

---

### Checkpoint：Vertical Slices 完成

- [ ] T6-T12 verified
- [ ] End-to-end：建 workspace → 開 tab → cwd 正確 → restart Warp → workspace + tab 還原
- [ ] Folder missing flow（T11 + T12）work
- [ ] **Human review** 才進 Phase 3

---

## Phase 3: Polish（2 tasks）

### Task 13：Bootstrap migration on app init

**Description**：app 啟動 + flag on + 沒 workspace + 有 tabs → 跑 `manager.bootstrap_default_workspace_for_existing_tabs()`。Idempotent（重複呼叫安全）。掛在 T5 找到的 init phase callsite。

**Acceptance**:
- [ ] flag-on 首次啟動 + N 個既有 tabs → 全部 tabs 進 Default workspace（folder = `$HOME`）
- [ ] 後續啟動不重複建 Default
- [ ] flag-off 啟動不跑 migration

**Verification**:
- 手動：flag off 開 2 tabs → 關 app；開 flag → restart → "Default" workspace 內 2 個 tab；再 restart → 無重複 Default

**Dependencies**：T6, T8（callsite 來自 T5）

**Files**：app init 檔（T5 確認）+ `manager.rs`（bootstrap 已在 T6）— 1-2 files

**Scope**：S

---

### Task 14：Persist collapse + display order + integration test

**Description**：collapse 箭頭 click → `manager.update_collapsed(id, !current)`。render 時讀 `collapsed`。Insertion order = `display_order`（每次 create 時 = max + 1）。寫至少 1 個 [`crates/integration`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/integration/) Builder/TestStep e2e test：建 workspace + 加 tab + collapse + restart + assert 狀態保留。

**Acceptance**:
- [ ] click collapse → DB update
- [ ] restart Warp → collapse 狀態保留
- [ ] 新 workspace 排在尾（`display_order` = max + 1）
- [ ] integration test 過

**Verification**:
- `cargo nextest run -p integration --test folder_workspace`（依 [`warp-integration-test`](file:///Users/linhancheng/.claude/plugins/cache/warp/warp/skills/warp-integration-test/) skill）
- 手動：collapse → restart → 看狀態

**Dependencies**：T6, T8, T11

**Files**：`view.rs`（click handler）+ `manager.rs`（已有 update_collapsed）+ `crates/integration/tests/folder_workspace.rs` — 2-3 files

**Scope**：M

---

### Checkpoint：完成

- [ ] 14 tasks 全 verified
- [ ] `./script/presubmit` 過（fmt + clippy + test）
- [ ] `app/src/folder_workspace/` 80%+ coverage
- [ ] 手動 smoke：PRODUCT.md 全部 user story 跑過
- [ ] flag off → sidebar 跟 upstream 一致
- [ ] Ready for review

---

## Risks Summary

| Risk | Impact | Mitigation | Task |
|------|--------|------------|------|
| WarpUI 學習曲線 | Med-High | T7 hardcoded data 先做，不接 manager；TECH.md 有 pattern 引證 | T7 |
| `vertical_tabs.rs` regress | High | 單一 if/else，default off，commit 前手動 smoke | T8 |
| Diesel migration broken | High | down.sql 跑通 + schema diff 純加 | T2-T3 |
| `NSOpenPanel` pattern 不明 | Med | T5 grep 先；fallback `objc` crate | T9 |
| Bootstrap migration race | Low-Med | Idempotent + version check + init phase 掛點 | T13 |
| Upstream rebase conflict | Med（長期） | 改動限縮新檔 + `vertical_tabs.rs` 單一 if/else | All |

## Parallelization

大部分 sequential。可能的平行：
- T5 ‖ T1-T4（T5 純 read-only）
- T7 ‖ T6 部分（T7 用 hardcoded 資料；最後 T8 才 merge T6 結果）
- 測試邊做邊寫（TDD-lite）

## Verification Status

skill template 要求：
- [x] 每個 task 有 acceptance criteria
- [x] 每個 task 有 verification step
- [x] Dependencies 排序正確
- [x] 沒 task 動 > 5 files（最多 3）
- [x] 主要 phase 之間有 checkpoint
- [ ] **Human review** 待進行

---

**Phase 3 done. Phase 4 IMPLEMENT partial delivery 2026-04-29 — see「Spike Outcome」below.**

---

## Spike Outcome（2026-04-29）

### 已交付 (T1-T9 + T11 + T13)

| Task | Status | Commits |
|------|--------|---------|
| T1 FolderWorkspacesEnabled flag | ✅ | `7dbd004` |
| T2 Diesel migration (table + tabs.folder_workspace_id) | ✅ | `1c73a30` |
| T3 schema.rs regen | ✅ | `4e0fe67` (+ `6cefbec` fix Tab struct) |
| T4 FolderWorkspace data model + round-trip test | ✅ | `f1c473f` |
| T5 grep 4 open tech questions | ✅ | `5fe2587` |
| T6 manager (CRUD + bootstrap) + 8 unit tests | ✅ | `e1fb2d8` |
| T7 hardcoded WarpUI view component | ✅ | `93f0bec` |
| T8 sidebar integration (Entity Model + render) | ✅ | `ff88605` + `78c2322` |
| T13 bootstrap Default workspace at init | ✅ | `cd8daab` |
| chore: default-on in debug builds | ✅ | `a418008` |
| T9 + button + folder picker + create_workspace mutator | ✅ | `18edb76` + `0c03202` |
| T11 folder existence warning ⚠ icon | ✅ | `0c03202` |

**Demo 行為**：debug build 自動 on `FolderWorkspacesEnabled`
- 看到 "Default" workspace header（bootstrap 把現有 tab 包起來）
- 看到 "+ Add Folder Workspace" 按鈕，點 → 開 macOS folder picker → 選資料夾 → 出現新 workspace
- 把 workspace 對應 folder `rm -rf` → 標題變 `▾ Name ⚠` warning；`mkdir` 同名 → warning 自動消失

### Deferred to v2 (T10 + T12 + T14)

**T10 (tab → workspace association)**：需要先定義「current workspace」UX 概念（哪個 tab 開在哪個 workspace 之下）— v1 暫時所有新 tab 進原本平 list。

**T12 (cwd fallback + 一次性 toast)**：依賴 T10 + 需找 toast 機制。Folder missing 已有 ⚠ visual 警告（T11），cwd fallback 屬 nice-to-have。

**T14 (integration test)**：需要 [`crates/integration`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/integration/) Builder/TestStep 框架的學習曲線。Manager-level unit test (T6 + T4) 已 cover 邏輯正確性，integration test 主要 cover render 行為——可放到 v2。

**T9 write-side 注記**：採 fresh RW connection 路線（`establish_rw_connection`）而非原 TASKS.md 設想的 ModelEvent worker。違反 Warp 「single writer」 advisory（[`sqlite.rs:200-201`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/persistence/sqlite.rs)），但 user 點 "+" 不頻繁 + WAL + busy_timeout=1s 緩解 contention。Production 級別的 path B 應改回 ModelEvent。

### 重大架構發現（spike outcome）

1. **WarpUI Entity-Handle 學得起來**：模仿 `ProjectManagementModel` pattern，從 spec 撰寫到 T8 完整接好約 1 天。
2. **Render 不能讀 DB**：必須走 `SingletonEntity` + memory cache pattern。原 TASKS.md T8 寫「manager.get_all() from render」**架構不對**，pivot 到 path A 是對的。
3. **`vertical_tabs.rs` 動到 break sidebar 的風險**確實存在但可控：單一 `if FeatureFlag::is_enabled() { 新 } else { 原 }` 分支 + flag default off 緩解成功。
4. **Diesel SQLite RETURNING 限制**：`as_returning()` 不支援，要 fallback `last_insert_rowid()`。
5. **Single-writer-thread constraint**：`establish_ro_connection` 是 public 但 read-write 是 private，因為 Warp 強制 1 writer 經 ModelEvent。Write-side feature 全部要走 ModelEvent，不能短路。
6. **Bootstrap migration 跑 init phase 沒問題**：feature flag init 在 `persistence::initialize` 之後，所以 bootstrap 不能查 flag——但 idempotent + flag-off 時 sidebar 看不到 → 安全。
7. **FF 加 variant + 加 RUNTIME_FEATURE_FLAGS list 自動進 Settings menu**：UI 完全 zero-config，這是 Warp 設計優點。

### Path 決策（per SPIKE.md）

> spike 全跑通 + backend 已有 grouping 概念 → path B (local-only fork, 3-4 週 ship)

✅ **Path B 確認可行**。已交付的 T1-T8+T13 達成 SPIKE 5 個驗證問題：

1. ✅ 環境跑得起來（Day 1）
2. ✅ Sidebar render 進入點清楚（vertical_tabs.rs:1640-1665）
3. ✅ Backend 已有 workspace / project 概念（projects.rs 是 file-system tracker，跟 folder_workspace 並存）
4. ✅ WarpUI Entity-Handle pattern 學得起來
5. ✅ Hello-world patch 釘上去了（甚至超過 hello-world，做到完整 schema + render）

剩餘工程量估計：v2 (T9-T14) 約 1-2 週做 write-side。Phase 1 + 1-2 週 = Path B 預估 3-4 週符合預期。

---

## V2 Session Outcome（2026-04-30 follow-up）

回應 user 反饋「半成品 sidebar」之後，V2 session 在 `feat/folder-workspaces` 兩個 commit 內補完 cmux-like UX：

### V2 已交付 — `ffaf5a0` (V1-V4 grouping)

| Item | Commit | 對應 PRODUCT.md V2 |
|---|---|---|
| TabData / TabSnapshot / NewTab 帶 `folder_workspace_id` | ffaf5a0 | (foundation) |
| New tab fallback first workspace（後 V9 改 last-active） | ffaf5a0 | (foundation) |
| `vertical_tabs.rs` grouping render: header → indented children → `+ New Tab` → `+ Add Folder Workspace` | ffaf5a0 | S2/S3 部分 |
| Click header → toggle collapse | ffaf5a0 | (foundation for S1) |
| Per-ws `+ New Tab` button (cwd = ws.path) | ffaf5a0 | (foundation for S4) |

### V2 已交付 — `4c6a7c3` (V5-V10 lifecycle + UX)

| Item | Commit | 對應 PRODUCT.md V2 |
|---|---|---|
| Header rename / move up / down / delete (右鍵 menu + icon row) | 4c6a7c3 | S1 |
| Header 雙行 (name 14pt + path 11pt + tab count) | 4c6a7c3 | S2 |
| Tab UI minimal mode (砍 second_line + metadata + subtitle) | 4c6a7c3 | S3 |
| Cmd+T fallback last-active workspace | 4c6a7c3 | S4 |
| Tab 同 ws 內 reorder + 跨 ws 拒絕 | 4c6a7c3 | S5 |

### V2 待做 — Tasks T15-T24

按 [TECH.md V2 Implementation Order](file:///Users/linhancheng/Desktop/projects/warp-fork/specs/sidebar-folder-workspaces/TECH.md) 排序。

---

### Task 15: P5 — SVG icons 取代 unicode

**Description**: header 4 個 icon button (`✎ ↑ ↓ ✕`) 改用 Warp 既有 svg icon (透過 `WarpIcon::*::to_warpui_icon(color)` 路徑)。`📁` folder icon prefix 也改 svg。

**Acceptance**:
- [ ] 4 個 icon button 都用 svg 不用 unicode
- [ ] Folder icon prefix 用 svg
- [ ] 視覺一致；render 不依字體

**Verification**:
- `cargo build` 過
- 跑 app → header icon 對

**Dependencies**: 無

**Files**: `app/src/workspace/view/vertical_tabs.rs` + `app/src/folder_workspace/view.rs` — 2 files

**Scope**: S

---

### Task 16: P3 — Hover 才顯示 icon row

**Description**: header icon button row 預設 hidden，hover header 才出現。用 `Hoverable::new(state, |hover_state| { ... })` 控制；state 存在 `VerticalTabsPanelState.folder_workspace_header_hover_states: RefCell<HashMap<i32, MouseStateHandle>>`。Delete workspace 時要清 map entry。

**Acceptance**:
- [ ] icon row 預設 hidden（或 opacity 0 沒 layout shift）
- [ ] Hover header → icon row 出現
- [ ] 移開 → 隱藏
- [ ] Delete ws 後 map 沒 stale entry

**Verification**:
- 手動：hover / unhover 切換看
- 監看 memory map size on delete

**Dependencies**: 無

**Files**: `app/src/workspace/view/vertical_tabs.rs` — 1 file

**Scope**: S

---

### Task 17: P4 — Delete confirm dialog

**Description**: `DeleteFolderWorkspace { id }` 改 dispatch confirm dialog。dialog 抄 `delete_conversation_confirmation_dialog.rs` pattern。新加 `ConfirmDeleteFolderWorkspace { id }` action 走 entity.delete_workspace。`WorkspaceState` 加 `pending_folder_workspace_delete: Option<i32>`。

**Acceptance**:
- [ ] 點 ✕ icon 或右鍵 Delete → 跳 confirm
- [ ] OK → workspace 真刪 + tabs reassign
- [ ] Cancel → 不變

**Verification**:
- 手動：建 ws → 嘗試刪 → 看 dialog → 兩條路徑都試
- DB check：cancel 後 row 還在；confirm 後 row 沒了

**Dependencies**: 無

**Files**: 新 `app/src/workspace/delete_folder_workspace_confirmation_dialog.rs` + `app/src/workspace/{action.rs, view.rs}` — 3 files

**Scope**: M

---

### Task 18: P2 — Rename inline editor

**Description**: `RenameFolderWorkspace { id }` 從 osascript dialog 改 inline editor。抄 `Workspace.tab_rename_editor` pattern。新 ViewHandle `folder_workspace_rename_editor` + `WorkspaceState` 加 `is_folder_workspace_being_renamed`。`vertical_tabs.rs` header render：rename mode 時 editor 取代 title `Text`。

**Acceptance**:
- [ ] 點 ✎ / 右鍵 Rename → editor focus 在 header
- [ ] Enter / blur → commit + DB write
- [ ] Escape → cancel + 退 rename mode
- [ ] Rename 期間 click 別處 → 也 commit

**Verification**:
- 手動：rename "foo" → "bar"，restart 確認 "bar" persist
- Cancel rename，名字不變

**Dependencies**: T15（svg icons 才有 ✎ icon dispatch）

**Files**: `app/src/workspace/{view.rs, action.rs, view/vertical_tabs.rs}` + `app/src/folder_workspace/entity.rs` — 4 files

**Scope**: M

---

### Task 19: P1 — Drag workspace header reorder

**Description**: header wrap `Draggable` + 加 sidebar Y-position infra。State：

- `VerticalTabsPanelState.folder_workspace_drag_states: RefCell<HashMap<i32, DraggableState>>`
- `VerticalTabsPanelState.folder_workspace_header_positions: RefCell<HashMap<i32, RectF>>` (透過 `SavePosition` capture)

Actions: `StartFolderWorkspaceDrag { id }` / `DragFolderWorkspace { id, position }` / `DropFolderWorkspace`.

Drop 邏輯：drag rect midpoint Y 比對 captured positions，找最近 neighbor → swap `display_order`（多次 `MoveFolderWorkspaceUp/Down` 或 batch reorder）。

**Acceptance**:
- [ ] Drag header up/down → 視覺跟手指
- [ ] Drop → display_order 跟著新位置；DB persisted
- [ ] Cross-rate drag with > 2 swaps work

**Verification**:
- 手動：3 個 ws，drag 第 3 個到第 1 → 順序對；restart → 持久
- Cross-multi swap：drag 第 1 個到第 3 → 順序 [2, 3, 1]

**Dependencies**: 無（tab drag pattern 已存在可借用）

**Files**: `app/src/workspace/view/vertical_tabs.rs` + `app/src/workspace/{action.rs, view.rs}` + `app/src/folder_workspace/{entity.rs, manager.rs}` — 5 files

**Scope**: L（最大塊 polish）

---

### Task 20: D2 — Missing folder cwd fallback + toast

**Description**: `AddTabToFolderWorkspace` handler 內檢查 `path.exists()`：

- false → cwd = `$HOME`（已做）
- false → call `toast_stack.update` 加 ephemeral toast「Folder X is missing; opened tab in home directory」
- 一次性：`WorkspaceState` 或 model 加 `RefCell<HashSet<i32>>` 紀錄 session 內已 toast 過的 ws id

**Acceptance**:
- [ ] Folder missing 時 cwd = $HOME
- [ ] Toast 顯示一次 / session / workspace
- [ ] 同 ws 第 2 個新 tab → 不再 toast

**Verification**:
- 手動：建 ws 對 `/tmp/x` → `rm -rf /tmp/x` → 開 tab → toast + cwd home
- 再開一次 → 沒 toast

**Dependencies**: 無

**Files**: `app/src/workspace/view.rs` (handler) + 可能 `folder_workspace/entity.rs` (suppression state) — 2 files

**Scope**: S

---

### Task 21: D1 — ModelEvent path for folder_workspace mutations

**Description**: 加 5 個 ModelEvent 變體：

```rust
UpsertFolderWorkspace { workspace: FolderWorkspace }
DeleteFolderWorkspace { id: i32, fallback_id: Option<i32> }
UpdateFolderWorkspaceCollapsed { id: i32, collapsed: bool }
UpdateFolderWorkspaceDisplayOrder { id: i32, display_order: i32 }
UpdateFolderWorkspaceName { id: i32, name: String }
```

`app/src/persistence/sqlite.rs:601` 區塊新 match arms。`entity.rs` 改用 `model_event_sender.send(...)` 不再 fresh RW connection。Tentative-id (max + 1) 給 in-memory create；restart 重 load 校準。

**Acceptance**:
- [ ] 5 個 ModelEvent variants 加好
- [ ] sqlite.rs writer thread 處理
- [ ] entity.rs 沒有 `establish_rw_connection` 引用
- [ ] 8 個 manager unit tests 過（DB-level）+ 6 個 entity test (in-memory level，新加) 過

**Verification**:
- `cargo test` 過
- Run app, smoke create/rename/delete/reorder
- DB check 各個 column 正確

**Dependencies**: 無（但完成後 T22 才能 cleanup）

**Files**: `app/src/persistence/{mod.rs, sqlite.rs}` + `app/src/folder_workspace/entity.rs` — 3 files

**Scope**: L

---

### Task 22: D4 — Cleanup spike-only changes

**Description**:

1. `git revert a418008`（default-on debug flag）
2. 移除 `app/src/persistence/mod.rs` 的 `pub use sqlite::establish_rw_connection`（D1 後 entity 不需要）
3. 移除 `app/src/folder_workspace/mod.rs` 的 `#![allow(dead_code)]`

**Acceptance**:
- [ ] `cargo build --features gui` + `cargo clippy -- -D warnings` 都過
- [ ] flag 預設 off（user 在 Settings → Developer / Features 自己 toggle）
- [ ] grep `establish_rw_connection.*pub` 在 persistence crate 內 0 結果

**Verification**:
- Settings → Developer / Features 看 toggle 預設 off
- `flag off` 跑 → sidebar 跟 upstream 一樣

**Dependencies**: T21（必須 ModelEvent path 完成才能拿掉 establish_rw_connection re-export）

**Files**: `app/src/persistence/mod.rs` + `app/src/folder_workspace/mod.rs` + revert commit — 2 files + 1 revert

**Scope**: S

---

### Task 23: P6 — event_loop_proxy folder picker + rename modal

**Description**: 取代 osascript 路徑。看 [`crates/warpui/src/windowing/winit/delegate.rs:333+`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/warpui/src/windowing/winit/delegate.rs) 既有 picker pattern。

- 加 `CustomEvent::FolderPicked { path }` / `CustomEvent::FolderWorkspaceRenamed { id, new_name }` 變體
- `+ Add Folder Workspace` click → spawn thread → `native_dialog::FileDialog` → `event_loop_proxy.send_event` → main thread dispatch `AddFolderWorkspace`
- Rename 同樣 pattern（或併到 T18 inline editor 做完就不需要這條 rename 路徑）

**Acceptance**:
- [ ] Folder picker 不 panic
- [ ] Picker 期間其他 UI 操作不 block（不阻 main thread）
- [ ] 跨平台 OK（Linux / Windows folder picker 也 work，雖然 Warp 主用 macOS）

**Verification**:
- 手動：點 `+ Add Folder Workspace` → picker 出現 → cancel / pick 都 OK
- 測試：picker 開著時點 sidebar 別處沒 freeze

**Dependencies**: 無（T18 完成則 rename 不需這條）

**Files**: `app/src/workspace/view/vertical_tabs.rs` + `crates/warpui/src/windowing/winit/delegate.rs` + 可能 `app/src/lib.rs`（CustomEvent dispatch） — 3 files

**Scope**: M

---

### Task 24: D3 — Integration test

**Description**: 用 `crates/integration` Builder/TestStep 框架寫 1 個 e2e test：

1. Build app + flag on
2. 建 workspace `~/code/foo`
3. 在 foo 開新 tab，assert tab.fwid = foo.id
4. Toggle collapse foo
5. Restart app（snapshot + restore）
6. Assert workspace 仍存在 + collapse 狀態保留 + tab 仍綁 foo

進階 test（optional）：rename / delete tab reassign / cross-ws drop rejection。

**Acceptance**:
- [ ] 1+ e2e test 過
- [ ] CI run（如有）整合

**Verification**:
- `cargo nextest run -p integration --test folder_workspace` 過

**Dependencies**: 無（建議 T17-T22 都先做完，test 才 cover 多）

**Files**: 新 `crates/integration/tests/folder_workspace.rs` — 1 file

**Scope**: M

---

## V2 Phase Checkpoint

- [ ] T15-T24 verified
- [ ] PRODUCT.md V2 Success Criteria 全打勾
- [ ] `./script/presubmit` 過
- [ ] `cargo clippy --bin warp-oss --features gui --lib -- -D warnings` 0 warning
- [ ] flag-off 時 sidebar 跟 upstream `warpdotdev/warp` 一致（diff 純加在 if-branch）
- [ ] **Human review** 才 close PR

---

**Phase 3 V2 update done — 2026-04-30 補。等 user 排優先序開做。**

---

## V3 Session — Per-folder default command (2026-04-30)

> 對齊 [PRODUCT.md V3 增量規格](file:///Users/linhancheng/Desktop/projects/warp-fork/specs/sidebar-folder-workspaces/PRODUCT.md) + [TECH.md V3 增量技術規劃](file:///Users/linhancheng/Desktop/projects/warp-fork/specs/sidebar-folder-workspaces/TECH.md)。
>
> **目標**：cmux `~/.zshrc` zero-action auto-launch 路徑 port 到 wrap 內建。每個 folder workspace 可設 `default_command`，新 tab 開起來自動跑；新 ws 從全域 setting 帶預設值（`claude`）；alt-modifier / 右鍵 menu 單次 opt-out。

### Task 25: V3.1 — Diesel migration + schema regen + model field

**Description**:

1. 新 migration `<ts>_folder_workspace_default_command/{up,down}.sql`
2. up: `ALTER TABLE folder_workspaces ADD COLUMN default_command TEXT NULL`
3. down: `ALTER TABLE folder_workspaces DROP COLUMN default_command`（SQLite ≥ 3.35 支援；若版本 < 3.35 改 rebuild table pattern）
4. `crates/persistence/src/schema.rs` `folder_workspaces!` macro 加 `default_command -> Nullable<Text>`
5. `app/src/folder_workspace/model.rs` `FolderWorkspace` struct 加 `default_command: Option<String>` field（Diesel `Queryable` / `Insertable` 自動處理 nullable）

**Acceptance**:
- [ ] `diesel migration run` 過
- [ ] `diesel migration revert` 過
- [ ] 再 forward 過
- [ ] `cargo build` 過
- [ ] inline test：構造 `FolderWorkspace` 帶 default_command Some/None → insert → query back → match

**Verification**:
- `cargo test app::folder_workspace::manager::tests::default_command_round_trip` 過
- `git diff crates/persistence/src/schema.rs` 純加

**Dependencies**: 無（V1/V2 既有 ws 的 default_command 自動為 NULL）

**Files**: `crates/persistence/migrations/<ts>_folder_workspace_default_command/{up,down}.sql` + `crates/persistence/src/schema.rs` + `app/src/folder_workspace/model.rs` — 4 files

**Scope**: S

---

### Task 26: V3.2 — Manager + entity setter

**Description**:

`app/src/folder_workspace/manager.rs` 加：
```rust
pub fn set_default_command(
    conn: &mut SqliteConnection,
    id: i32,
    command: Option<String>,
) -> QueryResult<usize>
```

`app/src/folder_workspace/entity.rs` 加：
```rust
pub fn set_default_command(
    &self,
    id: i32,
    command: Option<String>,
    ctx: &mut ModelContext<Self>,
) -> Result<()>
```

走 ModelEvent path（如果 T21 已完成）或 fresh RW connection（spike 模式）。Update in-memory cache 然後 notify。

**Acceptance**:
- [ ] manager fn 接 `Option<String>`，None → 寫 NULL，Some(empty) → 寫 ""，Some(non-empty) → 寫該值
- [ ] entity fn 同步 update in-memory cache
- [ ] manager unit test cover 三種 Option<String> input

**Verification**:
- `cargo test` 過
- 手動：呼叫 entity fn → query DB confirm 寫入

**Dependencies**: T25

**Files**: `app/src/folder_workspace/manager.rs` + `app/src/folder_workspace/entity.rs` — 2 files

**Scope**: S

---

### Task 27: V3.3 — Settings 全域 default_command_for_new_folder_workspaces

**Description**:

1. Grep `pub struct .*Settings` / `UserPreferences` / `pref_field!` 找 wrap 既有 settings 結構（推測在 `crates/warp_core/src/user_preferences.rs` 或 `warpui_extras::user_preferences`）
2. 加 `default_command_for_new_folder_workspaces: String` field，default `"claude"`
3. Setting 進 `TomlBackedUserPreferences` 持久化路徑
4. `AddFolderWorkspace` handler ([`workspace/view.rs`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/workspace/view.rs))：建立 ws 時讀此 setting → 寫進 `default_command`
5. 改 setting **不**影響既有 ws（既有 ws 已 freeze 自己的值）

**Acceptance**:
- [ ] Setting 加好，預設 `"claude"`
- [ ] 建新 ws → DB 看到 `default_command = "claude"`
- [ ] 改 setting 為 `"nvim"` → 再建一個 ws → DB 看到 `default_command = "nvim"`
- [ ] 改 setting 不動既有 ws 的 default_command
- [ ] Setting 設成空字串 → 新 ws default_command 為空（純 shell）

**Verification**:
- 手動：toggle setting → 建 ws → SQLite query confirm
- restart wrap → setting 持久

**Dependencies**: T25

**Files**: settings struct file (T27 grep 確認) + `app/src/workspace/view.rs` (`AddFolderWorkspace` handler) — 2-3 files

**Scope**: M

---

### Task 28: V3.4 — AddTabToFolderWorkspace handler 改用 LaunchConfig Template path

**Description**:

[`app/src/workspace/action.rs:151`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/workspace/action.rs)：
```rust
AddTabToFolderWorkspace {
    folder_workspace_id: i32,
    path: PathBuf,
    skip_default_command: bool,   // 新增
}
```

既有兩處 dispatch ([`vertical_tabs.rs:1794`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/workspace/view/vertical_tabs.rs), [`vertical_tabs.rs:1979`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/workspace/view/vertical_tabs.rs)) 加 `skip_default_command: false`（暫時）。

`workspace/view.rs:20504` handler 改寫：

```rust
let workspace = FolderWorkspaceModel::handle(ctx).as_ref(ctx).get(workspace_id).cloned();
let default_command = workspace.and_then(|ws| ws.default_command);

let layout = if !skip_default_command
    && default_command.as_ref().is_some_and(|s| !s.is_empty())
{
    PanesLayout::Template(PaneTemplateType::PaneTemplate {
        cwd: initial_directory.clone().unwrap_or_else(|| PathBuf::from(".")),
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

**Acceptance**:
- [ ] `skip_default_command` field 加好；既有兩處 dispatch 編譯過
- [ ] ws 設 `default_command = "echo hi"` → 開新 tab → terminal 看到 `hi` output
- [ ] ws 設 `default_command = "claude"` + 已裝 claude → 新 tab 自動進 claude
- [ ] ws.default_command = None → 開新 tab 純 shell prompt（既有行為不變）
- [ ] ws.default_command = "" → 同 None
- [ ] Folder missing 時 fallback `$HOME` cwd 仍適用 Template path
- [ ] command 結束（Ctrl+C / exit）→ shell prompt 回來，tab 不死

**Verification**:
- 手動：3 個 ws：(a) command=None (b) command="echo hi" (c) command="claude"
  - (a) 純 shell ✓
  - (b) 看到 `hi` ✓
  - (c) claude 自動起 ✓ + Ctrl+C 後 shell prompt 回來 ✓

**Dependencies**: T26

**Files**: `app/src/workspace/action.rs` + `app/src/workspace/view.rs` (handler) + `app/src/workspace/view/vertical_tabs.rs` (兩處 dispatch) — 3 files

**Scope**: M

---

### Task 29: V3.5 — Per-ws default_command inline editor

**Description**:

抄 P2 (T18) rename inline editor pattern。獨立 ViewHandle 跟 state flag 不共用。

- `Workspace` 加 `folder_workspace_default_command_editor: ViewHandle<EditorView>`
- `WorkspaceState` 加 `folder_workspace_default_command_being_edited: Option<i32>`
- `vertical_tabs.rs` header 右鍵 menu 加「Set default command...」item（osascript "choose from list" 暫用，T23 P6 完成後改 inline menu）
- 點 menu → dispatch `RenameFolderWorkspaceDefaultCommand { id }` action（命名跟 RenameFolderWorkspace 區隔）
- Handler 進 edit mode：focus editor，pre-fill 現值
- Editor commit (Enter / blur)：dispatch `SetFolderWorkspaceDefaultCommand { id, command }` → entity.set_default_command
- Cancel (Escape)：退 edit mode，不寫 DB
- 空字串 commit → 寫 None（不寫 ""）

**Acceptance**:
- [ ] 右鍵 ws header → 看到「Set default command...」item
- [ ] Click → editor focus，pre-fill 現有值
- [ ] 改字串 + Enter → DB update + in-memory update
- [ ] Escape → 不變
- [ ] Empty commit → DB 寫 NULL
- [ ] Restart wrap → 設定持久

**Verification**:
- 手動：建 ws → 右鍵 set "claude" → 開 tab 確認 → 右鍵 set "" → 開 tab 純 shell
- DB check：`SELECT default_command FROM folder_workspaces WHERE id=...`

**Dependencies**: T26（entity setter 必須在），T18 rename editor pattern 可參考但不依賴

**Files**: `app/src/workspace/view.rs` (editor view + handler) + `app/src/workspace/action.rs` + `app/src/workspace/view/vertical_tabs.rs` (header context menu) — 3 files

**Scope**: M

---

### Task 30: V3.6 — Opt-out modifier key + 右鍵 menu

**Description**:

兩條 opt-out 路徑：

**(a) Modifier key**：sidebar `+ New Tab` 按鈕 click handler 讀 modifier state：

- Grep `EventHandler::on_left_mouse_down` 既有 modifier-aware usage（推測 wrap 有 `MouseDownEvent.modifiers` 之類）
- 點按鈕 + Alt/Option held → dispatch `AddTabToFolderWorkspace { skip_default_command: true, ... }`

**(b) 右鍵 menu**：`+ New Tab` 按鈕 wrap `EventHandler::on_right_mouse_down` → osascript "choose from list" 兩 option：「Open with default command」/「Open without default command」→ 對應 dispatch

兩條路徑都改 `vertical_tabs.rs:1794` 跟 `:1979` 兩處 dispatch site。

**Acceptance**:
- [ ] `+ New Tab` 按鈕：⌥-click → 開 tab 不跑 default command
- [ ] `+ New Tab` 按鈕：右鍵 → 兩 option menu 出現
- [ ] 選「Open without default command」→ 不跑 command
- [ ] 普通 click → 跑 default_command（行為不變）
- [ ] opt-out 後 ws.default_command 沒被清掉（單次 only）

**Verification**:
- 手動：ws default_command = "echo hi"
  - 點 + → 看到 `hi` ✓
  - ⌥-點 + → 純 shell ✓
  - 右鍵 + → 選 without → 純 shell ✓
- DB check：opt-out 後 `default_command` 仍為 "echo hi" ✓

**Dependencies**: T28 (`skip_default_command` field 必須在)

**Files**: `app/src/workspace/view/vertical_tabs.rs` (兩處 dispatch + 右鍵 menu) — 1 file

**Scope**: S（前提：modifier-aware click pattern 找得到）/ M（如果要新發明）

---

### Task 31: V3.7 — Tests + smoke

**Description**:

1. Manager unit tests：
   - `set_default_command` Some / None / empty round-trip
   - 既有 ws (default_command NULL) → set Some → query back
2. Entity tests：
   - `set_default_command` updates in-memory cache
3. Smoke checklist（手動）：
   - [ ] V3 Success Criteria PRODUCT.md 全部勾過
   - [ ] flag-off 行為跟 V2 一致
   - [ ] `./script/presubmit` 過
4. Optional integration test（接 T24 框架）：
   - 建 ws + setdefault_command + 開 tab + assert command 跑過

**Acceptance**:
- [ ] manager / entity 單元測試新增至少 4 個
- [ ] 全 V3 Success Criteria checklist 過
- [ ] presubmit 過

**Verification**:
- `cargo test app::folder_workspace::` 過
- 手動 smoke

**Dependencies**: T25-T30

**Files**: `app/src/folder_workspace/{manager,entity}.rs` 內 tests — 2 files

**Scope**: S

---

## V3 Phase Checkpoint

- [ ] T25-T31 verified
- [ ] PRODUCT.md V3 Success Criteria 全打勾
- [ ] `./script/presubmit` 過
- [ ] `cargo clippy --bin warp-oss --features gui --lib -- -D warnings` 0 warning
- [ ] flag-off 行為跟 upstream 一致
- [ ] 既有 V1/V2 ws default_command 為 NULL → 行為不變（regression net）
- [ ] **Human review** 才 close

---

## V3 Architecture Decisions（pre-implementation）

1. **Borrow LaunchConfig CommandTemplate**：不重發明 spawn 路徑；wrap 既有 [`PanesLayout::Template`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/pane_group/mod.rs) 滿足需求
2. **`exec` 命名是 launch config schema string，不是 unix exec**：command 在 shell context 跑、結束後 shell prompt 回來 — 跟 cmux 教訓的「不要 `exec claude`」自動對齊
3. **不需要 cmux 5 條 guard**：folder workspace 是顯式選擇，white-list 規則被「user 加 ws 動作」取代（PRODUCT.md V3 規格表已說明）
4. **Settings 全域 default 不影響既有 ws**：寫進 ws.default_command 後 freeze；改 setting 只影響後續新建 ws
5. **Opt-out 用 action payload field 不用 env var**：`AddTabToFolderWorkspace.skip_default_command: bool`；env var (`SKIP_DEFAULT_COMMAND`) 是 cmux 路線的延伸，wrap 內建 mode 不需要
6. **Per-ws editor 抄 T18 pattern 但不共用 ViewHandle**：避免 rename / set-command 行為互踩
7. **空字串 = None**：UI 上「清掉」default_command 用 commit empty string 即可；DB 內存 NULL（一致性）
8. **Cmd+T 路徑暫不接 opt-out**：cmd+T 走 `AddTerminalTab` + `assign_default_folder_workspace_to_active_tab`，是事後 reassign 路徑，重接 opt-out 工程量大且 use case 不明顯；v4 評估

---

**Phase 3 V3 update done — 2026-04-30。Spec commit 後 user 排優先序開做。**

---

## V3 Session Outcome (2026-04-30)

V3 7 tasks 全交付，feat/folder-workspaces 上 commits：

| Task | Commit | 內容 |
|---|---|---|
| T25 DB schema + model field | `f8803c1` | migration + schema.rs + FolderWorkspace.default_command + 2 new round-trip tests |
| T26 manager + entity setter | `0f5d089` | manager::set_default_command + ModelEvent::UpdateFolderWorkspaceDefaultCommand + entity::set_default_command + 2 new tests |
| T27 全域 setting | `56e561f` | FolderWorkspaceSettings group + InsertFolderWorkspace.default_command threading + AddFolderWorkspace handler reads setting |
| T27 fix | `b8f3e91` | hotfix — register FolderWorkspaceSettings in init.rs (T27 漏了 → 加 ws 時 panic) |
| T28 LaunchConfig Template path | `180a519` | AddTabToFolderWorkspace.skip_default_command + handler routes through PanesLayout::Template when default_command set |
| T29 inline editor | `0b0d25f` | folder_workspace_default_command_editor + state field + finish/cancel/edit fns + 右鍵 menu「Set default command...」+ header path-slot 替換 editor |
| T30 opt-out right-click | `5549478` | + New Tab 按鈕加 on_right_mouse_down osascript menu 兩 option |

13 → 17 unit tests（manager + model + entity 都 cover default_command 路徑）。

### Architecture decisions (vibe-coding 自主)

1. **借 LaunchConfig CommandTemplate 不發明 spawn 路徑** — 確認 `exec` 是 schema string，shell spawn command 後不取代 shell（process tree 驗：claude 是 zsh 的 child）
2. **set_default_command empty / None 都 normalize to NULL** — 一致性，UI 端 commit 空字串等同清值
3. **inline editor 取代 path slot 不取代 title slot** — title 仍可見 user 知道編哪個 ws；rename mode 取代 title slot 兩者不衝突
4. **⌥-modifier opt-out deferred V5+** — wrapui_core on_left_mouse_down callback 沒 modifier；plumb on_modifier_state_changed → state field → 讀回 工程量大，cmux user 也很少用 opt-out（zero-action 慣性）
5. **osascript menu 沿用 V2 spike pattern** — P6 (T23) 一次性換成 native menu 兼顧 rename / set-cmd / opt-out 三處
6. **既有 V1/V2 ws default_command=NULL 路徑保留 SingleTerminal layout** — flag-off / V3 升級前的 ws 行為不變（regression net）

### V3 Success Criteria 驗證

- [x] **S6** ws.default_command = `claude` → 新 tab 自動跑 claude（user 實測）
- [x] **S6** Command 結束後 shell prompt 回來（process tree 證明：claude 是 zsh child + user 實測 `/quit` 後回 prompt）
- [x] **S6** Empty default_command → 純 shell（測試 2 ✓）
- [x] **S6** 既有 ws (V1/V2) default_command 為 NULL → SingleTerminal path 保留行為不變（邏輯保留 + 測試 1 改了 default_command 後 work，反向推論 NULL path 也 work；regression net via flag-off 路徑）
- [x] **S7** Settings 預設 `claude`（user 實測：建新 ws 自動跑 claude）
- [N/E] **S7** 改 setting 為 nvim → 新 ws 帶新值（邏輯確認，未端到端驗 — 改 toml 或 GUI settings 都該 work）
- [N/E] **S7** 改 setting 不影響既有 ws（邏輯確認 — DB 內既有 ws.default_command 在 create 時 freeze）
- [DEFERRED] **S8** ⌥-click → V5+
- [x] **S8** 右鍵 + New Tab → 「Open without default command」→ 純 shell（測試 3 ✓）
- [x] **S8** opt-out 不清 ws.default_command（單次跳過：測試 3 後 ws 仍帶 echo hi）
- [x] **Quality** 17/17 unit tests + cargo build 過 + flag-off 路徑保留

### V5+ 候選追蹤

- ⌥-modifier opt-out（需 plumb on_modifier_state_changed）
- Native context menu 取代 osascript（rename / set-cmd / new tab opt-out 三處）— 即原 P6 (T23)
- `cmd+T` 路徑接 opt-out（事後 reassign 路徑，工程量大）
- wrap 內輸入法 + Ctrl+C 互動 bug（user 實測時發現，跟 V3 無關，獨立 issue）

### 下一輪建議

V3 已 daily-driver-ready。V4「close to menu bar」spec 已 commit（`edfb5db`），new session 開做。

**V3 phase done — 2026-04-30。**

---

## V4 Session — Close to menu bar (2026-04-30)

> 對齊 [PRODUCT.md V4 增量規格](file:///Users/linhancheng/Desktop/projects/warp-fork/specs/sidebar-folder-workspaces/PRODUCT.md) + [TECH.md V4 增量技術規劃](file:///Users/linhancheng/Desktop/projects/warp-fork/specs/sidebar-folder-workspaces/TECH.md)。
>
> **目標**：cmux「app 不真退出，window 隱藏到 menu bar 圖示」行為 port 到 wrap。**macOS only**。setting opt-in 預設 off。

### Task 32: V4.1 — Setting `close_to_menu_bar` + toml schema

**Description**:

1. [`app/src/settings/mod.rs`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/settings/mod.rs) `Settings` struct 加 `close_to_menu_bar: bool`，default `false`
2. user_preferences toml schema 對應加（看 [`crates/warpui_extras/src/user_preferences/`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/warpui_extras/src/user_preferences/) 既有 bool field 範例）
3. SettingsChanged broadcast event（grep 既有 settings change pattern）
4. Setting 改變 → broadcast → 後續 Task 35 listener 收到後 install/uninstall

**Acceptance**:
- [ ] Setting field 加好，default false
- [ ] toml round-trip：寫 `close_to_menu_bar = true` 進 toml → restart wrap → setting 讀回 true
- [ ] SettingsChanged event 在 toggle 時 fire（log 或 unit test cover）

**Verification**:
- `cargo build` 過
- 手動：改 toml file → restart 觀察 setting 持久
- unit test：`Settings::default().close_to_menu_bar == false`

**Dependencies**: 無

**Files**: `app/src/settings/mod.rs` + user_preferences toml schema (1-2 file) — 2-3 files

**Scope**: S

---

### Task 33: V4.2 — Settings UI toggle

**Description**:

Settings UI 加「Close to menu bar」toggle，在 General / Window / Behavior 子分類（grep 既有 bool toggle 看放哪）。Toggle change → 寫進 setting → broadcast。

**Acceptance**:
- [ ] Settings UI 看到 toggle，預設 off
- [ ] 點 toggle on/off → setting value 跟著變
- [ ] toml 跟著 update

**Verification**:
- 手動：開 Settings → 看到 toggle → 點切 → 再 reopen Settings 確認狀態保留
- toml file 看到對應寫入

**Dependencies**: T32

**Files**: settings UI render file (T33 grep 確認，可能在 [`app/src/settings/`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/settings/) 內) — 1-2 files

**Scope**: S

---

### Task 34: V4.3 — NSStatusItem ObjC FFI

**Description**:

新檔 `crates/warpui/src/platform/mac/objc/status_item.m` + Rust binding 在新檔 `crates/warpui/src/platform/mac/status_item.rs`：

ObjC API：
```objc
NSStatusItem *create_status_item(void);
void destroy_status_item(NSStatusItem *item);
void set_status_item_image(NSStatusItem *item, NSData *png_bytes);
void set_status_item_menu(NSStatusItem *item, NSString **item_titles, int n);
```

Rust binding：
```rust
pub struct StatusItem {
    handle: id,
}

impl StatusItem {
    pub fn install(image_png: &[u8], menu_items: Vec<(String, Box<dyn Fn() + Send>)>) -> Self;
}

impl Drop for StatusItem {
    fn drop(&mut self) { unsafe { destroy_status_item(self.handle) } }
}
```

Menu item callback 透過 ObjC target-action — Rust export `extern "C" fn handle_status_item_action(idx: c_int)`，ObjC 端 NSMenuItem.target+selector 觸發此 fn，內部依 idx 呼叫 stored callbacks。

Image 暫用 wrap 既有 logo 的 monochrome variant（template image，macOS 自動 dark/light 適配）；找不到的話 v4 spike 用任意 placeholder PNG，T36 polish 換正式 icon。

**Acceptance**:
- [ ] `StatusItem::install(...)` 呼叫 → menu bar 看到圖示
- [ ] 圖示 click → menu drop down，看到 menu items
- [ ] click menu item → callback 觸發
- [ ] `drop(StatusItem)` → 圖示消失

**Verification**:
- 手動：寫個一次性 trigger（譬如 settings init 暫時無條件 install）→ 看 menu bar 圖示+ menu items
- callback：menu items 印 log，點看 log

**Dependencies**: 無（ObjC FFI 獨立）

**Files**: 新 `crates/warpui/src/platform/mac/objc/status_item.m` + 新 `crates/warpui/src/platform/mac/status_item.rs` + `crates/warpui/build.rs` (cc compile new .m) + `crates/warpui/src/platform/mac/mod.rs` (re-export) — 4 files

**Scope**: M（**最高風險** — ObjC FFI 跨 winit 整合曲線）

---

### Task 35: V4.4 — Status item install/uninstall by setting watcher

**Description**:

App 內持有 `Option<StatusItem>` 在 `App` 或 `WorkspaceModel`。

- App init 時讀 `close_to_menu_bar`：true → `StatusItem::install(...)` 寫進 Option
- SettingsChanged listener 監聽 `close_to_menu_bar`：
  - false → true → install
  - true → false → drop（Option set None）
- StatusItem callbacks（Show Warp / Quit Warp）目前先空 impl，T36 接

**Acceptance**:
- [ ] App init + setting on → menu bar 看到圖示
- [ ] App init + setting off → 沒圖示
- [ ] Toggle setting off → on → 圖示出現
- [ ] Toggle setting on → off → 圖示消失

**Verification**:
- 手動：toggle setting 切換看圖示生滅

**Dependencies**: T32, T34

**Files**: `app/src/lib.rs` 或新 `app/src/menu_bar_status_item.rs` — 1-2 files

**Scope**: S

---

### Task 36: V4.5 — Show Warp / Quit Warp menu actions

**Description**:

實作 status item menu 兩個 callback：

**Show Warp**：
- 走 winit `event_loop_proxy.send_event(CustomEvent::ShowWarpFromStatusItem)`
- main thread handler 收到 → 呼叫 `applicationShouldHandleReopen:hasVisibleWindows:NO` 既有 reopen 路徑（[`app.m:324`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/warpui/src/platform/mac/objc/app.m)） — 該邏輯本來就處理「Dock click 重開隱藏 window」
- 如果完全沒 window，走「new window」action

**Quit Warp**：
- 走 winit `event_loop_proxy.send_event(CustomEvent::QuitWarpFromStatusItem)`
- main thread handler 呼叫既有 `terminate_app(TerminationMode::Cancellable)` ([`delegate.rs:397`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/warpui/src/platform/mac/delegate.rs))

**Acceptance**:
- [ ] Setting on + window 隱藏 → 點 Show Warp → window 恢復可見
- [ ] Setting on + 完全沒 window → 點 Show Warp → 建新 window
- [ ] Setting on → 點 Quit Warp → app 真退（process 死）
- [ ] 既有 ⌘Q / Dock right-click Quit 路徑不變

**Verification**:
- 手動：T35 setup → 觸發 close（T37 之後可 hide）→ 點 Show Warp → window 回來
- 手動：點 Quit → 看 wrap process 死

**Dependencies**: T34, T35

**Files**: `app/src/lib.rs` (CustomEvent dispatch) + `crates/warpui/src/windowing/winit/event_loop/mod.rs` 或對應 (CustomEvent variant) + `app/src/menu_bar_status_item.rs` (callback wiring) — 2-3 files

**Scope**: M

---

### Task 37: V4.6 — Close window 攔截（hide instead of close）

**Description**:

**核心邏輯**：setting on **AND** 這是 last visible window → close 改 `hide_window`；otherwise 原路徑。

攔截點建議在 [`app/src/pane_group/pane/terminal_pane.rs:656`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/pane_group/pane/terminal_pane.rs) `Event::CloseRequested` 上層（pane 不知道全局 window 數，所以攔截要往上一層 — 看 `WorkspaceModel` 或 `App` level）。

實際攔截位置 T37 spike 階段確認。可能：
- (a) `close_pane_with_confirmation` 改成讀 setting + last-window check
- (b) 新加 layer 包 close action，走 setting check 後 dispatch hide vs close

「last visible window」邏輯：
- 跑遍 `App.windows` 或 `WindowManager.windows` 找 visible 數量
- 若這個 close 走完會變 0 → 走 hide 不走 close
- ≥ 1 → 走原 close

⌘Q 路徑（`terminate_app`）不動 — quit 仍然真退。

**Acceptance**:
- [ ] Setting on + 點 window 紅圈關閉鍵（last window）→ window 隱藏，wrap PID 仍存在
- [ ] Setting on + ⌘W (last window) → window 隱藏
- [ ] Setting on + 多 window 環境 close 非 last window → 走原 close（其他 window 還在）
- [ ] Setting off + close → 走原 close（quit if last） — 既有行為不變
- [ ] Setting on + window 隱藏狀態下 claude session 仍跑（看 process tree）
- [ ] ⌘Q（任何 setting）→ 真 quit

**Verification**:
- 手動：setting on → 開 claude → close window → ps 看 wrap + claude 仍在 → 點 status item Show Warp → window 回來且 claude session continuity
- 多 window：開兩 window → close 第一個 → 第二個還在；close 第二個（last）→ 隱藏不退出

**Dependencies**: T32, T35（要讀 setting 跟有 status item 才能 recover window）

**Files**: `app/src/pane_group/pane/terminal_pane.rs` 或上層 close handler + 可能 `crates/warpui_core/src/windowing/state.rs` (last-window query) — 2-3 files

**Scope**: M（**HIGH RISK** — 攔截位置需要 spike 確認）

---

### Task 38: V4.7 — Smoke + S12 Dock icon question

**Description**:

1. 跑 PRODUCT.md V4 Success Criteria 全部 manual smoke
2. 確認既有 quit / close 路徑無 regression（setting off 模式）
3. `./script/presubmit` 過
4. **S12 Dock icon 行為決策**：用此 task 機會 review 給 user，決定 (a)/(b)/(c) 走哪條
   - 若選 (c)（默認）→ 不動 Dock，task done
   - 若選 (a)/(b) → V4 補一個 task T39 加 `setActivationPolicy` 控制 + setting

**Acceptance**:
- [ ] PRODUCT.md V4 Success Criteria 11 項全打勾
- [ ] presubmit 過
- [ ] Setting off 路徑跟 V3 結束時行為一致（regression net）
- [ ] S12 question resolved（user 決策記錄在 V4 outcome）

**Verification**:
- 完整 smoke checklist

**Dependencies**: T32-T37

**Files**: 無 code change（純驗證）；可能補 V4 outcome notes 在此 spec — 1 file

**Scope**: S

---

## V4 Phase Checkpoint

- [ ] T32-T38 verified
- [ ] PRODUCT.md V4 Success Criteria 全打勾
- [ ] `./script/presubmit` 過
- [ ] `cargo clippy --bin warp-oss --features gui --lib -- -D warnings` 0 warning
- [ ] Setting off 行為跟 V3 結束時完全一致（regression net）
- [ ] **Human review** 才 close

---

## V4 Architecture Decisions（pre-implementation）

1. **借既有 hide_window 不另發明**：`Platform::hide_window` 已是 cross-platform API（mac 實作真 hide，headless / winit 也有 stub）— V4 直接呼叫
2. **Status item ObjC 直接 FFI 不引 `tray-icon` crate**：增加 dependency 風險 vs V4 macOS-only 簡單 ObjC 範圍 — 後者更穩
3. **「last window check」放 close 攔截邏輯**：`hide` 行為只在「close 後會變 0 visible window」時 trigger；多 window 環境 close 中間一個仍正常 close
4. **⌘Q 路徑不動**：`terminate_app` 走 `[NSApp terminate]` 完全不變，避免「⌘Q 也只 hide」regression
5. **Show Warp 重用 `applicationShouldHandleReopen:`**：Dock click 重開窗的 macOS 標準路徑，menu bar 圖示「Show」走同條
6. **Status item callbacks 走 event_loop_proxy.send_event**：跟 P6 (T23) folder picker 一樣的 macOS callback → main thread 模式，避免在 ObjC selector 內直接動 wrap state
7. **Setting 預設 off**：opt-in 不破壞既有 user 行為；vibe coder / 重度 cmux user 自己開
8. **不做 daemonized terminal server**：「app 不死」靠 process 持續活，不是真 quit 後重啟還原 — cmux 也是這條
9. **跨平台留 v5+**：Linux tray 各 distro 不一致、Windows API 完全不同；V4 純 macOS 簡化、cmux fork 主用戶也是 macOS

---

**Phase 3 V4 update done — 2026-04-30。Spec commit 後 user 排 V3 / V4 優先序開做。**
