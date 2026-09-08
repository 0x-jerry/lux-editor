//! Cross-domain actions. Each capability gets its own file of `impl Ctx`
//! methods — behaviour that spans two or more domain structs. Pure state
//! transitions stay on the domain structs themselves; these modules only
//! orchestrate them.

mod chrome;
mod documents;
mod editing;
mod highlighting;
mod settings;
mod workspace;
