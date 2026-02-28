//! Filesystem utility helpers used across the project.

use crate::types::Result;
use std::path::{Path, PathBuf};

const KB: u64 = 1024;
const MB: u64 = KB * 1024;
const GB: u64 = MB * 1024;

fn format_scaled(bytes: u64, unit: u64, suffix: &str) -> String {
    let hundredths = (u128::from(bytes) * 100) / u128::from(unit);
    let whole = hundredths / 100;
    let fraction = hundredths % 100;
    format!("{whole}.{fraction:02} {suffix}")
}

/// Copy directory recursively
/// # Errors
/// Returns an error if the underlying operation fails.
pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();

    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

/// Check if path exists
#[must_use]
pub fn exists(path: impl AsRef<Path>) -> bool {
    path.as_ref().exists()
}

/// Ensure directory exists, create if not
/// # Errors
/// Returns an error if the underlying operation fails.
pub fn ensure_dir(path: impl AsRef<Path>) -> Result<()> {
    std::fs::create_dir_all(path)?;
    Ok(())
}

/// Read file to string
/// # Errors
/// Returns an error if the underlying operation fails.
pub fn read_to_string(path: impl AsRef<Path>) -> Result<String> {
    std::fs::read_to_string(path).map_err(Into::into)
}

/// Write string to file
/// # Errors
/// Returns an error if the underlying operation fails.
pub fn write_string(path: impl AsRef<Path>, content: &str) -> Result<()> {
    std::fs::write(path, content).map_err(Into::into)
}

/// Get file name from path
#[must_use]
pub fn file_name(path: impl AsRef<Path>) -> Option<String> {
    path.as_ref()
        .file_name()
        .and_then(|n| n.to_str())
        .map(std::string::ToString::to_string)
}

/// Join paths safely
#[must_use]
pub fn join(base: impl AsRef<Path>, path: impl AsRef<Path>) -> PathBuf {
    base.as_ref().join(path)
}

/// Get size of a file in bytes
/// # Errors
/// Returns an error if the underlying operation fails.
pub fn file_size(path: impl AsRef<Path>) -> Result<u64> {
    let metadata = std::fs::metadata(path)?;
    Ok(metadata.len())
}

/// Get total size of a directory recursively in bytes
/// # Errors
/// Returns an error if the underlying operation fails.
pub fn dir_size(path: impl AsRef<Path>) -> Result<u64> {
    fn calc_size(path: &Path) -> std::io::Result<u64> {
        let mut total = 0u64;

        if path.is_file() {
            return Ok(std::fs::metadata(path)?.len());
        }

        if path.is_dir() {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let entry_path = entry.path();
                total += calc_size(&entry_path)?;
            }
        }

        Ok(total)
    }

    calc_size(path.as_ref()).map_err(Into::into)
}

/// Format bytes to human-readable size (KB, MB, GB)
#[must_use] 
pub fn format_size(bytes: u64) -> String {
    if bytes >= GB {
        format_scaled(bytes, GB, "GB")
    } else if bytes >= MB {
        format_scaled(bytes, MB, "MB")
    } else if bytes >= KB {
        format_scaled(bytes, KB, "KB")
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn q_a_format_size_covers_units() {
        let cases = [
            (0, "0 B"),
            (1024, "1.00 KB"),
            (1024 * 1024, "1.00 MB"),
            (1024 * 1024 * 1024, "1.00 GB"),
        ];

        for (bytes, expected) in cases {
            assert_eq!(format_size(bytes), expected);
        }
    }

    #[test]
    fn q_a_exists_and_ensure_dir_work_together() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("a").join("b").join("c");
        assert!(!exists(&nested));
        ensure_dir(&nested).unwrap();
        assert!(exists(&nested));
    }

    #[test]
    fn q_a_read_write_and_file_size() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.txt");
        write_string(&file, "hello crafter").unwrap();
        assert_eq!(read_to_string(&file).unwrap(), "hello crafter");
        assert_eq!(file_size(&file).unwrap(), 13);
    }

    #[test]
    fn q_a_dir_size_and_copy_dir_all() {
        let src = tempdir().unwrap();
        let dst = tempdir().unwrap();

        write_string(src.path().join("file.txt"), "data").unwrap();
        let sub = src.path().join("sub");
        ensure_dir(&sub).unwrap();
        write_string(sub.join("nested.txt"), "nested").unwrap();

        assert_eq!(dir_size(src.path()).unwrap(), 10);

        copy_dir_all(src.path(), dst.path()).unwrap();

        assert_eq!(read_to_string(dst.path().join("file.txt")).unwrap(), "data");
        assert_eq!(
            read_to_string(dst.path().join("sub").join("nested.txt")).unwrap(),
            "nested"
        );
    }

    #[test]
    fn q_a_file_name_and_missing_paths_behave() {
        assert_eq!(file_name("/some/path/foo.txt"), Some("foo.txt".to_string()));
        assert_eq!(file_name("/"), None);
        assert!(read_to_string("/tmp/__crafter_missing_file_xyz.txt").is_err());
        assert!(file_size("/tmp/__crafter_no_file_xyz").is_err());
    }
}
