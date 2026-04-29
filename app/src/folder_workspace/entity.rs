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
}
