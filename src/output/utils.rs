//! Output formatting helpers.

use std::path::{Path, PathBuf};

/// Format size as human-readable or raw bytes.
#[must_use] 
pub fn format_size(bytes: u64, raw: bool) -> String {
    if raw {
        bytes.to_string()
    } else {
        format_size_human(bytes)
    }
}

/// Format size using binary units.
#[must_use] 
pub fn format_size_human(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format_scaled(bytes, TB, "TB")
    } else if bytes >= GB {
        format_scaled(bytes, GB, "GB")
    } else if bytes >= MB {
        format_scaled(bytes, MB, "MB")
    } else if bytes >= KB {
        format_scaled(bytes, KB, "KB")
    } else {
        format!("{bytes} B")
    }
}

fn format_scaled(bytes: u64, unit: u64, suffix: &str) -> String {
    let hundredths = (u128::from(bytes) * 100) / u128::from(unit);
    let whole = hundredths / 100;
    let fraction = hundredths % 100;
    format!("{whole}.{fraction:02} {suffix}")
}

/// Convert size to MB string with 2 decimals.
#[must_use] 
pub fn size_as_mb(bytes: u64) -> String {
    format_scaled(bytes, 1024_u64 * 1024, "")
        .trim_end()
        .to_string()
}

/// Format path with optional home abbreviation.
#[must_use] 
pub fn format_path(path: &Path, full: bool) -> String {
    if full {
        path.display().to_string()
    } else {
        abbreviate_home(path)
    }
}

/// Replace home directory prefix with `~` when possible.
#[must_use] 
pub fn abbreviate_home(path: &Path) -> String {
    if let Some(home) = home::home_dir() {
        if let Ok(stripped) = path.strip_prefix(&home) {
            return format!("~/{}", stripped.display());
        }
    }
    path.display().to_string()
}

/// Expand `~/` to home directory when possible.
#[must_use] 
pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = home::home_dir() {
            return home.join(stripped);
        }
    }
    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size_human() {
        assert_eq!(format_size_human(500), "500 B");
        assert_eq!(format_size_human(1024), "1.00 KB");
        assert_eq!(format_size_human(1536), "1.50 KB");
        assert_eq!(format_size_human(1_048_576), "1.00 MB");
        assert_eq!(format_size_human(1_073_741_824), "1.00 GB");
    }

    #[test]
    fn test_format_size_raw() {
        assert_eq!(format_size(1_234_567, true), "1234567");
        assert_eq!(format_size(1_234_567, false), "1.17 MB");
    }

    #[test]
    fn test_size_as_mb() {
        assert_eq!(size_as_mb(1_048_576), "1.00");
        assert_eq!(size_as_mb(2_097_152), "2.00");
        assert_eq!(size_as_mb(1_572_864), "1.50");
    }
}
