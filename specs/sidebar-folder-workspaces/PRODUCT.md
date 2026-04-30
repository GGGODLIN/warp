# Spec: Sidebar Folder Workspaces

> **Spec phase**: Phase 1 (SPECIFY) — awaiting human review before Phase 2 (PLAN)
> **SDD framework**: agent-skills:spec-driven-development（locked 2026-04-29T21:49+0800 in `~/.claude/sdd-framework-log.md`）
> **Day 1 evidence**: env build / bundle / launch verified（`target/debug/warp-oss` 696MB，PID-tested）

## Objective

把 cmux 風格的「workspace = 資料夾，每個 workspace 多個 tab」結構移植到 Warp 的 sidebar，類似瀏覽器分頁。

**痛點**：Warp 現有 sidebar 是平 tab list，使用者開多個專案時 tab 全部混在一起，找上下文要切來切去。Issue [#2314](https://github.com/warpdotdev/Warp/issues/2314)（137+ 👍 / 3.5 年無進度 / 現 label `needs-mocks`）是 canonical 請求，沒任何 fork / PR 在攻。

**動機**：使用者目前 daily driver 是 [cmux](https://github.com/manaflow-ai/cmux)，cmux 滿足這個 grouping 但是 macOS-only Swift app；目標是把這個概念帶進 Warp，讓 Warp 能取代 cmux 的工作區角色。

**用戶故事**:
- 從 sidebar 工具列按一個 "+" 按鈕，開啟 macOS folder picker，選一個資料夾建立 workspace
- 建立後 workspace 顯示在 sidebar，header 是該資料夾的 basename（譬如 `~/code/foo` → `foo`），可改名
- 在 workspace 內開多個 tab，每個 tab 預設 cwd = workspace 對應的資料夾
- 收起 / 展開 workspace，重開 Warp 後狀態保留
- 資料夾被外部刪掉 → sidebar tab 顯示 warning icon + tooltip；新開 tab 時 cwd fallback `$HOME` + 一次性 toast；workspace 不自動移除
- 升級到帶此 feature 的 fork → 所有現有 tab 進預設 "Default" workspace（folder = `$HOME`），不丟

## Tech Stack

- Rust 1.92.0（per `rust-toolchain.toml`）
- WarpUI Entity-Handle pattern（[`crates/warpui_core/src/core/entity.rs:35-41`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/warpui_core/src/core/entity.rs)）
- SQLite via Diesel ORM
- WGSL / Metal GPU UI（不會動）
- Feature flag system：enum 真正定義在 [`crates/warp_features/`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/warp_features/)，[`crates/warp_core/src/features.rs`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/warp_core/src/features.rs) 只是 `pub use warp_features::*;` re-export
- Diesel migrations：[`crates/persistence/migrations/`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/persistence/migrations/) + [`crates/persistence/src/schema.rs`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/persistence/src/schema.rs)（Diesel CLI auto-gen，搭配 `schema.patch` 手寫 patch file 自定義型別）

## Commands

```bash
# Build (debug)
cargo build --bin warp-oss --features gui

# Bundle .app + ad-hoc codesign（macOS dev workflow）
./script/run --dont-open
open target/debug/bundle/osx/WarpOss.app

# Run unit tests
cargo test --bin warp-oss --features gui

# Lint / format / presubmit (CI 等價)
./script/presubmit

# DB migration generation
diesel migration generate folder_workspaces \
  --migration-dir crates/persistence/migrations
```

## Project Structure

新增 / 修改：

```
app/src/
├── folder_workspace/          # ⭐ 新 module（業務邏輯 + UI）
│   ├── mod.rs                 # public re-exports
│   ├── model.rs               # FolderWorkspace struct + Diesel Queryable/Insertable
│   ├── manager.rs             # CRUD + folder existence check + bootstrap migration
│   ├── view.rs                # WarpUI render integration（feature-flag gated）
│   └── tests.rs
└── workspace/view/vertical_tabs.rs   # 修改：feature-flag 包住 grouping render

crates/persistence/migrations/<ts>_folder_workspaces/   # Diesel migration
├── up.sql
└── down.sql
crates/persistence/src/schema.rs       # auto-regen 後加 folder_workspaces 表

crates/warp_features/                  # 加 FolderWorkspacesEnabled enum variant

specs/sidebar-folder-workspaces/
├── PRODUCT.md                          # 這份
├── TECH.md                             # Phase 2 後寫
└── tasks.md                            # Phase 3 後寫
```

**避免動到的範圍**：
- [`app/src/workspaces/`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/workspaces/)（複數！cloud team workspace，跟 folder workspace 是不同概念）
- [`crates/warpui/`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/warpui/) / [`crates/warpui_core/`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/warpui_core/)（自家 UI 框架）
- [`app/src/projects.rs`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/projects.rs) 的 `ProjectManagementModel`（與 FolderWorkspace 並存，v1 不合併）
- 任何 GraphQL / cloud sync server-side schema

## Code Style

按 Warp 既有 Rust 風格 + [`WARP.md`](file:///Users/linhancheng/Desktop/projects/warp-fork/WARP.md) 警告：
- 4 空格縮排、snake_case 函數、PascalCase 型別
- WarpUI fluent API 風格（範例摘自 [`crates/warpui_core/src/ui_components/text.rs:56-74`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/warpui_core/src/ui_components/text.rs)）：

```rust
impl UiComponent for FolderWorkspaceHeader {
    type ElementType = Container;
    fn build(self) -> Container {
        let styles = self.styles;
        Container::new(
            Text::new(
                self.name,
                styles.font_family_id.unwrap(),
                styles.font_size.unwrap_or_default(),
            )
            .with_color(self.text_color),
        )
        .with_padding(Padding::uniform(8.))
        .finish()
    }
}
```

- ⚠️ **禁忌**（per WARP.md）：
    - `MouseStateHandle::default()` 在 render 內會壞
    - `model.lock()` on `TerminalModel` 重複 lock 會 deadlock
- Feature flag check 統一用：

```rust
if FeatureFlag::FolderWorkspacesEnabled.is_enabled() {
    // new grouping path
} else {
    // legacy flat sidebar
}
```

## Testing Strategy

- **Unit**：每個 module 內 `#[cfg(test)] mod tests`，cover：
    - `FolderWorkspace` CRUD
    - Folder existence check + fallback `$HOME`
    - Diesel migration up / down 來回
    - Bootstrap migration（既有 tab → "Default" workspace）
- **Integration**：[`crates/integration`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/integration/) Builder/TestStep framework（依 `warp-integration-test` skill 寫法）
    - End-to-end: 建 workspace → 開 tab → 收/展 → 重開保留 → folder 刪除 + warning + fallback
- **Coverage 目標**：新 module 80% via `cargo-llvm-cov`
- **Manual smoke**：每個 task 結束跑一次 `./script/run --dont-open && open …` 確認 GUI 沒 regress

## Boundaries

**Always do**:
- 所有新行為包在 `FolderWorkspacesEnabled` feature flag 後
- DB migration 寫 `down.sql`（可 rollback）
- 寫測試（unit + at least 1 integration）
- 跑 `./script/presubmit` 過 fmt + clippy + test 才 commit
- Conventional commit message（feat / fix / refactor / test / docs）

**Ask first**:
- 動 [`app/src/workspace/view/vertical_tabs.rs`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/workspace/view/vertical_tabs.rs) 的 grouping 以外段落
- 改 `Project` struct（[`app/src/projects.rs`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/projects.rs)）— 預設並存
- 加新 cargo dependency
- 改 feature flag 預設狀態（譬如從 dogfood 推到 stable）

**Never do**:
- 動 GraphQL / cloud sync server-side schema
- 動 [`crates/warpui/`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/warpui/) 自家框架
- 動 [`app/src/workspaces/`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/workspaces/)（複數，cloud team workspace）
- Bypass feature flag 直接改既有 sidebar render path
- Commit secrets、API key、auth token

## Success Criteria

可驗證 / 可測試的完成標準：

- [ ] **Feature flag toggle**：Settings → Developer / Features 可開關 `FolderWorkspacesEnabled`
- [ ] **關閉時 sidebar 同 upstream**：flag off → sidebar render path 跟 upstream `warpdotdev/warp` 相同（`vertical_tabs.rs` 既有 path 不變動，diff 限縮在新增分支）
- [ ] **建立 workspace**：sidebar "+" 按鈕觸發 macOS folder picker → 選資料夾後 workspace 出現在 sidebar
- [ ] **預設名稱**：新 workspace 的 name = folder basename（`~/code/foo` → `foo`）
- [ ] **Tab 歸屬 + cwd**：在 workspace 內開新 tab → tab 顯示在該 workspace 下 + 預設 cwd = workspace folder path
- [ ] **持久化**：建立 workspace、改 collapse 狀態、開 tab → 關閉 Warp 重開 → 全部還原
- [ ] **Bootstrap migration**：第一次升級到帶此 feature 的版本 → migration 把所有既有 tab 灌進 "Default" workspace（folder = `$HOME`）
- [ ] **Folder missing 行為**：
    - 手動 `rm -rf` workspace 對應 folder → sidebar tab 顯示 warning icon + tooltip "Folder no longer exists"
    - 在該 workspace 開新 tab → cwd fallback `$HOME` + 一次性 toast 通知
    - 重新 `mkdir` 同名 folder → warning icon 自動消失（每次 render 重 check `fileExists`）
- [ ] **Quality gates**：`./script/presubmit` 過、新 code 80% coverage、`cargo clippy` 無 warning
- [ ] **不破壞既有 sidebar**：flag off 時所有既有 tab / pane / drag-and-drop 行為跟 upstream 一致（manual smoke test pass）

## Open Questions（v1 範圍外，記錄供日後決定）

- **v2 nice-to-have**: cross-workspace tab drag、workspace 自訂 icon / color / emoji、跨 workspace cmd palette / search
- **Upstream PR 計畫**: Issue [#2314](https://github.com/warpdotdev/Warp/issues/2314) 還是 `needs-mocks`，沒人在攻；fork 完成後是否 push upstream 屆時再評估
- **與 [`projects.rs`](file:///Users/linhancheng/Desktop/projects/warp-fork/app/src/projects.rs) 的 Project 整合**：v1 並存，不合併；v2 視屆時情況決定
- **數量上限**：workspace 數量 / tab per workspace 暫不設限，UI 過多走 sidebar scroll
- **Cross-machine sync**：永遠不做（GraphQL server 閉源 + spike 範圍）

---

# V2 增量規格（2026-04-30 後補）

> **Why**: V2 session 在沒 spec 的情況下做了 V5-V10 + 一批 polish；之後 user 確認規格有缺漏，回填這份增量規格供下次接手對齊。命名與 V2_HANDOFF.md 內 `V1-V9` 編號**不同**（handoff 是 work group 編號，這裡是 user story 編號），避免混淆 — 下表對齊。

## V2 已交付 / 部分交付的 user stories

### S1 Workspace 可整理（rename / reorder / delete）

- 從 sidebar header 右鍵 menu 選 Rename / Move Up / Move Down / Delete 操作
- 從 sidebar header 上的 icon button row 點 ✎↑↓✕ 也是同樣行為
- Delete 時 tabs reassign 到第一個 remaining workspace（沒則 fwid → None）— **不丟工作**
- Rename 用文字輸入框（**目前 osascript dialog；polish 後改 inline editor**）
- Reorder 用兩個方向鍵切順序（**目前用 Move Up/Down；polish 後加 drag header**）

對應交付：commit `4c6a7c3`。Polish 未做：inline editor / drag header / Hoverable / svg icons / delete confirm。

### S2 Workspace header 顯示更多識別資訊

- Header line 1：`▾ 📁 <name> (<tab_count>)` 14pt — folder icon + 名稱 + tab 數
- Header line 2：`<path>` 11pt secondary color — 完整路徑
- Folder missing 時 line 1 加 `⚠`
- 收起時 count 跟 path **都還看得到**，summary-at-a-glance 不需展開

對應交付：commit `4c6a7c3`。

### S3 Tab 視覺對換 — 路徑移到 workspace 層級

- 同個 workspace 內所有 tab 都跑同一個資料夾，tab 重複顯示 cwd / branch 是無效資訊
- FolderWorkspacesEnabled on 時 tab 強制 minimal mode：
    - **保留**：title (Command 模式) + active session indicator dot + tab color + right-side badges (unread / agent activity / PR link) + close button (hover) + pane count badge
    - **砍**：second_line (cwd / branch / 別的 subtitle) + 整條 metadata-left line
- 不依 user `VerticalTabsPrimaryInfo` 設定（強制 override）

對應交付：commit `4c6a7c3`。

### S4 Cmd+T 開新 tab 落在「上次操作的 workspace」

- 不新增快捷鍵（`Cmd+Shift+T` 是 Warp 既有 `AddTerminalTab`）
- `FolderWorkspaceModel` 內存 `last_active_id`
- 點 workspace header 或點 `+ New Tab` 都更新 last_active_id
- 既有 cmd+T fallback 改用 last_active_id 而非 first_workspace
- 重啟 app 後 last_active_id reset 為 first workspace（純 in-memory）

對應交付：commit `4c6a7c3`。

### S5 Tab 同 workspace 內可 reorder，跨 workspace 拒絕

- 既有 tab drag 在 grouping mode 下會混淆群組視覺，需限制
- `on_tab_drag` 內加同 `folder_workspace_id` 檢查，跨 workspace 直接 no-op
- 同 workspace 內 reorder 仍透過 `Draggable` + `DropTarget` 既有 infra work

對應交付：commit `4c6a7c3`。

## V2 polish 已知缺口（**還沒做**）

| ID | 項目 | 為何要做 |
|---|---|---|
| P1 | **Drag workspace header** 切順序（不只 Move Up/Down icons） | User 明說「都要」；多 workspace 時方向鍵 N 次太繁瑣 |
| P2 | **Rename 用 inline editor** 取代 osascript dialog | Inline 比 modal 流暢；跟 Warp tab rename pattern 一致 |
| P3 | **Hover 才顯示 icon button row** 而非 always-visible | 減少 sidebar 視覺雜訊；符合 hover 慣例 |
| P4 | **Delete 二次確認** dialog | Workspace 配置刪掉雖 tabs 不丟，但 display_order / name / collapse 全沒了 |
| P5 | **Icon 用 Warp 既有 svg** 取代 unicode (✎↑↓✕📁) | 字體 render 跨平台不一致；svg 可控 |
| P6 | **osascript → event_loop_proxy thread picker**（folder picker + rename dialog 都改） | macOS-only / blocking；production 要改 |

## V1 deferred 仍未做（從原 V2_HANDOFF.md / TECH.md）

| ID | 項目 | Handoff 編號 |
|---|---|---|
| D1 | ModelEvent path for FolderWorkspace mutations（取代 `establish_rw_connection`） | 原 V1.3 |
| D2 | Folder missing 開新 tab → cwd `$HOME` + 一次性 toast | 原 V7 |
| D3 | Integration test via `crates/integration` Builder/TestStep | 原 V8 |
| D4 | Cleanup spike-only changes：revert debug-on flag (`a418008`)、`establish_rw_connection` pub re-export、`#![allow(dead_code)]` in `app/src/folder_workspace/mod.rs` | 原 V9 |

## V2 Success Criteria（補充原 v1 那組）

- [x] **Header lifecycle**：rename / reorder / delete 三件事都可從 header 觸發（icon button **或** 右鍵 menu）
- [x] **Header info density**：name + tab count + path 在 expanded / collapsed 兩個狀態都看得到
- [x] **Tab UI minimal mode**：flag-on 時 tab row 不顯示 cwd / branch / 任何 secondary subtitle
- [x] **Cmd+T placement**：新 tab 落在 last-active workspace（不新增快捷鍵）
- [x] **Tab reorder constraint**：同 ws 內 reorder OK；跨 ws drop no-op
- [ ] **P1 drag header reorder**：workspace header 可 drag 重排
- [ ] **P2 rename inline editor**：rename 用 sidebar 內 inline editor，不 modal
- [ ] **P3 hover icons**：icon button row 預設隱藏，hover header 才顯示
- [ ] **P4 delete confirm**：刪 workspace 跳一次確認 dialog
- [ ] **P5 svg icons**：所有 4 個 header icons 用 Warp svg 不用 unicode
- [ ] **D1 ModelEvent path**：所有 folder workspace mutation 走 ModelEvent，不再 fresh RW connection
- [ ] **D2 missing folder toast**：開 tab 在 missing folder workspace → cwd $HOME + toast 一次
- [ ] **D3 integration test**：`crates/integration` 至少 1 個 e2e test
- [ ] **D4 cleanup**：spike-only changes 全 revert

---

**V2 spec done — 2026-04-30 補。Awaiting Phase 2 (TECH.md) update + Phase 3 (TASKS.md) update.**

---

**Phase 1 done. Awaiting human review before Phase 2 (PLAN).**
