//! Markdown preview renderer. Fences are laid out with the same tree-sitter
//! palette the editor uses for the injected grammar.

mod code;
mod renderer;

use code::CodeHighlighter;
use renderer::Renderer;

use crate::theme::SyntaxColors;
use eframe::egui;
use egui_commonmark_backend::{CommonMarkCache, CommonMarkOptions};
use std::sync::Arc;

#[derive(Default)]
pub(crate) struct MarkdownPreviewState {
    code: CodeHighlighter,
    cache: CommonMarkCache,
}

pub(crate) struct MarkdownView;

impl MarkdownView {
    pub(crate) fn show(
        ui: &mut egui::Ui,
        state: &mut MarkdownPreviewState,
        text: &str,
        syntax: &Arc<SyntaxColors>,
    ) {
        let options = CommonMarkOptions::default();
        state.code.begin_frame();
        let mut renderer = Renderer::new(&mut state.code, syntax);
        renderer.show(ui, &mut state.cache, &options, text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{self, ThemeChoice};

    fn render(markdown: &str) -> egui::FullOutput {
        render_with(markdown, ThemeChoice::Dark)
    }

    fn render_with(markdown: &str, choice: ThemeChoice) -> egui::FullOutput {
        let ctx = egui::Context::default();
        let syntax = theme::syntax_colors(choice);
        let mut state = MarkdownPreviewState::default();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            MarkdownView::show(ui, &mut state, markdown, &syntax);
        })
    }

    fn collect_text_colors(shape: &egui::Shape, out: &mut Vec<egui::Color32>) {
        match shape {
            egui::Shape::Text(text) => {
                out.extend(text.galley.job.sections.iter().map(|s| s.format.color));
            }
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect_text_colors(shape, out);
                }
            }
            _ => {}
        }
    }

    fn color(value: [u8; 4]) -> egui::Color32 {
        egui::Color32::from_rgba_unmultiplied(value[0], value[1], value[2], value[3])
    }

    #[test]
    fn the_renderer_handles_every_block_kind() {
        let mut output = render(concat!(
            "# Heading\n\n",
            "Paragraph with *emphasis*, **strong**, ~~strike~~ and `code`.\n\n",
            "> quote\n>\n> [!NOTE]\n> alert\n\n",
            "- one\n- two\n  - nested\n\n",
            "1. first\n2. second\n\n",
            "- [x] done\n- [ ] todo\n\n",
            "| a | b |\n|---|---|\n| 1 | 2 |\n\n",
            "```rust\nfn main() {\n    println!(\"hi\");\n}\n```\n\n",
            "```\nplain fence\n```\n\n",
            "    indented code\n\n",
            "[link](https://example.com)\n\n",
            "---\n\n",
            "Term\n: definition\n\n",
            "Footnote[^1]\n\n[^1]: note\n",
        ));
        output.textures_delta.clear();
    }

    #[test]
    fn preview_fence_is_painted_with_the_editor_palette() {
        for choice in [ThemeChoice::Dark, ThemeChoice::Light] {
            let syntax = theme::syntax_colors(choice);
            let mut output = render_with("```rust\nfn main() {}\n```\n", choice);
            let mut colors = Vec::new();
            for clipped in &output.shapes {
                collect_text_colors(&clipped.shape, &mut colors);
            }
            output.textures_delta.clear();
            assert!(
                colors.contains(&color(syntax.tokens["keyword"])),
                "keyword color missing from the {choice:?} preview: {colors:?}"
            );
            assert!(
                colors.contains(&color(syntax.tokens["function"])),
                "function color missing from the {choice:?} preview: {colors:?}"
            );
        }
    }
}
