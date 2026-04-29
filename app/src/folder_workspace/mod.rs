//! Sidebar folder workspaces — group tabs by file system folder (cmux-style).
//!
//! Gated behind `FeatureFlag::FolderWorkspacesEnabled`. See
//! `specs/sidebar-folder-workspaces/{PRODUCT,TECH,TASKS}.md`.
//
// Intentional WIP allow: items become "live" once T8 wires the manager +
// view into vertical_tabs.rs sidebar render. Remove this attribute then.
#![allow(dead_code)]

pub mod entity;
pub mod manager;
pub mod model;
pub mod view;

pub use entity::FolderWorkspaceModel;
pub use model::FolderWorkspace;
