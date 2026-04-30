-- Per-folder default command (V3 of sidebar folder workspaces).
-- See specs/sidebar-folder-workspaces/{PRODUCT,TECH,TASKS}.md V3 增量規格.

ALTER TABLE folder_workspaces ADD COLUMN default_command TEXT;
