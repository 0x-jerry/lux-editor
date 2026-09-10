//! Root view component: renders the shell plus the overlay windows (command
//! palette, about dialog) and reports every effect they requested as events.

use super::about::AboutWindow;
use super::close_prompt::ClosePrompt;
use super::command_panel::{CommandPanel, CommandPanelInput, PaletteContext};
use super::frame_input::{CaretInput, DocumentInput, SidebarInput, TabsInput, WorkspaceInput};
use super::shell::{Shell, ShellInput};
use crate::component::Component;
use crate::events::CustomEvent;
use crate::settings::Config;
use eframe::egui;

/// Everything the root view needs this frame: the components it renders
/// (mutable, borrowed from the app) plus the document/workspace snapshot.
pub struct AppViewInput<'a> {
    pub shell: &'a mut Shell,
    pub command_panel: &'a mut CommandPanel,
    pub about_window: &'a mut AboutWindow,
    pub close_prompt: &'a mut ClosePrompt,
    pub sidebar: SidebarInput<'a>,
    pub workspace: WorkspaceInput<'a>,
    pub tabs: TabsInput<'a>,
    pub document: DocumentInput<'a>,
    pub caret: CaretInput<'a>,
    pub editor_config: &'a Config,
}

/// The app's view root. Owns no state itself; the shell and overlay
/// components keep theirs and are borrowed in per frame.
pub struct AppView;

impl Component for AppView {
    type Message = CustomEvent;
    type Input<'a> = AppViewInput<'a>;

    fn render(&mut self, ui: &mut egui::Ui, input: Self::Input<'_>) -> Vec<CustomEvent> {
        let AppViewInput {
            shell,
            command_panel,
            about_window,
            close_prompt,
            sidebar,
            workspace,
            tabs,
            document,
            caret,
            editor_config,
        } = input;

        let active_is_markdown = tabs.active_is_markdown;
        let tab_metas = tabs.tabs;
        let mut events = shell.render(
            ui,
            ShellInput {
                sidebar,
                workspace,
                tabs,
                document,
                caret,
                editor_config,
            },
        );

        // Overlays render after the main layout so they stay on top.
        events.extend(command_panel.render(
            ui,
            CommandPanelInput {
                config: editor_config,
                context: PaletteContext { active_is_markdown },
            },
        ));
        about_window.render(ui, ());
        events.extend(close_prompt.render(ui, tab_metas));

        events
    }
}
