use eframe::egui;

pub(crate) enum EditorCommand {
    InsertText(String),
    Paste(String),
    InsertNewline,
    InsertTab,
    Backspace,
    Delete,
    DeleteWordBackward,
    DeleteWordForward,
    MoveLeft { selecting: bool },
    MoveRight { selecting: bool },
    MoveWordLeft { selecting: bool },
    MoveWordRight { selecting: bool },
    MoveUp { selecting: bool },
    MoveDown { selecting: bool },
    MoveHome { selecting: bool },
    MoveEnd { selecting: bool },
    SelectAll,
    Copy,
    Cut,
    Undo,
    Redo,
    Save,
    ToggleCommandPanel,
    CollapseCarets,
    AddCursorBelow,
    AddCursorAbove,
}

pub(crate) fn commands_from_event(event: egui::Event) -> Vec<EditorCommand> {
    match event {
        egui::Event::Text(text) => {
            if text.starts_with(|c: char| c.is_ascii_control()) {
                Vec::new()
            } else {
                vec![EditorCommand::InsertText(text)]
            }
        }
        egui::Event::Paste(text) => vec![EditorCommand::Paste(text)],
        egui::Event::Copy => vec![EditorCommand::Copy],
        egui::Event::Cut => vec![EditorCommand::Cut],
        egui::Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } => {
            let shortcut = modifiers.command || modifiers.ctrl;
            let selecting = modifiers.shift;
            if shortcut && modifiers.alt {
                return match key {
                    egui::Key::ArrowDown => vec![EditorCommand::AddCursorBelow],
                    egui::Key::ArrowUp => vec![EditorCommand::AddCursorAbove],
                    _ => Vec::new(),
                };
            }
            if shortcut && !modifiers.alt {
                return match key {
                    egui::Key::A => vec![EditorCommand::SelectAll],
                    egui::Key::C => vec![EditorCommand::Copy],
                    egui::Key::K => vec![EditorCommand::ToggleCommandPanel],
                    egui::Key::X => vec![EditorCommand::Cut],
                    egui::Key::S => vec![EditorCommand::Save],
                    egui::Key::Z if modifiers.shift => vec![EditorCommand::Redo],
                    egui::Key::Z => vec![EditorCommand::Undo],
                    egui::Key::Y => vec![EditorCommand::Redo],
                    _ => Vec::new(),
                };
            }
            if modifiers.alt {
                return match key {
                    egui::Key::ArrowLeft => vec![EditorCommand::MoveWordLeft { selecting }],
                    egui::Key::ArrowRight => vec![EditorCommand::MoveWordRight { selecting }],
                    egui::Key::Backspace => vec![EditorCommand::DeleteWordBackward],
                    egui::Key::Delete => vec![EditorCommand::DeleteWordForward],
                    _ => Vec::new(),
                };
            }
            match key {
                egui::Key::Enter => vec![EditorCommand::InsertNewline],
                egui::Key::Tab => vec![EditorCommand::InsertTab],
                egui::Key::Backspace => vec![EditorCommand::Backspace],
                egui::Key::Delete => vec![EditorCommand::Delete],
                egui::Key::Escape => vec![EditorCommand::CollapseCarets],
                egui::Key::ArrowLeft => vec![EditorCommand::MoveLeft { selecting }],
                egui::Key::ArrowRight => vec![EditorCommand::MoveRight { selecting }],
                egui::Key::ArrowUp => vec![EditorCommand::MoveUp { selecting }],
                egui::Key::ArrowDown => vec![EditorCommand::MoveDown { selecting }],
                egui::Key::Home => vec![EditorCommand::MoveHome { selecting }],
                egui::Key::End => vec![EditorCommand::MoveEnd { selecting }],
                _ => Vec::new(),
            }
        }
        _ => Vec::new(),
    }
}
