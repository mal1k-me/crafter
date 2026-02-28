//! Shared response helpers for CLI handlers.

use serde::Serialize;

/// Emit JSON in JSON mode, otherwise run human output unless quiet.
pub(crate) fn json_or_when_not_quiet<T, F>(json: &T, human: F) -> crafter::types::Result<()>
where
    T: Serialize,
    F: FnOnce() -> crafter::types::Result<()>,
{
    if crafter::output::compat::is_json() {
        crafter::output::compat::emit_json(json)?;
    } else if !crafter::output::compat::is_quiet() {
        human()?;
    }
    Ok(())
}

/// Execute human output only when quiet mode is disabled.
pub(crate) fn when_not_quiet<F>(human: F) -> crafter::types::Result<()>
where
    F: FnOnce() -> crafter::types::Result<()>,
{
    if !crafter::output::compat::is_quiet() {
        human()?;
    }
    Ok(())
}