//! Sidebar folder workspaces — group tabs by file system folder (cmux-style).
//!
//! Gated behind `FeatureFlag::FolderWorkspacesEnabled`. See
//! `specs/sidebar-folder-workspaces/{PRODUCT,TECH,TASKS}.md`.

pub mod entity;
pub mod manager;
pub mod model;
pub mod view;

pub use entity::FolderWorkspaceModel;
pub use model::FolderWorkspace;
