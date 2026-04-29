-- Sidebar folder workspaces (cmux-style: workspace = folder, multiple tabs per workspace).
-- See specs/sidebar-folder-workspaces/{PRODUCT,TECH,TASKS}.md.

CREATE TABLE folder_workspaces (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  name TEXT NOT NULL,
  path TEXT NOT NULL,
  display_order INTEGER NOT NULL DEFAULT 0,
  collapsed BOOLEAN NOT NULL DEFAULT 0,
  created_ts TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Tab → workspace association. Nullable so flag-off / pre-bootstrap tabs work unchanged.
-- SQLite ALTER TABLE ADD COLUMN cannot declare FOREIGN KEY; the relationship is expressed
-- in Diesel's schema.rs via `joinable!(tabs -> folder_workspaces (folder_workspace_id))`.
ALTER TABLE tabs ADD COLUMN folder_workspace_id INTEGER;
