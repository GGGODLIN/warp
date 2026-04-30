//! WarpUI render component for folder workspace headers.
//!
//! Renders a 2-line header: line 1 has the disclosure arrow, folder icon,
//! workspace name, and a row of small CircleFilled dots indicating tab count
//! (one dot per tab, max 9; "9+" fallback for >=10); line 2 shows the
//! folder path in a secondary color, clipped to ellipsis on overflow.
//! The host (vertical_tabs.rs) is responsible for the click handler that
//! toggles collapse and the hover icon button row.

use std::borrow::Cow;

use warpui::elements::{
    ConstrainedBox, Container, CrossAxisAlignment, Element, Flex, MainAxisSize, Padding,
    ParentElement, Shrinkable, Text,
};
use warpui::text_layout::ClipConfig;
use warpui::ui_components::components::{UiComponent, UiComponentStyles};

use pathfinder_color::ColorU;
use warp_core::ui::Icon as WarpIcon;

const HEADER_TITLE_FONT_SIZE: f32 = 14.0;
const HEADER_PATH_FONT_SIZE: f32 = 11.0;
const TAB_COUNT_DOT_SIZE: f32 = 6.0;
const TAB_COUNT_DOT_SPACING: f32 = 3.0;
const TAB_COUNT_DOT_MAX: usize = 9;

#[derive(Debug, Clone, Default)]
pub struct FolderWorkspaceHeader {
    name: String,
    path: String,
    tab_count: usize,
    collapsed: bool,
    folder_missing: bool,
    title_color: Option<ColorU>,
    path_color: Option<ColorU>,
    dot_color: Option<ColorU>,
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
            dot_color: None,
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

    pub fn with_dot_color(mut self, color: ColorU) -> Self {
        self.dot_color = Some(color);
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

        let mut title_text = Text::new(
            Cow::Owned(title_label),
            font_family,
            HEADER_TITLE_FONT_SIZE,
        )
        .soft_wrap(false)
        .with_clip(ClipConfig::ellipsis());
        if let Some(color) = self.title_color {
            title_text = title_text.with_color(color);
        }

        let mut title_row = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Center);
        title_row.add_child(Shrinkable::new(1., title_text.finish()).finish());

        if self.tab_count > 0 {
            if let Some(color) = self.dot_color.or(self.path_color) {
                let mut dots = Flex::row()
                    .with_main_axis_size(MainAxisSize::Min)
                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_spacing(TAB_COUNT_DOT_SPACING);
                let dot_n = self.tab_count.min(TAB_COUNT_DOT_MAX);
                for _ in 0..dot_n {
                    dots.add_child(
                        ConstrainedBox::new(
                            WarpIcon::CircleFilled.to_warpui_icon(color.into()).finish(),
                        )
                        .with_width(TAB_COUNT_DOT_SIZE)
                        .with_height(TAB_COUNT_DOT_SIZE)
                        .finish(),
                    );
                }
                if self.tab_count > TAB_COUNT_DOT_MAX {
                    dots.add_child(
                        Text::new(
                            Cow::Borrowed("+"),
                            font_family,
                            HEADER_PATH_FONT_SIZE,
                        )
                        .with_color(color)
                        .finish(),
                    );
                }
                title_row.add_child(
                    Container::new(dots.finish())
                        .with_margin_left(8.)
                        .finish(),
                );
            }
        }

        let mut col = Flex::column()
            .with_main_axis_size(MainAxisSize::Min)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(2.);

        col.add_child(title_row.finish());

        if !self.path.is_empty() {
            let mut path_text = Text::new(
                Cow::Owned(self.path),
                font_family,
                HEADER_PATH_FONT_SIZE,
            )
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
