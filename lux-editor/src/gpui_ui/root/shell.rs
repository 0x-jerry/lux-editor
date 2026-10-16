//! The shell layout and the action wiring: everything the root element is made
//! of, and which method each application action runs.

use super::Root;
use crate::gpui_ui::actions::{
    ClearRecentItems, CloseTab, FormatFile, NewFile, OpenCommandPalette, OpenFile, OpenFolder,
    OpenSettings, Quit, Save, SaveAs, ToggleMarkdownPreview, ToggleSidebar,
};
use crate::gpui_ui::theme::Palette;
use gpui_kit::component::TitleBar;
use gpui_kit::component::h_flex;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

impl Render for Root {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = Palette::get(cx);
        self.sync_window_title(window);
        self.sync_system_appearance(window, cx);
        if let Some(path) = self.pending_open.take() {
            self.open_file(path, window, cx);
        }

        let title_bar = TitleBar::new()
            .on_close_window(cx.listener(|root, _: &ClickEvent, window, cx| {
                // The title bar's close button calls `remove_window`, which does
                // not consult the platform should-close callback, so it needs
                // the same gate as Quit.
                root.dismiss_palette(cx);
                if root.confirm_all_dirty(window, cx) {
                    window.remove_window();
                }
            }))
            .child(
                h_flex()
                    .flex_1()
                    .items_center()
                    .gap_2()
                    .overflow_hidden()
                    .child(self.menu_bar.clone())
                    .child(
                        div()
                            .text_xs()
                            .text_color(palette.weak)
                            .truncate()
                            .child(self.window_caption()),
                    ),
            );

        let mut body = div().flex().flex_row().flex_1().min_h_0();
        if self.sidebar_visible {
            body = body.child(self.render_sidebar(&palette, cx));
        }
        body = body.child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .min_h_0()
                .h_full()
                .child(self.render_tabs(&palette, cx))
                .child(self.render_editor_area(&palette, cx)),
        );

        let palette_open = self.palette_open;
        let palette_view = self.palette.clone();

        div()
            .id("lux-root")
            .track_focus(&self.focus_handle)
            .key_context("Lux")
            .size_full()
            .flex()
            .flex_col()
            .bg(palette.window_bg)
            .text_color(palette.text)
            .on_action(cx.listener(|root, _: &NewFile, window, cx| {
                root.dismiss_palette(cx);
                root.new_file(window, cx)
            }))
            .on_action(cx.listener(|root, _: &OpenFile, window, cx| {
                root.dismiss_palette(cx);
                root.open_file_dialog(window, cx)
            }))
            .on_action(cx.listener(|root, _: &OpenFolder, window, cx| {
                root.dismiss_palette(cx);
                root.open_folder_dialog(window, cx)
            }))
            .on_action(cx.listener(|root, _: &Save, window, cx| {
                root.dismiss_palette(cx);
                root.save(window, cx)
            }))
            .on_action(cx.listener(|root, _: &SaveAs, window, cx| {
                root.dismiss_palette(cx);
                root.save_as(window, cx)
            }))
            .on_action(cx.listener(|root, _: &FormatFile, window, cx| {
                root.dismiss_palette(cx);
                root.format_active(window, cx)
            }))
            .on_action(cx.listener(|root, _: &CloseTab, window, cx| {
                root.dismiss_palette(cx);
                root.close_active_tab(window, cx)
            }))
            .on_action(cx.listener(|root, _: &ToggleSidebar, _window, cx| {
                root.dismiss_palette(cx);
                root.toggle_sidebar(cx)
            }))
            .on_action(cx.listener(|root, _: &OpenCommandPalette, _window, cx| {
                root.open_palette(cx)
            }))
            .on_action(cx.listener(|root, _: &ToggleMarkdownPreview, _window, cx| {
                root.dismiss_palette(cx);
                root.toggle_preview(cx)
            }))
            .on_action(cx.listener(|root, _: &OpenSettings, window, cx| {
                root.dismiss_palette(cx);
                root.open_settings(window, cx)
            }))
            .on_action(cx.listener(|root, _: &ClearRecentItems, _window, cx| {
                root.dismiss_palette(cx);
                root.state.config.clear_recent_items();
                cx.notify();
            }))
            .on_action(cx.listener(|root, _: &Quit, window, cx| {
                root.dismiss_palette(cx);
                if root.confirm_all_dirty(window, cx) {
                    cx.quit();
                }
            }))
            .child(title_bar)
            .child(body)
            .child(self.render_status_bar(&palette, cx))
            .when(palette_open, move |root| root.child(palette_view))
    }
}
