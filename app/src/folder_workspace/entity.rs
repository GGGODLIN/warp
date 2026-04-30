//! In-memory `SingletonEntity` for folder workspaces.
//!
//! Render path reads via [`warpui::AppContext::get_singleton_model_handle`];
//! mutations route through `model_event_sender` to the SQLite writer thread,
//! matching the pattern used by `ProjectManagementModel` and other Warp
//! singletons. Loaded once at app init from
//! [`PersistedData::folder_workspaces`](crate::persistence::PersistedData).
//!
//! Tentative-id discipline: in-memory `id` for newly created workspaces is
//! `max(existing.id) + 1`. The SQLite writer thread uses that same id when
//! inserting (via raw SQL) so foreign keys in `tabs.folder_workspace_id`
//! stay valid across the async write boundary.

use std::collections::HashSet;
use std::sync::mpsc::SyncSender;

use chrono::Utc;
use warpui::{Entity, ModelContext, SingletonEntity};

use crate::persistence::ModelEvent;

use super::FolderWorkspace;

#[derive(Debug)]
#[allow(dead_code, reason = "event variants emitted for downstream subscribers")]
pub enum FolderWorkspaceEvent {
    Created { id: i32 },
    Updated { id: i32 },
    Deleted { id: i32 },
}

pub struct FolderWorkspaceModel {
    workspaces: Vec<FolderWorkspace>,
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

    fn send_event(&self, event: ModelEvent) {
        if let Some(sender) = &self.model_event_sender {
            if let Err(err) = sender.send(event) {
                log::warn!("Failed to enqueue folder workspace ModelEvent: {err}");
            }
        }
    }

    fn next_tentative_id(&self) -> i32 {
        self.workspaces.iter().map(|w| w.id).max().unwrap_or(0) + 1
    }

    fn next_display_order(&self) -> i32 {
        self.workspaces
            .iter()
            .map(|w| w.display_order)
            .max()
            .unwrap_or(-1)
            + 1
    }

    pub fn create_workspace(
        &mut self,
        name: &str,
        path: &std::path::Path,
        ctx: &mut ModelContext<Self>,
    ) -> anyhow::Result<i32> {
        let id = self.next_tentative_id();
        let display_order = self.next_display_order();
        let path_str = path.to_string_lossy().into_owned();
        self.send_event(ModelEvent::InsertFolderWorkspace {
            tentative_id: id,
            name: name.to_string(),
            path: path_str.clone(),
            display_order,
            collapsed: false,
        });
        let workspace = FolderWorkspace {
            id,
            name: name.to_string(),
            path: path_str,
            display_order,
            collapsed: false,
            created_ts: Utc::now().naive_utc(),
            default_command: None,
        };
        self.workspaces.push(workspace);
        self.last_active_id = Some(id);
        ctx.emit(FolderWorkspaceEvent::Created { id });
        Ok(id)
    }

    pub fn toggle_collapsed(
        &mut self,
        id: i32,
        ctx: &mut ModelContext<Self>,
    ) -> anyhow::Result<()> {
        let Some(idx) = self.workspaces.iter().position(|w| w.id == id) else {
            return Ok(());
        };
        let new_value = !self.workspaces[idx].collapsed;
        self.workspaces[idx].collapsed = new_value;
        self.send_event(ModelEvent::UpdateFolderWorkspaceCollapsed {
            id,
            collapsed: new_value,
        });
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
        self.workspaces[idx].name = new_name.to_string();
        self.send_event(ModelEvent::UpdateFolderWorkspaceName {
            id,
            name: new_name.to_string(),
        });
        ctx.emit(FolderWorkspaceEvent::Updated { id });
        Ok(())
    }

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
        let a = self.workspaces[idx].display_order;
        let b = self.workspaces[neighbor_idx].display_order;
        let neighbor_id = self.workspaces[neighbor_idx].id;
        self.workspaces[idx].display_order = b;
        self.workspaces[neighbor_idx].display_order = a;
        self.workspaces.sort_by_key(|w| w.display_order);
        self.send_event(ModelEvent::UpdateFolderWorkspaceDisplayOrder {
            id,
            display_order: b,
        });
        self.send_event(ModelEvent::UpdateFolderWorkspaceDisplayOrder {
            id: neighbor_id,
            display_order: a,
        });
        ctx.emit(FolderWorkspaceEvent::Updated { id });
        ctx.emit(FolderWorkspaceEvent::Updated { id: neighbor_id });
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
        self.workspaces.remove(idx);
        if self.last_active_id == Some(id) {
            self.last_active_id = fallback_id;
        }
        self.send_event(ModelEvent::DeleteFolderWorkspace { id, fallback_id });
        ctx.emit(FolderWorkspaceEvent::Deleted { id });
        Ok(())
    }
}
