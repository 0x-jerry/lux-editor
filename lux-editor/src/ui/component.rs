use eframe::egui;

/// Standard contract for every renderable UI unit.
///
/// A component renders from a snapshot of borrowed input data and reports the
/// effects it wants as messages. It never mutates app state itself — the host
/// maps messages to actions. State that must survive between frames lives in
/// the component instance, which the host owns.
pub trait Component {
    /// Effects this component requests from its host.
    type Message;
    /// Everything the component needs this frame; borrowed, never copied.
    type Input<'a>;

    /// Renders the component and returns the messages it emitted this frame.
    fn render(&mut self, ui: &mut egui::Ui, input: Self::Input<'_>) -> Vec<Self::Message>;
}
