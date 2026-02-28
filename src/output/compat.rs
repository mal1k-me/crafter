//! Backward-compatibility wrappers for legacy output call sites.

use crate::output::{Format, Output, OutputPolicy};
use serde::Serialize;
use std::io;
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use termcolor;

// Global output instance for backward compatibility
static GLOBAL_OUTPUT: std::sync::LazyLock<Mutex<Output>> =
    std::sync::LazyLock::new(|| Mutex::new(Output::new(OutputPolicy::new())));

// Global policy for format/verbosity/path/color preferences
static GLOBAL_CONFIG: std::sync::LazyLock<Mutex<OutputPolicy>> = std::sync::LazyLock::new(|| Mutex::new(OutputPolicy::new()));

// Global verbose flag (mirrors utils::output::VERBOSE)
static VERBOSE: AtomicBool = AtomicBool::new(false);

fn with_output_mut<F>(mut f: F)
where
    F: FnMut(&mut Output),
{
    if let Ok(mut out) = GLOBAL_OUTPUT.lock() {
        f(&mut out);
    }
}

fn read_config<R>(default: R, f: impl FnOnce(&OutputPolicy) -> R) -> R {
    GLOBAL_CONFIG.lock().map_or(default, |cfg| f(&cfg))
}

/// Print operation message.
pub fn operation(msg: &str) {
    with_output_mut(|out| out.step(msg));
}

/// Print step message.
pub fn step(msg: &str) {
    operation(msg);
}

/// Print success message.
pub fn success(msg: &str) {
    with_output_mut(|out| out.success(msg));
}

/// Print error message.
pub fn error(msg: &str) {
    with_output_mut(|out| out.error(msg));
}

/// Print warning message.
pub fn warn(msg: &str) {
    with_output_mut(|out| out.warn(msg));
}

#[deprecated(note = "use warn() instead")]
pub fn warning(msg: &str) {
    warn(msg);
}

/// Print info message.
pub fn info(msg: &str) {
    with_output_mut(|out| out.info(msg));
}

/// Print detailed message.
pub fn detail(msg: &str) {
    with_output_mut(|out| out.detail(msg));
}

/// Print debug message.
pub fn debug(msg: &str) {
    with_output_mut(|out| out.debug(msg));
}

/// Print verbose message when enabled.
pub fn verbose(msg: &str) {
    if is_verbose() {
        with_output_mut(|out| out.detail(msg));
    }
}

/// Print suggestion message.
pub fn suggestion(msg: &str) {
    with_output_mut(|out| out.suggestion(msg));
}

/// Return unmodified string (legacy helper).
#[must_use] 
pub fn dim(msg: &str) -> String {
    msg.to_string()
}

/// Enable or disable verbose mode.
pub fn set_verbose(enabled: bool) {
    VERBOSE.store(enabled, Ordering::Relaxed);
}

/// Check if verbose mode is enabled.
pub fn is_verbose() -> bool {
    VERBOSE.load(Ordering::Relaxed)
        || read_config(false, |cfg| cfg.level >= crate::output::Level::Verbose)
}

/// Initialize output (no-op).
pub const fn init() {}

/// Print list of items.
pub fn list<T: AsRef<str>>(items: &[T]) {
    with_output_mut(|out| out.list(items));
}

/// Print key-value pairs.
pub fn keyval(pairs: &[(&str, &str)]) {
    with_output_mut(|out| out.keyval(pairs));
}

/// Print summary data.
pub fn summary(title: &str, items: &[(&str, &str)]) {
    with_output_mut(|out| out.summary(title, items));
}

/// Print table data.
pub fn table(headers: &[&str], rows: &[Vec<String>]) {
    with_output_mut(|out| out.table(headers, rows));
}

/// Return stdout stream using current color settings.
#[must_use] 
pub fn stdout() -> termcolor::StandardStream {
    termcolor::StandardStream::stdout(color_choice())
}

/// Execute a writer function with configured stdout.
/// # Errors
/// Returns an error if the underlying operation fails.
pub fn with_stdout<R>(f: impl FnOnce(&mut termcolor::StandardStream) -> io::Result<R>) -> io::Result<R> {
    let mut out = stdout();
    f(&mut out)
}

/// Emit JSON via global output.
/// # Errors
/// Returns an error if the underlying operation fails.
pub fn emit_json<T: Serialize>(obj: &T) -> io::Result<()> {
    GLOBAL_OUTPUT.lock().map_or(Ok(()), |mut out| out.json(obj))
}

/// Reconfigure global output.
pub fn configure(config: OutputPolicy) {
    if let Ok(mut out) = GLOBAL_OUTPUT.lock() {
        *out = Output::new(config.clone());
    }
    if let Ok(mut cfg) = GLOBAL_CONFIG.lock() {
        *cfg = config;
    }
}

/// Get current output format.
pub fn get_format() -> Format {
    read_config(Format::Human, |cfg| cfg.format)
}

/// Get current verbosity level.
pub fn get_level() -> crate::output::Level {
    read_config(crate::output::Level::Normal, |cfg| cfg.level)
}

/// Check if JSON output is enabled.
#[must_use] 
pub fn is_json() -> bool {
    matches!(get_format(), Format::Json)
}

/// Check if quiet mode is enabled.
pub fn is_quiet() -> bool {
    read_config(false, |cfg| cfg.level == crate::output::Level::Silent)
}

/// Resolve color choice for current output policy.
pub fn color_choice() -> termcolor::ColorChoice {
    read_config(termcolor::ColorChoice::Auto, |cfg| cfg.effective_color_choice())
}

/// Check raw-size preference.
pub fn use_raw_sizes() -> bool {
    read_config(false, |cfg| cfg.raw_sizes)
}

/// Check full-path preference.
pub fn use_full_paths() -> bool {
    read_config(false, |cfg| cfg.full_paths)
}

/// Format path using current path-display preference.
#[must_use]
pub fn format_path(path: &Path) -> String {
    crate::output::utils::format_path(path, use_full_paths())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compat_functions() {
        operation("Testing operation");
        success("Testing success");
        error("Testing error");
        warn("Testing warning");
        detail("Testing detail");
        debug("Testing debug");
        info("Testing info");
        verbose("Testing verbose");
        suggestion("Testing suggestion");
        assert_eq!(dim("plain"), "plain");
    }

    #[test]
    fn test_is_verbose_reflects_output_level() {
        set_verbose(false);
        configure(OutputPolicy::new().with_level(crate::output::Level::Verbose));
        assert!(is_verbose());

        configure(OutputPolicy::new().with_level(crate::output::Level::Normal));
        assert!(!is_verbose());

        set_verbose(true);
        assert!(is_verbose());
        set_verbose(false);
    }
}
