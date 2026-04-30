//! CRUD + bootstrap migration for [`FolderWorkspace`].
//!
//! Free functions because the manager is stateless — every operation takes a
//! `&mut SqliteConnection`. Callers own the connection lifecycle.

use std::path::Path;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::persistence::schema::{folder_workspaces, tabs};

use super::model::{FolderWorkspace, NewFolderWorkspace};

/// Insert a folder workspace row using the supplied id + display_order /
/// collapsed (no max + 1 lookup). Used by the ModelEvent worker thread
/// where the in-memory model already chose all values; explicit id
/// keeps `tabs.folder_workspace_id` references consistent across the
/// async write boundary.
pub fn create_with_id_and_attrs(
    conn: &mut SqliteConnection,
    id: i32,
    name: &str,
    path: &str,
    display_order: i32,
    collapsed: bool,
) -> QueryResult<usize> {
    diesel::sql_query(
        "INSERT INTO folder_workspaces (id, name, path, display_order, collapsed) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind::<diesel::sql_types::Integer, _>(id)
    .bind::<diesel::sql_types::Text, _>(name)
    .bind::<diesel::sql_types::Text, _>(path)
    .bind::<diesel::sql_types::Integer, _>(display_order)
    .bind::<diesel::sql_types::Bool, _>(collapsed)
    .execute(conn)
}

/// Create a new folder workspace and return the persisted row (with id /
/// created_ts populated by SQLite).
pub fn create(
    conn: &mut SqliteConnection,
    name: &str,
    path: &Path,
) -> QueryResult<FolderWorkspace> {
    let next_order: i32 = folder_workspaces::table
        .select(diesel::dsl::max(folder_workspaces::display_order))
        .first::<Option<i32>>(conn)?
        .map(|m| m + 1)
        .unwrap_or(0);

    let new = NewFolderWorkspace {
        name: name.to_string(),
        path: path.to_string_lossy().into_owned(),
        display_order: next_order,
        collapsed: false,
    };

    diesel::insert_into(folder_workspaces::table)
        .values(&new)
        .execute(conn)?;

    let last_id: i32 = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>(
        "last_insert_rowid()",
    ))
    .get_result(conn)?;

    get_by_id(conn, last_id)
}

/// Return all folder workspaces ordered by `display_order` ascending.
pub fn get_all(conn: &mut SqliteConnection) -> QueryResult<Vec<FolderWorkspace>> {
    folder_workspaces::table
        .order(folder_workspaces::display_order.asc())
        .select(FolderWorkspace::as_select())
        .load(conn)
}

pub fn get_by_id(conn: &mut SqliteConnection, id: i32) -> QueryResult<FolderWorkspace> {
    folder_workspaces::table
        .find(id)
        .select(FolderWorkspace::as_select())
        .first(conn)
}

pub fn update_collapsed(
    conn: &mut SqliteConnection,
    id: i32,
    collapsed: bool,
) -> QueryResult<usize> {
    diesel::update(folder_workspaces::table.find(id))
        .set(folder_workspaces::collapsed.eq(collapsed))
        .execute(conn)
}

#[cfg(test)]
pub fn delete(conn: &mut SqliteConnection, id: i32) -> QueryResult<usize> {
    diesel::delete(folder_workspaces::table.find(id)).execute(conn)
}

pub fn rename(conn: &mut SqliteConnection, id: i32, new_name: &str) -> QueryResult<usize> {
    diesel::update(folder_workspaces::table.find(id))
        .set(folder_workspaces::name.eq(new_name))
        .execute(conn)
}

pub fn set_display_order(
    conn: &mut SqliteConnection,
    id: i32,
    new_order: i32,
) -> QueryResult<usize> {
    diesel::update(folder_workspaces::table.find(id))
        .set(folder_workspaces::display_order.eq(new_order))
        .execute(conn)
}

/// Delete a workspace, reassigning its tabs to `fallback_id` (or NULL if None).
pub fn delete_with_tab_reassignment(
    conn: &mut SqliteConnection,
    id: i32,
    fallback_id: Option<i32>,
) -> QueryResult<()> {
    diesel::update(tabs::table.filter(tabs::folder_workspace_id.eq(id)))
        .set(tabs::folder_workspace_id.eq(fallback_id))
        .execute(conn)?;
    diesel::delete(folder_workspaces::table.find(id)).execute(conn)?;
    Ok(())
}

/// Idempotent bootstrap: if no folder_workspaces exist but tabs do, create a
/// "Default" workspace (path = `$HOME`) and assign all existing tabs to it.
/// Subsequent calls return `Ok(None)` without modifying state.
pub fn bootstrap_default_workspace_for_existing_tabs(
    conn: &mut SqliteConnection,
) -> QueryResult<Option<FolderWorkspace>> {
    let workspace_count: i64 = folder_workspaces::table.count().get_result(conn)?;
    if workspace_count > 0 {
        return Ok(None);
    }

    let tab_count: i64 = tabs::table.count().get_result(conn)?;
    if tab_count == 0 {
        return Ok(None);
    }

    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    let workspace = create(conn, "Default", Path::new(&home))?;

    diesel::update(tabs::table)
        .set(tabs::folder_workspace_id.eq(workspace.id))
        .execute(conn)?;

    Ok(Some(workspace))
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel_migrations::MigrationHarness;

    fn setup_conn() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        conn.run_pending_migrations(persistence::MIGRATIONS).unwrap();
        conn
    }

    fn insert_test_tabs(conn: &mut SqliteConnection, n: usize) {
        diesel::sql_query(
            "INSERT INTO windows (active_tab_index, quake_mode, fullscreen_state) VALUES (0, 0, 0)",
        )
        .execute(conn)
        .unwrap();
        for _ in 0..n {
            diesel::sql_query("INSERT INTO tabs (window_id) VALUES (1)")
                .execute(conn)
                .unwrap();
        }
    }

    #[test]
    fn create_and_get_by_id() {
        let mut conn = setup_conn();
        let fw = create(&mut conn, "test", Path::new("/tmp/test")).unwrap();
        assert_eq!(fw.name, "test");
        assert_eq!(fw.path, "/tmp/test");
        assert_eq!(fw.display_order, 0);
        assert!(!fw.collapsed);

        let same = get_by_id(&mut conn, fw.id).unwrap();
        assert_eq!(same.id, fw.id);
        assert_eq!(same.name, "test");
    }

    #[test]
    fn create_assigns_incrementing_display_order() {
        let mut conn = setup_conn();
        let a = create(&mut conn, "a", Path::new("/tmp/a")).unwrap();
        let b = create(&mut conn, "b", Path::new("/tmp/b")).unwrap();
        let c = create(&mut conn, "c", Path::new("/tmp/c")).unwrap();
        assert_eq!(a.display_order, 0);
        assert_eq!(b.display_order, 1);
        assert_eq!(c.display_order, 2);
    }

    #[test]
    fn get_all_returns_in_display_order() {
        let mut conn = setup_conn();
        create(&mut conn, "a", Path::new("/tmp/a")).unwrap();
        create(&mut conn, "b", Path::new("/tmp/b")).unwrap();
        create(&mut conn, "c", Path::new("/tmp/c")).unwrap();

        let all = get_all(&mut conn).unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].name, "a");
        assert_eq!(all[1].name, "b");
        assert_eq!(all[2].name, "c");
    }

    #[test]
    fn delete_removes_row() {
        let mut conn = setup_conn();
        let fw = create(&mut conn, "doomed", Path::new("/tmp/doomed")).unwrap();
        let n = delete(&mut conn, fw.id).unwrap();
        assert_eq!(n, 1);
        assert!(get_by_id(&mut conn, fw.id).is_err());
    }

    #[test]
    fn update_collapsed_toggles() {
        let mut conn = setup_conn();
        let fw = create(&mut conn, "x", Path::new("/tmp/x")).unwrap();
        assert!(!fw.collapsed);

        update_collapsed(&mut conn, fw.id, true).unwrap();
        let after = get_by_id(&mut conn, fw.id).unwrap();
        assert!(after.collapsed);

        update_collapsed(&mut conn, fw.id, false).unwrap();
        let after_off = get_by_id(&mut conn, fw.id).unwrap();
        assert!(!after_off.collapsed);
    }

    #[test]
    fn bootstrap_with_no_tabs_returns_none() {
        let mut conn = setup_conn();
        let result = bootstrap_default_workspace_for_existing_tabs(&mut conn).unwrap();
        assert!(result.is_none());
        assert_eq!(get_all(&mut conn).unwrap().len(), 0);
    }

    #[test]
    fn bootstrap_with_tabs_creates_default_workspace_and_assigns_tabs() {
        let mut conn = setup_conn();
        insert_test_tabs(&mut conn, 3);

        let result = bootstrap_default_workspace_for_existing_tabs(&mut conn).unwrap();
        let workspace = result.expect("expected Default workspace to be created");
        assert_eq!(workspace.name, "Default");

        let assigned: i64 = tabs::table
            .filter(tabs::folder_workspace_id.eq(workspace.id))
            .count()
            .get_result(&mut conn)
            .unwrap();
        assert_eq!(assigned, 3);
    }

    #[test]
    fn bootstrap_is_idempotent_on_repeated_calls() {
        let mut conn = setup_conn();
        insert_test_tabs(&mut conn, 2);

        let first = bootstrap_default_workspace_for_existing_tabs(&mut conn).unwrap();
        assert!(first.is_some());

        let second = bootstrap_default_workspace_for_existing_tabs(&mut conn).unwrap();
        assert!(second.is_none());

        assert_eq!(get_all(&mut conn).unwrap().len(), 1);
    }
}
