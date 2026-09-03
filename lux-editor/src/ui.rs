use crate::app::ShellView;
use crate::config::Config;
use crate::events::CustomEvent;
use eframe::egui;
use kit::shell::{status_bar, StatusBarData, StatusBarSection};
use kit::title_bar::{TitleBarData, TitleBarMessage};

mod configuration;
mod editor;
mod highlight;
pub mod kit;
mod shell;
pub mod theme;
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
        shell_view,
        sidebar_visible,
        reveal_active_in_tree,
        carets,
        selection_ranges,
        active_caret_index,
        caret_visible,
        document_dirty,
    } = state;

    let mut events = Vec::new();

    let active_nav = match shell_view {
        ShellView::Editor => 0,
        ShellView::Configuration => 1,
    };
    let title_messages = kit::title_bar::title_bar(
        ctx,
        TitleBarData {
            app_title: "Lux",
            nav_tabs: &["Editor", "Configuration"],
            active_nav,
        },
    );
    for message in title_messages {
        match message {
            TitleBarMessage::Menu(menu) => events.push(CustomEvent::TitleBarMenu(menu)),
            TitleBarMessage::Navigation(index) => {
                let custom = match index {
                    0 => CustomEvent::SwitchToEditor,
                    _ => CustomEvent::SwitchToConfiguration,
                };
                events.push(custom);
            }
        }
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
    let right_label = match shell_view {
        ShellView::Editor => buffer
            .path()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "Untitled".to_string()),
        ShellView::Configuration => Config::user_settings_path().display().to_string(),
    };
    status_bar(
        ctx,
        StatusBarData {
            mode_label: match shell_view {
                ShellView::Editor => "EDITOR",
                ShellView::Configuration => "CONFIGURATION",
            },
            section,
            right_label: &right_label,
        },
    );

    if shell_view == ShellView::Editor
        && sidebar_visible
        && let Some(tree) = file_tree
    {
        shell::draw_file_tree_panel(
            ctx,
            tree,
            workspace_path.map(|path| path.as_path()),
            buffer.path().map(|path| path.as_path()),
            reveal_active_in_tree,
            &mut events,
        );
    }

    let central_fill = if shell_view == ShellView::Editor {
        crate::ui::highlight::snapshot_color(
            highlight_snapshot.background,
            ctx.style().visuals.code_bg_color,
        )
    } else {
        ctx.style().visuals.panel_fill
    };
    egui::CentralPanel::default()
        .frame(
            egui::Frame::central_panel(&ctx.style())
                .fill(central_fill)
                .inner_margin(0),
        )
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
