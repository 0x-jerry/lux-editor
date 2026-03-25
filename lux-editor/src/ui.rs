mod configuration;
mod editor;
mod highlight;
mod shell;
mod types;

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
        shell_view,
    } = state;

    let mut events = Vec::new();

    shell::draw_shell_navigation(ctx, shell_view, &mut events);

    if shell_view == ShellView::Editor
        && let Some(tree) = file_tree
    {
        shell::draw_file_tree_panel(ctx, tree, &mut events);
    }

    egui::CentralPanel::default().show(ctx, |ui| {
        if shell_view == ShellView::Editor {
            editor::render_editor_view(
                ui,
                workspace_path,
                buffer,
                highlight_snapshot,
                editor_config,
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
