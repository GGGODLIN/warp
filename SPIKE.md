# Warp Fork Spike — Workspace Grouping

**起點日期**：2026-04-29
**Fork**：[`GGGODLIN/warp`](https://github.com/GGGODLIN/warp)（自動 sync from upstream `warpdotdev/warp`）
**Upstream**：[`warpdotdev/warp`](https://github.com/warpdotdev/warp)

## 為什麼 spike 這個

Warp 開源 2 天（2026-04-28 公告，AGPL-3.0）。**核心痛點**：sidebar 是平 tab list，缺 workspace / project 兩層樹分組（cmux 那種「workspace 包 panes」結構）。

**現況**：
- Canonical issue [#2314 — Pinned / Locked tabs and tab groups](https://github.com/warpdotdev/Warp/issues/2314) — 137+ 👍 / 3.5 年無進度 / 現 label `needs-mocks`
- 4 個 dup issues 在 2026-04 全 close 為 dup
- **沒任何 fork / PR 在攻這題**（2026-04-29 確認）
- 開源剛過 2 天，社群還沒卡位，現在是時機

## Spike 目標

**1 週 sunk cost 換真實工程量資訊**，不是要 ship 完整 grouping。

驗證問題：
1. 環境跑得起來嗎？（`./script/bootstrap` + `cargo run` 過）
2. Sidebar render 進入點清楚嗎？（從 `app/src/workspace/view.rs` 開始追）
3. Backend 已有 workspace/project 概念嗎？（`app/src/projects.rs` + `app/src/workspaces/`(複數) + `app/src/pane_group/`）
4. WarpUI Entity-Handle pattern 學得起來嗎？
5. 能不能寫個 hello-world patch（如「在 sidebar 第一個 tab 旁加假 group header」）？

## Day-by-day Plan

### Day 1-2: 環境
```bash
./script/bootstrap     # macOS bootstrap
cargo run              # 確認 build 過 + 跑起來
./script/presubmit     # fmt / clippy / test 通過
```
→ Bootstrap fail → **退回 path C/D（cmux 留著 + 等社群推）**

### Day 3-4: 找 sidebar render 進入點
用 CC 4.7 探索 + 自己讀（建議啟動 CC 在 spike repo 內）：
- `app/src/workspace/view.rs`（單數，sidebar UI 根）
- `app/src/workspaces/`（複數！refactor 暗示）
- `app/src/pane_group/`
- `crates/warpui/`（Entity-Handle UI 框架）

→ render path 清楚 → 繼續
→ 全 GPU shader 邏輯動不到 → **退**

### Day 5: 摸清 backend 概念
讀 `app/src/projects.rs` 跟 `app/src/workspaces/`，判斷：
- 是否已有 workspace grouping data model（強烈暗示有）
- 還是要從零建

決定工程量：3-4 週（已有概念）vs 5-7 週（從零）

### Day 6-7: Hello-world patch
不做完整 grouping，做最小驗證——例如「在 sidebar 第一個 tab 旁加假 group header」+ feature flag 包住。

→ 跑通 → **path B（local-only fork，3-4 週）**
→ 卡死 → **path C（推 #2314 mock/spec，agent-first contribution）**

## 已知架構（從 SPIKE 前查證）

### Repo 結構
```
warp-fork/
├── Cargo.toml          # workspace, default-members 11 個 crate
├── app/                # 主 binary
│   └── src/
│       ├── workspace/  # ⭐ 單數，sidebar UI 根 (view.rs / view/)
│       ├── workspaces/ # ⭐ 複數！refactor 中
│       ├── pane_group/ # ⭐ pane group 已第一公民
│       ├── projects.rs # ⭐⭐ project 概念已在
│       ├── tab_configs/
│       ├── persistence/  # SQLite + Diesel ORM
│       ├── drive/      # 雲端 sync (client side)
│       └── ...
├── crates/             # 34+ crates
│   ├── warpui/         # ⭐⭐⭐ 自家 GPU UI 框架 (Entity-Handle)
│   ├── warp_core/      # features.rs (FeatureFlag enum)
│   ├── persistence/
│   └── ...
└── script/             # bootstrap / presubmit
```

### Tech Stack（確認）
- **Rust**（46 MB source）+ **WGSL/Metal GPU shader**
- **不是 Electron / 不是 JS**
- WarpUI Entity-Handle pattern（類似 Zed GPUI）
- SQLite + Diesel ORM
- GraphQL 連雲端 server（**server 閉源**）
- Feature flag 系統 friendly（`crates/warp_core/src/features.rs` 加新 variant）

### Drive 雲端 sync 死穴
- `app/src/drive/` 有 client side code
- 但 sync 雲端要動 GraphQL server schema，**server 閉源**
- **Spike 路線：local-only fork**，不 sync 雲端 group state，每台機器各自設

## 已知陷阱

1. **git-lfs 必裝**（`brew install git-lfs && git lfs install`）—— 否則 clone 後 binary 資產 checkout 失敗（已踩，已修）
2. **WarpUI 學習曲線最大風險**（自家框架，無社群文件，Rust + GPU shader 雙重新挑戰）
3. **macOS Gatekeeper 簽名**：self-built fork 不簽名會被擋
   - 用 free Apple ID dev 簽，每 7 天 rebuild
   - 或 Apple Developer $99/年
4. **Upstream rebase 持續成本**：Warp daily merge 大量 PR（4/29 一天 20+），fork 維護成本高
5. **`MouseStateHandle::default()` 在 render 內會壞**（WARP.md 警告）
6. **`model.lock()` on TerminalModel 重複 lock 會 deadlock**（WARP.md 警告）

## 三條路徑決策樹

```
spike 全跑通 + backend 已有 grouping 概念
  → path B (local-only fork, 3-4 週 ship)

spike 部分卡 (e.g. WarpUI 學得慢)
  → path C (推 #2314 mock + spec, Warp Oz agent 自己寫)

spike 環境/render 都卡死
  → path D (cmux 留著 + 等社群推 #2314)
```

## Cross-link to Memory

- [`reference_warp_terminal_for_cc_user`](file:///Users/linhancheng/.claude/projects/-Users-linhancheng-Desktop-projects/memory/reference_warp_terminal_for_cc_user.md) — Warp 完整 reference（stack / 開源 / CC 整合 / Warp Agent / fork 評估）
- [`project_cmux_setup`](file:///Users/linhancheng/.claude/projects/-Users-linhancheng/memory/project_cmux_setup.md) — cmux 已滿足同個 grouping 痛點
- [`feedback_new_project_folder_rule`](file:///Users/linhancheng/.claude/projects/-Users-linhancheng-Desktop-projects/memory/feedback_new_project_folder_rule.md) — `~/Desktop/projects/<name>/` 規則
- [`feedback_fact_check_every_point_not_just_thesis`](file:///Users/linhancheng/.claude/projects/-Users-linhancheng-Desktop-projects/memory/feedback_fact_check_every_point_not_just_thesis.md) — Warp 是 Rust 不是 Electron 那次教訓

## Spike 失敗也不浪費

Spike 卡死的話：
- 觀察寫成 [#2314](https://github.com/warpdotdev/Warp/issues/2314) 的 mock + spec
- 推 `needs-mocks` → `ready-to-spec`
- Warp 自家 Oz agent-first contribution 會去 implement
- 你的 1 週變成「spec author」角色，工作做掉但不用碰 Rust
