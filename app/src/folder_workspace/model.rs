use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::persistence::schema::folder_workspaces;

/// A folder-bound workspace that groups tabs in the sidebar (cmux-style).
///
/// Each `FolderWorkspace` corresponds to one file system folder. Tabs may
/// reference a workspace via the nullable `tabs.folder_workspace_id` column.
/// See specs/sidebar-folder-workspaces/.
#[derive(Debug, Clone, Identifiable, Queryable, Selectable)]
#[diesel(table_name = folder_workspaces)]
#[diesel(primary_key(id))]
pub struct FolderWorkspace {
    pub id: i32,
    pub name: String,
    pub path: String,
    pub display_order: i32,
    pub collapsed: bool,
    pub created_ts: NaiveDateTime,
    pub default_command: Option<String>,
}

/// A new folder workspace ready to insert. `id` is autoincrement and
/// `created_ts` defaults to `CURRENT_TIMESTAMP` per the migration.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = folder_workspaces)]
pub struct NewFolderWorkspace {
    pub name: String,
    pub path: String,
    pub display_order: i32,
    pub collapsed: bool,
    pub default_command: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::sqlite::SqliteConnection;
    use diesel_migrations::MigrationHarness;

    #[test]
    fn round_trip_through_in_memory_sqlite() {
        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        conn.run_pending_migrations(persistence::MIGRATIONS).unwrap();

        let new = NewFolderWorkspace {
            name: "test".to_string(),
            path: "/tmp/test".to_string(),
            display_order: 0,
            collapsed: false,
            default_command: None,
        };

        let inserted = diesel::insert_into(folder_workspaces::table)
            .values(&new)
            .execute(&mut conn)
            .unwrap();
        assert_eq!(inserted, 1);

        let all: Vec<FolderWorkspace> = folder_workspaces::table
            .select(FolderWorkspace::as_select())
            .load(&mut conn)
            .unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "test");
        assert_eq!(all[0].path, "/tmp/test");
        assert_eq!(all[0].display_order, 0);
        assert!(!all[0].collapsed);
        assert!(all[0].id > 0);
        assert_eq!(all[0].default_command, None);
    }

    #[test]
    fn default_command_round_trip() {
        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        conn.run_pending_migrations(persistence::MIGRATIONS).unwrap();

        let with_cmd = NewFolderWorkspace {
            name: "with-cmd".to_string(),
            path: "/tmp/a".to_string(),
            display_order: 0,
            collapsed: false,
            default_command: Some("claude".to_string()),
        };
        let with_empty = NewFolderWorkspace {
            name: "with-empty".to_string(),
            path: "/tmp/b".to_string(),
            display_order: 1,
            collapsed: false,
            default_command: Some(String::new()),
        };
        let without_cmd = NewFolderWorkspace {
            name: "without".to_string(),
            path: "/tmp/c".to_string(),
            display_order: 2,
            collapsed: false,
            default_command: None,
        };

        for new in [&with_cmd, &with_empty, &without_cmd] {
            diesel::insert_into(folder_workspaces::table)
                .values(new)
                .execute(&mut conn)
                .unwrap();
        }

        let mut all: Vec<FolderWorkspace> = folder_workspaces::table
            .select(FolderWorkspace::as_select())
            .order(folder_workspaces::display_order.asc())
            .load(&mut conn)
            .unwrap();
        assert_eq!(all.len(), 3);

        let without = all.pop().unwrap();
        let empty = all.pop().unwrap();
        let claude = all.pop().unwrap();
        assert_eq!(claude.default_command.as_deref(), Some("claude"));
        assert_eq!(empty.default_command.as_deref(), Some(""));
        assert_eq!(without.default_command, None);
    }
}
