use crate::app::ShellView;
use crate::events::CustomEvent;
use eframe::egui;
use kit::shell::{navigation, status_bar, NavigationTab, StatusBarData, StatusBarSection};
use kit::title_bar::{TitleBarData, TitleBarMessage};

mod configuration;
mod editor;
mod highlight;
pub mod kit;
mod shell;
mod types;
mod welcome;

pub use types::DocumentTab;
pub use types::DrawUiState;

pub fn draw_ui(ctx: &egui::Context, state: DrawUiState<'_>) -> Vec<CustomEvent> {
    let DrawUiState {
        file_tree,
        workspace_path,
        buffer,
        document_tabs,
        active_document_index,
        highlight_snapshot,
        editor_config,
        config_draft,
        config_status,
        document_status,
        document_title,
        shell_view,
        reveal_active_in_tree,
        carets,
        selection_ranges,
        active_caret_index,
        caret_visible,
        document_dirty,
    } = state;

    let mut events = Vec::new();

    let title_messages = kit::title_bar::title_bar(
        ctx,
        TitleBarData {
            app_title: "Lux",
            document_title: &document_title,
        },
    );
    for message in title_messages {
        match message {
            TitleBarMessage::Menu(menu) => events.push(CustomEvent::TitleBarMenu(menu)),
        }
    }

    let tabs = [
        NavigationTab::new("Editor"),
        NavigationTab::new("Configuration"),
    ];
    let active_tab = match shell_view {
        ShellView::Editor => 0,
        ShellView::Configuration => 1,
    };
    match navigation(ctx, &tabs, active_tab) {
        Some(0) => events.push(CustomEvent::SwitchToEditor),
        Some(1) => events.push(CustomEvent::SwitchToConfiguration),
        _ => {}
    }

    let selection_len: usize = selection_ranges
        .iter()
        .map(|range| range.end - range.start)
        .sum();
    let (caret_line, caret_column) = carets.get(active_caret_index).copied().unwrap_or((1, 1));
    let section = match shell_view {
        ShellView::Editor => StatusBarSection::Editor {
            caret_line,
            caret_column,
            selection_len,
            document_dirty,
            document_status,
        },
        ShellView::Configuration => StatusBarSection::Configuration { config_status },
    };
    status_bar(
        ctx,
        StatusBarData {
            mode_label: match shell_view {
                ShellView::Editor => "EDITOR",
                ShellView::Configuration => "CONFIGURATION",
            },
            section,
        },
    );

    if shell_view == ShellView::Editor
        && let Some(tree) = file_tree
    {
        shell::draw_file_tree_panel(
            ctx,
            tree,
            buffer.path().map(|path| path.as_path()),
            reveal_active_in_tree,
            &mut events,
        );
    }

    egui::CentralPanel::default()
        .frame(egui::Frame::central_panel(&ctx.style()).inner_margin(0))
        .show(ctx, |ui| {
        if shell_view == ShellView::Editor {
            editor::render_editor_view(
                ui,
                editor::EditorViewState {
                    workspace_path,
                    buffer,
                    document_tabs,
                    active_document_index,
                    highlight_snapshot,
                    editor_config,
                    carets: &carets,
                    selection_ranges: &selection_ranges,
                    active_caret_index,
                    caret_visible,
                },
                &mut events,
            );
        } else {
            configuration::render_configuration_view(
                ui,
                workspace_path,
                buffer,
                editor_config,
                config_draft,
                config_status,
                &mut events,
            );
        }

    });

    // Bottom-right resize grip for frameless window builds (no-op on macOS).
    egui::Area::new(egui::Id::new("window_resize_handle"))
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-3.0, -3.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            kit::title_bar::window_resize_handle(ui);
        });

    events
}
