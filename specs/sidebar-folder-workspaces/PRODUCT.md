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

**Phase 1 done. Awaiting human review before Phase 2 (PLAN).**
