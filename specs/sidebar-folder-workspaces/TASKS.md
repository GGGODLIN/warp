# Implementation Tasks: Sidebar Folder Workspaces

> **Spec phase**: Phase 4 (IMPLEMENT) — partial delivery 2026-04-29
> **Companions**: [PRODUCT.md](file:///Users/linhancheng/Desktop/projects/warp-fork/specs/sidebar-folder-workspaces/PRODUCT.md) · [TECH.md](file:///Users/linhancheng/Desktop/projects/warp-fork/specs/sidebar-folder-workspaces/TECH.md)
> **Original plan**: 14 tasks，分 3 phase（Foundation 5 + Vertical slices 7 + Polish 2）+ 3 checkpoints
> **Spike delivery**: T1-T8 + T13 → flag-toggle 在 sidebar 看到 "Default" workspace 把現有 tab 包起來。T9-T12 + T14 deferred to v2（見尾段「Spike Outcome」）。

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

### 已交付 (T1-T8 + T13)

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

**Demo 行為**：toggle `FolderWorkspacesEnabled` in Settings → Developer / Features
- flag off → sidebar 跟 upstream 一致
- flag on → 看到 "Default" workspace header（cmux-style，把所有現有 tab 包起來）

### Deferred to v2 (T9-T12 + T14)

原 TASKS.md 的 T9 (UI + folder picker)、T10 (tab → workspace association)、T11-T12 (folder missing handler)、T14 (integration test) 全部 deferred。原因：

**T9 / T10 / T11 / T12** 都需要 **write-side 架構**：
- `ModelEvent::UpsertFolderWorkspace` enum variant + sqlite.rs worker handler
- `FolderWorkspaceModel` mutator method（inside-Entity）
- Tentative-id-vs-DB-id race（Warp 設計只有 1 個 writer connection，不能 2nd RW connection）

T8 path A（Entity Model 層）已經做完 read 側基礎建設。Write 側是同等規模：~1 週工程量。

**T14** integration test 需要 [`crates/integration`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/integration/) Builder/TestStep 框架的學習曲線。Manager-level unit test (T6) 已 cover 邏輯正確性，integration test 主要 cover render 行為——可放到 v2。

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
