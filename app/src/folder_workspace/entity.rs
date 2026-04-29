//! In-memory `SingletonEntity` for folder workspaces.
//!
//! Render path reads via [`warpui::AppContext::get_singleton_model_handle`];
//! mutations route through `model_event_sender` (wired in T9). Loaded once at
//! app init from [`PersistedData::folder_workspaces`](crate::persistence::PersistedData).
//! Pattern: [`crate::projects::ProjectManagementModel`].

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
        Self {
            workspaces: persisted,
            model_event_sender,
        }
    }

    pub fn all(&self) -> &[FolderWorkspace] {
        &self.workspaces
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
}
