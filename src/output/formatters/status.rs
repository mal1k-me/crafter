//! Formatter for project status output.

use crate::output::formatter::Formatter;
use crate::output::primitives::KeyValueList;
use crate::output::utils::format_size;
use crate::types::output::StatusOutput;
use std::io;
use termcolor::WriteColor;

/// Formats status output as key-value pairs.
pub struct StatusFormatter {
    status: StatusOutput,
    raw_sizes: bool,
}

impl StatusFormatter {
    /// Create status formatter.
    #[must_use] 
    pub const fn new(status: StatusOutput) -> Self {
        Self {
            status,
            raw_sizes: false,
        }
    }

    /// Enable raw-byte size display.
    #[must_use] 
    pub const fn with_raw_sizes(mut self, raw: bool) -> Self {
        self.raw_sizes = raw;
        self
    }
}

impl Formatter for StatusFormatter {
    fn format(&self, w: &mut dyn WriteColor) -> io::Result<()> {
        let mut list = KeyValueList::new();

        // Basic info
        list.add_mut("directory", &self.status.directory);

        if let Some(ref chall) = self.status.challenge {
            list.add_mut("challenge", chall);
        } else {
            list.add_mut("challenge", "unknown");
        }

        list.add_mut("buildpack", &self.status.buildpack);
        list.add_mut("debug", self.status.debug.to_string());

        // Challenge repo status
        if let Some(ref repo) = self.status.challenge_repo {
            if repo.downloaded {
                list.add_mut("challenge_repo", "downloaded");

                if let Some(size_bytes) = repo.size_bytes {
                    list.add_mut(
                        "challenge_repo_size",
                        format_size(size_bytes, self.raw_sizes),
                    );
                }

                if let Some(ref path) = repo.path {
                    list.add_mut("challenge_repo_path", path);
                }
            } else {
                list.add_mut("challenge_repo", "not downloaded");
            }
        }

        // Dockerfile status
        if let Some(ref dockerfile) = self.status.dockerfile {
            if dockerfile.found {
                list.add_mut("dockerfile", "available");

                if let Some(ref path) = dockerfile.path {
                    list.add_mut("dockerfile_path", path);
                }
            } else {
                list.add_mut(
                    "dockerfile",
                    format!("missing ({}.Dockerfile)", self.status.buildpack),
                );
            }
        }

        // Tester status
        if let Some(ref tester) = self.status.tester {
            if tester.downloaded {
                list.add_mut("tester", "ready");

                if let Some(ref version) = tester.version {
                    list.add_mut("tester_version", version);
                }

                if let Some(size_bytes) = tester.size_bytes {
                    list.add_mut("tester_size", format_size(size_bytes, self.raw_sizes));
                }

                if let Some(ref path) = tester.path {
                    list.add_mut("tester_path", path);
                }
            } else {
                list.add_mut("tester", "not available");
            }
        }

        // Docker status
        if let Some(ref docker) = self.status.docker {
            if docker.available {
                list.add_mut("docker", "available");
                if let Some(ref version) = docker.version {
                    list.add_mut("docker_version", version);
                }
            } else {
                list.add_mut("docker", "not available");
            }
        }

        // Write all key-value pairs
        list.write(w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::output::RepoStatus;
    use termcolor::Buffer;

    #[test]
    fn test_status_formatter_basic() {
        let status = StatusOutput {
            directory: "/path/to/project".to_string(),
            challenge: Some("redis".to_string()),
            buildpack: "rust-1.77".to_string(),
            debug: false,
            challenge_repo: None,
            dockerfile: None,
            tester: None,
            docker: None,
        };

        let mut buffer = Buffer::no_color();
        StatusFormatter::new(status).format(&mut buffer).unwrap();

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("directory: /path/to/project"));
        assert!(output.contains("challenge: redis"));
        assert!(output.contains("buildpack: rust-1.77"));
        assert!(output.contains("debug: false"));

        let expected = [
            "directory: /path/to/project",
            "challenge: redis",
            "buildpack: rust-1.77",
            "debug: false",
        ]
        .join("\n");
        assert!(output.starts_with(&expected));
    }

    #[test]
    fn test_status_formatter_with_repo() {
        let status = StatusOutput {
            directory: "/project".to_string(),
            challenge: Some("git".to_string()),
            buildpack: "c-23".to_string(),
            debug: false,
            challenge_repo: Some(RepoStatus {
                downloaded: true,
                size_bytes: Some(1_024_000),
                size_mb: Some("1.00".to_string()),
                path: Some("/path/to/repo".to_string()),
            }),
            dockerfile: None,
            tester: None,
            docker: None,
        };

        let mut buffer = Buffer::no_color();
        StatusFormatter::new(status).format(&mut buffer).unwrap();

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("challenge_repo: downloaded"));
        assert!(output.contains("challenge_repo_size:"));
        assert!(output.contains("challenge_repo_path: /path/to/repo"));
    }
}
