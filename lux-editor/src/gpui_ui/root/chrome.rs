//! What the window draws around the editor: the sidebar file tree, the tab
//! strip, the body of the active tab, the status bar, and the OS window title.

use super::workspace::Row;
use super::Root;
use crate::gpui_ui::language::language_for;
use crate::gpui_ui::markdown;
use crate::gpui_ui::tabs::TabContent;
use crate::gpui_ui::theme::Palette;
use crate::gpui_ui::tree_icon::{ICON_FONT_FAMILY, file_type_icon, folder_glyph};
use crate::gpui_ui::views::{self, Notice};
use crate::theme::{ThemeChoice, resolve};
use gpui_kit::component::h_flex;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

impl Root {
    pub(super) fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_visible = !self.sidebar_visible;
        cx.notify();
    }

    /// Reflect the active tab (name + dirty marker) in the OS window title.
    pub(super) fn sync_window_title(&mut self, window: &mut Window) {
        let title = match self.state.tabs.active_tab() {
            Some(tab) => {
                let marker = if tab.dirty() { " •" } else { "" };
                format!("{}{marker} — Lux", tab.title())
            }
            None => "Lux".to_string(),
        };
        if self.last_title.as_deref() != Some(title.as_str()) {
            window.set_window_title(&title);
            self.last_title = Some(title);
        }
    }

    /// The active tab name shown next to the menu in the title bar.
    pub(super) fn window_caption(&self) -> String {
        match self.state.tabs.active_tab() {
            Some(tab) => tab
                .path()
                .and_then(|path| path.file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| tab.title()),
            None => "Lux".to_string(),
        }
    }

    pub(super) fn render_sidebar(
        &mut self,
        palette: &Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mut sidebar = div()
            .flex()
            .flex_col()
            .w(px(240.0))
            .h_full()
            .flex_shrink_0()
            .bg(palette.panel_bg)
            .border_r_1()
            .border_color(palette.border)
            .overflow_hidden()
            .py_1();
        let dark = resolve(
            ThemeChoice::from_value(&self.state.config.settings.theme.choice),
            Some(self.system_is_light),
        ) == ThemeChoice::Dark;
        for (row_index, row) in self.sidebar_rows().into_iter().enumerate() {
            let Row {
                path,
                name,
                depth,
                is_dir,
                ignored,
            } = row;
            let color = if ignored { palette.weak } else { palette.text };
            let (marker, glyph, glyph_color) = if is_dir {
                let open = self.state.tree.expanded.contains(&path);
                (
                    if open { "▾" } else { "▸" },
                    folder_glyph(open),
                    color,
                )
            } else {
                let (glyph, icon_color) = file_type_icon(&path, dark);
                // A gitignored row dims its glyph along with the label.
                ("", glyph, if ignored { palette.weak } else { icon_color })
            };
            let entry = div()
                .id(("sidebar-row", row_index))
                .flex()
                .items_center()
                .gap_1()
                .h(px(22.0))
                .flex_shrink_0()
                .pl(px(8.0 + depth as f32 * 12.0))
                .pr_2()
                .text_sm()
                .text_color(color)
                .cursor_pointer()
                .hover(|style| style.bg(palette.hover))
                .on_click(cx.listener(move |root, _event, window, cx| {
                    if is_dir {
                        root.toggle_dir(path.clone(), cx);
                    } else {
                        root.open_file(path.clone(), window, cx);
                    }
                }));
            // A file keeps an empty marker slot, so every name stays in one
            // column whether or not its row can expand.
            sidebar = sidebar.child(
                entry
                    .child(
                        div()
                            .w(px(12.0))
                            .flex_shrink_0()
                            .text_center()
                            .child(marker),
                    )
                    .child(
                        div()
                            .w(px(14.0))
                            .flex_shrink_0()
                            .text_center()
                            .font_family(ICON_FONT_FAMILY)
                            .text_color(glyph_color)
                            .child(glyph.to_string()),
                    )
                    .child(name),
            );
        }
        sidebar
    }

    pub(super) fn render_tabs(&self, palette: &Palette, cx: &Context<Self>) -> impl IntoElement {
        let mut strip = div()
            .flex()
            .flex_row()
            .h(px(32.0))
            .flex_shrink_0()
            .bg(palette.panel_bg)
            .border_b_1()
            .border_color(palette.border);
        for (index, tab) in self.state.tabs.tabs.iter().enumerate() {
            let mut label = tab.title();
            if tab.dirty() {
                label.push_str(" •");
            }
            let active = index == self.state.tabs.active;
            strip = strip.child(
                div()
                    .id(("tab", index))
                    .flex()
                    .items_center()
                    .gap_2()
                    .h_full()
                    .px_3()
                    .text_sm()
                    .cursor_pointer()
                    .border_r_1()
                    .border_color(palette.border)
                    .when(active, |item| {
                        item.bg(palette.window_bg)
                            .text_color(palette.text)
                            .border_b_2()
                            .border_color(palette.accent)
                    })
                    .when(!active, |item| item.text_color(palette.weak))
                    .on_click(cx.listener(move |root, _event, window, cx| {
                        root.activate(index, window, cx)
                    }))
                    .child(label)
                    .child(
                        div()
                            .id(("tab-close", index))
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(16.0))
                            .h(px(16.0))
                            .rounded_sm()
                            .text_xs()
                            .hover(|style| style.bg(palette.active))
                            .on_click(cx.listener(move |root, _event, window, cx| {
                                cx.stop_propagation();
                                root.close_tab(index, window, cx);
                            }))
                            .child("×"),
                    ),
            );
        }
        strip
    }

    /// The body under the tab strip: whichever view the active tab holds.
    pub(super) fn render_editor_area(&self, palette: &Palette, cx: &App) -> AnyElement {
        let Some(tab) = self.state.tabs.active_tab() else {
            return div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(palette.weak)
                .child("No open files")
                .into_any_element();
        };

        match &tab.content {
            TabContent::Text(doc) => {
                let path = doc.path.clone();
                if doc.preview && path.as_deref().is_some_and(markdown::is_markdown) {
                    return markdown::preview(tab.id, &doc.text(cx));
                }
                div()
                    .flex_1()
                    .min_h_0()
                    .child(doc.view().size_full())
                    .into_any_element()
            }
            TabContent::Image(path) => views::image(path),
            TabContent::Notice { path, kind } => views::notice(path, *kind, cx),
            TabContent::Settings => div()
                .flex_1()
                .min_h_0()
                .child(self.settings_view.clone())
                .into_any_element(),
        }
    }

    pub(super) fn render_status_bar(&self, palette: &Palette, cx: &App) -> impl IntoElement {
        let (caret, language, dirty, preview) = match self.state.tabs.active_tab() {
            Some(tab) => match &tab.content {
                TabContent::Text(doc) => {
                    let (line, column) = doc.caret_line_column(cx);
                    let language = doc
                        .path
                        .as_deref()
                        .and_then(language_for)
                        .unwrap_or("Plain Text")
                        .to_string();
                    (
                        format!("Ln {line}, Col {column}"),
                        language,
                        doc.dirty,
                        doc.preview,
                    )
                }
                TabContent::Image(_) => (String::new(), "Image".to_string(), false, false),
                TabContent::Notice { kind, .. } => (
                    String::new(),
                    match kind {
                        Notice::Binary => "Binary",
                        Notice::Missing => "Unavailable",
                    }
                    .to_string(),
                    false,
                    false,
                ),
                TabContent::Settings => (String::new(), "Lux Settings".to_string(), false, false),
            },
            None => ("Ln 1, Col 1".to_string(), "Plain Text".to_string(), false, false),
        };
        let workspace = self
            .state
            .workspace
            .path
            .as_deref()
            .map(|path| path.display().to_string())
            .unwrap_or_default();

        h_flex()
            .flex_shrink_0()
            .h(px(24.0))
            .px_3()
            .gap_3()
            .text_xs()
            .bg(palette.panel_bg)
            .text_color(palette.weak)
            .border_t_1()
            .border_color(palette.border)
            .child(caret)
            .child(language)
            .when(preview, |bar| bar.child("Preview"))
            .when(dirty, |bar| bar.child("Modified"))
            .child(div().flex_1())
            .child(workspace)
    }
}
