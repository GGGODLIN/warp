//! In-memory `SingletonEntity` for folder workspaces.
//!
//! Render path reads via [`warpui::AppContext::get_singleton_model_handle`];
//! mutations route through `model_event_sender` (wired in T9). Loaded once at
//! app init from [`PersistedData::folder_workspaces`](crate::persistence::PersistedData).
//! Pattern: [`crate::projects::ProjectManagementModel`].

use std::collections::HashSet;
use std::sync::mpsc::SyncSender;

use warpui::{Entity, ModelContext, SingletonEntity};

use crate::persistence::ModelEvent;

use super::FolderWorkspace;

#[derive(Debug)]
pub enum FolderWorkspaceEvent {
    Created { id: i32 },
    Updated { id: i32 },
    Deleted { id: i32 },
}

pub struct FolderWorkspaceModel {
    workspaces: Vec<FolderWorkspace>,
    #[expect(unused, reason = "T9 will wire DB writes via ModelEvent variants")]
    model_event_sender: Option<SyncSender<ModelEvent>>,
    last_active_id: Option<i32>,
    toasted_missing_session: HashSet<i32>,
}

impl Entity for FolderWorkspaceModel {
    type Event = FolderWorkspaceEvent;
}

impl SingletonEntity for FolderWorkspaceModel {}

impl FolderWorkspaceModel {
    pub fn new(
        persisted: Vec<FolderWorkspace>,
        model_event_sender: Option<SyncSender<ModelEvent>>,
        _ctx: &mut ModelContext<Self>,
    ) -> Self {
        log::debug!(
            "Loading {} persisted folder workspaces",
            persisted.len()
        );
        let last_active_id = persisted.first().map(|w| w.id);
        Self {
            workspaces: persisted,
            model_event_sender,
            last_active_id,
            toasted_missing_session: HashSet::new(),
        }
    }

    pub fn was_missing_toast_shown(&self, id: i32) -> bool {
        self.toasted_missing_session.contains(&id)
    }

    pub fn record_missing_toast_shown(&mut self, id: i32) {
        self.toasted_missing_session.insert(id);
    }

    pub fn all(&self) -> &[FolderWorkspace] {
        &self.workspaces
    }

    pub fn last_active_id(&self) -> Option<i32> {
        self.last_active_id
            .filter(|id| self.workspaces.iter().any(|w| w.id == *id))
            .or_else(|| self.workspaces.first().map(|w| w.id))
    }

    pub fn set_last_active(&mut self, id: i32) {
        if self.workspaces.iter().any(|w| w.id == id) {
            self.last_active_id = Some(id);
        }
    }

    /// Create a new folder workspace, persist via a fresh RW connection
    /// (spike-pragmatic; see `establish_rw_connection` doc), update in-memory
    /// state, emit `Created`. Logs and returns Err on DB failure without
    /// mutating in-memory state, so the model stays consistent with persistence.
    pub fn create_workspace(
        &mut self,
        name: &str,
        path: &std::path::Path,
        ctx: &mut ModelContext<Self>,
    ) -> anyhow::Result<i32> {
        let database_path = crate::persistence::database_file_path();
        let mut conn =
            crate::persistence::establish_rw_connection(&database_path.to_string_lossy())?;
        let workspace = super::manager::create(&mut conn, name, path)?;
        let id = workspace.id;
        self.workspaces.push(workspace);
        ctx.emit(FolderWorkspaceEvent::Created { id });
        Ok(id)
    }

    /// Toggle the collapsed state of a workspace by id, persisting the new
    /// value and emitting `Updated`. No-op if the id is unknown.
    pub fn toggle_collapsed(
        &mut self,
        id: i32,
        ctx: &mut ModelContext<Self>,
    ) -> anyhow::Result<()> {
        let Some(idx) = self.workspaces.iter().position(|w| w.id == id) else {
            return Ok(());
        };
        let new_value = !self.workspaces[idx].collapsed;
        let database_path = crate::persistence::database_file_path();
        let mut conn =
            crate::persistence::establish_rw_connection(&database_path.to_string_lossy())?;
        super::manager::update_collapsed(&mut conn, id, new_value)?;
        self.workspaces[idx].collapsed = new_value;
        ctx.emit(FolderWorkspaceEvent::Updated { id });
        Ok(())
    }

    pub fn rename_workspace(
        &mut self,
        id: i32,
        new_name: &str,
        ctx: &mut ModelContext<Self>,
    ) -> anyhow::Result<()> {
        let Some(idx) = self.workspaces.iter().position(|w| w.id == id) else {
            return Ok(());
        };
        let database_path = crate::persistence::database_file_path();
        let mut conn =
            crate::persistence::establish_rw_connection(&database_path.to_string_lossy())?;
        super::manager::rename(&mut conn, id, new_name)?;
        self.workspaces[idx].name = new_name.to_string();
        ctx.emit(FolderWorkspaceEvent::Updated { id });
        Ok(())
    }

    /// Swap display_order with the neighbor in the requested direction.
    /// `delta = -1` moves up (towards index 0), `+1` moves down.
    pub fn move_workspace(
        &mut self,
        id: i32,
        delta: i32,
        ctx: &mut ModelContext<Self>,
    ) -> anyhow::Result<()> {
        let Some(idx) = self.workspaces.iter().position(|w| w.id == id) else {
            return Ok(());
        };
        let neighbor_idx = idx as i32 + delta;
        if neighbor_idx < 0 || neighbor_idx as usize >= self.workspaces.len() {
            return Ok(());
        }
        let neighbor_idx = neighbor_idx as usize;
        let database_path = crate::persistence::database_file_path();
        let mut conn =
            crate::persistence::establish_rw_connection(&database_path.to_string_lossy())?;
        let a = self.workspaces[idx].display_order;
        let b = self.workspaces[neighbor_idx].display_order;
        super::manager::set_display_order(&mut conn, self.workspaces[idx].id, b)?;
        super::manager::set_display_order(&mut conn, self.workspaces[neighbor_idx].id, a)?;
        self.workspaces[idx].display_order = b;
        self.workspaces[neighbor_idx].display_order = a;
        self.workspaces.sort_by_key(|w| w.display_order);
        let neighbor_id = self.workspaces.iter().find(|w| w.display_order == a).map(|w| w.id);
        ctx.emit(FolderWorkspaceEvent::Updated { id });
        if let Some(nid) = neighbor_id {
            ctx.emit(FolderWorkspaceEvent::Updated { id: nid });
        }
        Ok(())
    }

    pub fn delete_workspace(
        &mut self,
        id: i32,
        ctx: &mut ModelContext<Self>,
    ) -> anyhow::Result<()> {
        let Some(idx) = self.workspaces.iter().position(|w| w.id == id) else {
            return Ok(());
        };
        let fallback_id = self
            .workspaces
            .iter()
            .find(|w| w.id != id)
            .map(|w| w.id);
        let database_path = crate::persistence::database_file_path();
        let mut conn =
            crate::persistence::establish_rw_connection(&database_path.to_string_lossy())?;
        super::manager::delete_with_tab_reassignment(&mut conn, id, fallback_id)?;
        self.workspaces.remove(idx);
        ctx.emit(FolderWorkspaceEvent::Deleted { id });
        Ok(())
    }
}
