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

## Open Tech Questions（Phase 3 task breakdown 前確認）

1. **Tabs 表 structure**：現有 tabs 表長怎樣？加 column 還是 junction table？→ Phase 3 開始前先 grep schema.rs
2. **Bootstrap migration 觸發點**：app init phase 哪個 hook？→ grep 既有 migration init code 找 callsite
3. **Settings UI toggle 機制**：feature flag 加進 enum 後 settings 是 auto-discovery 還是手動 register？→ trace `runtime_flags_menu_items()` 即知
4. **Folder picker UI**：直接 NSOpenPanel 還是 WarpUI 包好的 helper？→ grep 確認

四個都是 implementation-time 即可解答的，不 block plan review。

## Out of Scope (v2+)

- Cross-workspace tab drag
- Workspace 自訂 icon / color / emoji
- 跨 workspace cmd palette / search
- Cloud sync（GraphQL server 閉源 + spike 範圍）
- Upstream PR（屆時看 Issue [#2314](https://github.com/warpdotdev/Warp/issues/2314) 的 mocks 進展再決定）

---

**Phase 2 done. Awaiting human review before Phase 3 (TASKS)。**

Phase 3 用 [`agent-skills:planning-and-task-breakdown`](file:///Users/linhancheng/.claude/plugins/cache/addy-agent-skills/agent-skills/1.0.0/skills/planning-and-task-breakdown) skill 把這份 plan 拆成 ~10-15 個 acceptance-tested tasks。
