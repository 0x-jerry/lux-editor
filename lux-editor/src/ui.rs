mod configuration;
mod editor;
mod highlight;
mod shell;
mod types;
mod welcome;

use crate::app::ShellView;
use crate::events::CustomEvent;

pub use types::DrawUiState;

pub fn draw_ui(ctx: &egui::Context, state: DrawUiState<'_>) -> Vec<CustomEvent> {
    let DrawUiState {
        file_tree,
        workspace_path,
        buffer,
        highlight_snapshot,
        editor_config,
        config_draft,
        config_status,
        document_status,
        shell_view,
        reveal_active_in_tree,
        caret_line,
        caret_column,
        selection_len,
        caret_visible,
        document_dirty,
    } = state;

    let mut events = Vec::new();

    shell::draw_shell_navigation(ctx, shell_view, &mut events);
    shell::draw_status_bar(
        ctx,
        shell_view,
        caret_line,
        caret_column,
        selection_len,
        document_dirty,
        document_status,
        config_status,
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

    egui::CentralPanel::default().show(ctx, |ui| {
        if shell_view == ShellView::Editor {
            editor::render_editor_view(
                ui,
                editor::EditorViewState {
                    workspace_path,
                    buffer,
                    highlight_snapshot,
                    editor_config,
                    caret_line,
                    caret_column,
                    selection_len,
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

    events
}
