//! WarpUI render component for folder workspace headers.
//!
//! Renders a 2-line header: line 1 has the disclosure arrow, folder icon,
//! workspace name, and (only when collapsed) a fixed-width pill badge with
//! the tab count; line 2 shows the folder path in a secondary color.
//! Both lines have soft_wrap disabled and ClipConfig::ellipsis so width
//! pressure becomes "..." rather than line wrap.
//! The host (vertical_tabs.rs) is responsible for the click handler that
//! toggles collapse and the hover icon button row.

use std::borrow::Cow;

use warpui::elements::{
    Container, CornerRadius, CrossAxisAlignment, Element, Fill, Flex, MainAxisSize, Padding,
    ParentElement, Radius, Shrinkable, Text,
};
use warpui::text_layout::ClipConfig;
use warpui::ui_components::components::{UiComponent, UiComponentStyles};

use pathfinder_color::ColorU;

const HEADER_TITLE_FONT_SIZE: f32 = 14.0;
const HEADER_PATH_FONT_SIZE: f32 = 11.0;
const TAB_BADGE_FONT_SIZE: f32 = 11.0;

#[derive(Debug, Clone, Default)]
pub struct FolderWorkspaceHeader {
    name: String,
    path: String,
    tab_count: usize,
    collapsed: bool,
    folder_missing: bool,
    title_color: Option<ColorU>,
    path_color: Option<ColorU>,
    badge_text_color: Option<ColorU>,
    badge_background: Option<Fill>,
    styles: UiComponentStyles,
}

impl FolderWorkspaceHeader {
    pub fn new(name: String) -> Self {
        Self {
            name,
            path: String::new(),
            tab_count: 0,
            collapsed: false,
            folder_missing: false,
            title_color: None,
            path_color: None,
            badge_text_color: None,
            badge_background: None,
            styles: UiComponentStyles::default(),
        }
    }

    pub fn with_path(mut self, path: String) -> Self {
        self.path = path;
        self
    }

    pub fn with_tab_count(mut self, count: usize) -> Self {
        self.tab_count = count;
        self
    }

    pub fn with_collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    pub fn with_folder_missing(mut self, missing: bool) -> Self {
        self.folder_missing = missing;
        self
    }

    pub fn with_title_color(mut self, color: ColorU) -> Self {
        self.title_color = Some(color);
        self
    }

    pub fn with_path_color(mut self, color: ColorU) -> Self {
        self.path_color = Some(color);
        self
    }

    pub fn with_badge_text_color(mut self, color: ColorU) -> Self {
        self.badge_text_color = Some(color);
        self
    }

    pub fn with_badge_background(mut self, fill: Fill) -> Self {
        self.badge_background = Some(fill);
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

        let arrow = if self.collapsed { "▾" } else { "▾" };
        let arrow = if self.collapsed { "▸" } else { arrow };
        let warn = if self.folder_missing { " ⚠" } else { "" };
        let title_label = format!("{} 📁 {}{}", arrow, self.name, warn);

        let mut title_text =
            Text::new(Cow::Owned(title_label), font_family, HEADER_TITLE_FONT_SIZE)
                .soft_wrap(false)
                .with_clip(ClipConfig::ellipsis());
        if let Some(color) = self.title_color {
            title_text = title_text.with_color(color);
        }

        let mut title_row = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Center);
        title_row.add_child(Shrinkable::new(1., title_text.finish()).finish());

        if self.collapsed && self.tab_count > 0 {
            let label = if self.tab_count > 99 {
                Cow::Borrowed("99+")
            } else {
                Cow::Owned(self.tab_count.to_string())
            };
            let badge_color = self
                .badge_text_color
                .or(self.path_color)
                .unwrap_or(ColorU::white());
            let badge_text = Text::new(label, font_family, TAB_BADGE_FONT_SIZE)
                .soft_wrap(false)
                .with_color(badge_color)
                .finish();
            let mut badge = Container::new(badge_text)
                .with_padding(Padding::uniform(1.).with_left(6.).with_right(6.))
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(8.)));
            if let Some(bg) = self.badge_background {
                badge = badge.with_background(bg);
            }
            title_row.add_child(Container::new(badge.finish()).with_margin_left(8.).finish());
        }

        let mut col = Flex::column()
            .with_main_axis_size(MainAxisSize::Min)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(2.);

        col.add_child(title_row.finish());

        if !self.path.is_empty() {
            let mut path_text =
                Text::new(Cow::Owned(self.path), font_family, HEADER_PATH_FONT_SIZE)
                    .soft_wrap(false)
                    .with_clip(ClipConfig::ellipsis());
            if let Some(color) = self.path_color {
                path_text = path_text.with_color(color);
            }
            col.add_child(path_text.finish());
        }

        Container::new(col.finish()).with_padding(Padding::uniform(8.))
    }

    fn with_style(self, styles: UiComponentStyles) -> Self {
        Self {
            styles: self.styles.merge(styles),
            ..self
        }
    }
}
