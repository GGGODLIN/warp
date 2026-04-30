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

# V3 增量規格 — Per-folder default command（2026-04-30）

> **Why**: cmux 工作流程 port 到 wrap 的最後一塊。cmux 使用者主要靠 `~/.zshrc` 的 zero-action auto-launch 在 mux pane + cwd 白名單時自動 spawn `claude`；wrap 改成「**workspace 是顯式選擇 → 把 default command 掛在 workspace setting**」，比 cmux 路徑前綴白名單語意更明確。
>
> **設計核心**：cmux 5 條 guard（`$CMUX_WORKSPACE_ID` / `$TERM_PROGRAM` / `$CMUX_SKIP_CLAUDE` / `$- == *i*` / cwd 白名單）在 wrap 內建模式**全部不需要復刻**，理由見下表。

| cmux guard | wrap 對應 | 取代方式 |
|---|---|---|
| `$CMUX_WORKSPACE_ID` 非空 | wrap 自己 spawn tab 必在 wrap 內 | 不需 |
| `$TERM_PROGRAM == "ghostty"` 防 env inherit | 同上，不靠 env 傳遞 | 不需 |
| `$- == *i*` interactive shell | wrap spawn 必為 interactive | 不需 |
| cwd 白名單 | folder workspace 已是顯式選擇 | 由 user 加 ws 取代 |
| `$CMUX_SKIP_CLAUDE` opt-out | per-tab opt-out | Modifier key / 右鍵 menu |

## V3 user stories

### S6 Per-workspace default command（必做）

- 每個 folder workspace 可設一個 `default_command`（字串，可 None）
- 在該 workspace 開新 tab 時，shell 起來後**自動跑 `default_command`**
- 預設值由全域 setting 決定（見 S7）
- **不能用 `exec` 取代 shell**：command 結束後 shell 必須仍存活、prompt 回來；wrap 既有 LaunchConfig `CommandTemplate` 路徑滿足此需求
- Empty / None default_command → 行為跟現在一樣（純 shell prompt）

### S7 Settings：新 workspace 預設 default command（必做）

- Settings 加 `default_command_for_new_folder_workspaces`（字串，預設 `"claude"`）
- 建立新 folder workspace 時，`default_command` 自動帶該 setting 值
- 改 setting 不影響既有 workspace（既有 ws 自己的 default_command 已存 DB）
- 設成空字串 → 新 ws 預設不跑任何 command

### S8 Opt-out：開 tab 時跳過 default command（必做）

- 即使 workspace 設了 default_command，使用者要能單次跳過：
  - **Modifier key**：⌥-click `+ New Tab` 或 ⌥-Cmd+T → 跳過
  - **右鍵 menu**：`+ New Tab` 按鈕右鍵 → 「Open without default command」
- 跳過時行為等同 default_command = 空（純 shell）
- 設定 default_command 後 default 行為仍為「自動跑」，opt-out 是顯式

## V3 不在範圍內

- Per-tab override（一個 ws 內不同 tab 跑不同 command）— 過度設計
- Multi-command（連續跑多個 command）— LaunchConfig CommandTemplate Vec 結構支援，但 v3 只接 1 command
- Pane split 各跑不同 command（cmux multi-pane workflow）— 需要更深整合，先看 v3 落地反饋
- Command 模板變數（`{{cwd}}` / `{{branch}}`）— 過度設計
- Per-OS / per-shell 不同 default command — 過度設計

## V3 Open Questions

- **Settings UI 位置**：放 Settings → Features → Folder Workspaces 子分頁？還是 Subsettings 內 inline？— T27 grep 既有 settings 結構決定
- **Opt-out 在 cmd+T 開 tab 時的觸發路徑**：cmd+T 在 wrap 是 `AddTerminalTab` action 不是 `AddTabToFolderWorkspace`，`assign_default_folder_workspace_to_active_tab` 是事後 reassign — opt-out 要在這條路徑也支援嗎？v3 先只 cover sidebar 內 `+ New Tab` 路徑，cmd+T 的 reassign 路徑不接 opt-out（v4 再評估）
- **Per-ws default_command edit UI**：抄 P2 inline editor pattern（T18）做一個 default_command editor？還是 reuse 同一個 ViewHandle？— T29 實作時看狀況

## V3 Success Criteria

- [ ] **S6** Workspace `default_command` 設成 `claude` → 新 tab 開起 → claude 自動 spawn 在 prompt
- [ ] **S6** Command 結束（Ctrl+C 或 exit）→ shell prompt 回來，tab 不死
- [ ] **S6** `default_command` 為 None / 空 → 新 tab 純 shell（無自動 spawn）
- [ ] **S6** 既有 workspace（V1/V2 建的）`default_command = NULL` → 行為不變
- [ ] **S7** Settings 預設 `default_command_for_new_folder_workspaces = "claude"`
- [ ] **S7** 改 setting 為 `nvim`、新增 ws → 新 ws default_command = `nvim`
- [ ] **S7** 改 setting 不影響既有 ws
- [ ] **S8** ⌥-click `+ New Tab` → 跳過 default_command（純 shell）
- [ ] **S8** 右鍵 `+ New Tab` → 「Open without default command」選項可用
- [ ] **S8** opt-out 不會清掉 ws.default_command（單次跳過 only）
- [ ] **Quality**：DB schema migration 來回過、`./script/presubmit` 過、新 code 80% coverage、flag-off 行為跟 upstream 一致

---

**V3 spec done — 2026-04-30。Awaiting TECH.md V3 + TASKS.md V3 update + commit + 開做。**

---

# V4 增量規格 — Close to menu bar（2026-04-30）

> **Why**: cmux 工作流程 port 到 wrap 的最後一塊。cmux 行為是「**app 不真的退出**，window 隱藏到 menu bar 圖示，process 持續執行」 — 這樣 claude session、shell、scrollback 全部活在 memory 內，下次點圖示恢復 window 一切還在。
>
> **設計核心**：**不是「真退出後重啟還原」**（那要 daemonize terminal server + reconnect + scrollback 持久化，月以上工程），是「app 不死，只 hide window」 — cmux 也是這條路徑。

## V4 範圍邊界

**做**：
- macOS only（cmux 也 macOS only；cross-platform tray 是 v5+）
- 「app 不死、window 隱藏到 menu bar 圖示」單一 feature
- Setting opt-in（預設 off 不破壞既有行為）

**不做（v5+）**：
- 跨平台（Linux libappindicator、Windows Shell_NotifyIcon）
- 「真退出後重啟還原 running process」（daemonized terminal server）
- Scrollback 跨重啟持久化
- Cmux 的「sidebar 通知圖示」/「unread count」進 menu bar 圖示

## V4 user stories

### S9 Close to menu bar（必做）

- 開了 setting 後，⌘W / 點 window 關閉鍵 / 「最後一個 window close」**不退出 app**
- Window 隱藏（既有 `hide_window` API），process 繼續跑
- claude / shell / running command 全部不死
- ⌘Q 才真退出 app（不變）
- 設定 off 時行為跟現在完全一致（既有 close = quit 路徑）

### S10 Menu bar status icon（必做）

- App active 時 menu bar 看到 wrap 圖示
- 點圖示 → drop-down menu 至少兩個 item：
  - 「Show Warp」→ 恢復隱藏的 window（沒 window 就建一個新的）
  - 「Quit Warp」→ 真退出 app（走 `[NSApp terminate]`）
- 圖示是 wrap 既有 logo 的 menu bar variant（黑白 template image）

### S11 Setting toggle（必做）

- Settings 加 `close_to_menu_bar`（bool，預設 `false`）
- Setting on → S9 + S10 行為啟用
- Setting off → 既有行為，無 menu bar 圖示，close = quit
- Setting 改動立即生效（不用重啟 app）

### S12 Hide Dock icon (optional, 評估後決定)

cmux 模式是「Dock 圖示也隱藏，只留 menu bar 圖示」（macOS `setActivationPolicy(.accessory)` 或 `LSUIElement = true`）。

**Open question — 跟 user 確認**：
- (a) 跟 cmux 完全一樣，setting on → 隱藏 Dock 圖示
- (b) 預設 Dock 仍在，加第二個 setting 「Hide from Dock when closed to menu bar」獨立控制
- (c) v4 只做 menu bar 不碰 Dock，保留 Dock 圖示永遠在（最小化風險）

v4 預設 (c)，待 user push back 再加 (a)/(b)。

## V4 不在範圍內

- **Daemonized terminal server**：wrap 的 terminal server 是 GUI fork 出來的，GUI 死它也死。要做「真退出後 process 還活」必須 daemonize + reconnect 機制 — v5+ 大工程
- **跨平台 tray**：Linux tray 各 distro 支援不齊（GNOME 預設拿掉 tray），Windows API 跟 macOS 完全不同 — v5+
- **多視窗 hide-all 行為**：v4 假設 user 主要單視窗用法；多視窗時「last window close 才 hide」邏輯（其他 window 仍正常 close）
- **Auto-hide on lose focus**：跟 quake mode `hide_window_when_unfocused` 不同概念，後者是 quake mode 行為，前者是 V4 close 路徑

## V4 Open Questions

- **S12 Dock icon 行為**：(a)/(b)/(c) 哪個 — 待 user push back
- **macOS App 已 hide 但 user ⌘Tab 切回來**：行為自動 unhide window 還是 no-op？標準 macOS 應該 unhide，但實作上要驗證
- **`applicationShouldTerminate:` hook 與 setting 互動**：既有 [`app.m:257`](file:///Users/linhancheng/Desktop/projects/warp-fork/crates/warpui/src/platform/mac/objc/app.m) 處理 confirm dialog；setting on 時要在哪一層攔截 close → 在 close window event 端攔（建議），不在 NSApp terminate 端
- **Menu bar 圖示 click action**：點圖示直接顯示 menu，還是 left-click = Show Warp + right-click = menu？標準 macOS 是「click 直接顯示 menu」（NSStatusItem.menu），v4 採此

## V4 Success Criteria

- [ ] **S9** Setting on + 點 window 關閉鍵 → window 隱藏，wrap process PID 仍存在
- [ ] **S9** Setting on + ⌘W → window 隱藏（不 quit）
- [ ] **S9** Setting on + window 隱藏狀態下 claude session 仍跑（macOS Activity Monitor 看 wrap process 還在）
- [ ] **S9** Setting on + ⌘Q → 真 quit
- [ ] **S9** Setting off → 行為跟現在完全一致
- [ ] **S10** Setting on → menu bar 看到 wrap 圖示
- [ ] **S10** 點圖示 → 看到「Show Warp」「Quit Warp」
- [ ] **S10** Setting on + 「Show Warp」→ window 恢復；點時沒 window 也建一個新的
- [ ] **S10** Setting on + 「Quit Warp」→ 真 quit
- [ ] **S10** Setting off → menu bar 沒 wrap 圖示
- [ ] **S11** Settings UI 看到「Close to menu bar」toggle，預設 off
- [ ] **S11** Toggle 後立即生效（不用重啟 app）
- [ ] **Quality**：`./script/presubmit` 過、不 break flag-off 行為、既有 quit / close 路徑無 regression

---

**V4 spec done — 2026-04-30。Awaiting TECH.md V4 + TASKS.md V4 update + commit + 排優先序。**

---

**Phase 1 done. Awaiting human review before Phase 2 (PLAN).**
