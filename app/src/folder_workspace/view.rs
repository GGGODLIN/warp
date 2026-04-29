//! WarpUI render component for folder workspace headers.
//!
//! T7 baseline: hardcoded data via `FolderWorkspaceHeader::new(name, tab_names)`,
//! styling via `UiComponentStyles` (caller supplies fonts / colors). T8 wires
//! the real [`super::manager::get_all`] result + workspace.tabs into this
//! component within the sidebar render path.
//!
//! Pattern source: [`crate::ui_components::WrappableText`](super::super::ui_components)
//! + render_tab_group in `vertical_tabs.rs:1696`.

use std::borrow::Cow;

use warpui::elements::{
    Container, CrossAxisAlignment, Element, Flex, MainAxisSize, Padding, ParentElement, Text,
};
use warpui::ui_components::components::{UiComponent, UiComponentStyles};

/// A collapsible sidebar group: workspace name + indented tab rows.
///
/// `tab_names` is plain strings for now; in T8/T10 the integration layer
/// renders real `TabData` rows and passes the workspace name from
/// [`super::FolderWorkspace`].
#[derive(Debug, Clone, Default)]
pub struct FolderWorkspaceHeader {
    name: String,
    tab_names: Vec<String>,
    collapsed: bool,
    folder_missing: bool,
    styles: UiComponentStyles,
}

impl FolderWorkspaceHeader {
    pub fn new(name: String, tab_names: Vec<String>) -> Self {
        Self {
            name,
            tab_names,
            collapsed: false,
            folder_missing: false,
            styles: UiComponentStyles::default(),
        }
    }

    pub fn with_collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    pub fn with_folder_missing(mut self, missing: bool) -> Self {
        self.folder_missing = missing;
        self
    }
}

impl UiComponent for FolderWorkspaceHeader {
    type ElementType = Container;

    fn build(self) -> Container {
        let styles = self.styles;
        let font_family = styles
            .font_family_id
            .expect("FolderWorkspaceHeader requires font_family_id (set via with_style)");
        let font_size = styles.font_size.unwrap_or_default();

        let arrow = if self.collapsed { "▸" } else { "▾" };
        let warn = if self.folder_missing { " ⚠" } else { "" };
        let header_text = Text::new(
            Cow::Owned(format!("{} {}{}", arrow, self.name, warn)),
            font_family,
            font_size,
        )
        .finish();

        let mut col = Flex::column()
            .with_main_axis_size(MainAxisSize::Min)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch);

        col.add_child(
            Container::new(header_text)
                .with_padding(Padding::uniform(4.))
                .finish(),
        );

        if !self.collapsed {
            for tab_name in self.tab_names {
                let row_text = Text::new(
                    Cow::Owned(format!("• {}", tab_name)),
                    font_family,
                    font_size,
                )
                .finish();
                col.add_child(
                    Container::new(row_text)
                        .with_padding(Padding::uniform(2.).with_left(20.))
                        .finish(),
                );
            }
        }

        Container::new(col.finish())
    }

    fn with_style(self, styles: UiComponentStyles) -> Self {
        Self {
            styles: self.styles.merge(styles),
            ..self
        }
    }
}
