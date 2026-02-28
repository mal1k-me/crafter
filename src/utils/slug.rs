//! Slug and URL manipulation utilities.

/// Extract challenge name from repository URL.
#[must_use] 
pub fn challenge_from_url(url: &str) -> Option<String> {
    url.split('/')
        .next_back()
    .map(|s| s.strip_suffix(".git").unwrap_or(s))
        .and_then(|s| s.strip_prefix("build-your-own-"))
        .map(std::string::ToString::to_string)
}

/// Extract challenge name from directory name.
#[must_use] 
pub fn challenge_from_dirname(name: &str) -> Option<String> {
    let rest = name.strip_prefix("codecrafters-")?;

    // Convention: directory format is `codecrafters-<challenge>-<language>`.
    // Keep all segments except the trailing language segment so multi-word
    // challenges remain intact (e.g. `bytecode-interpreter`).
    let parts: Vec<&str> = rest.split('-').collect();
    if parts.len() >= 2 {
        Some(parts[..parts.len() - 1].join("-"))
    } else {
        Some(rest.to_string())
    }
}

/// Build repository URL from challenge name.
#[must_use] 
pub fn challenge_to_url(challenge: &str) -> String {
    format!(
        "https://github.com/codecrafters-io/build-your-own-{challenge}"
    )
}

/// Build tester URL from challenge name.
#[must_use] 
pub fn tester_url(challenge: &str) -> String {
    format!("https://github.com/codecrafters-io/{challenge}-tester")
}

/// Return platform OS/arch tuple for release asset matching.
#[must_use] 
pub const fn get_platform() -> (&'static str, &'static str) {
    let os = if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "unknown"
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "amd64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "unknown"
    };

    (os, arch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_challenge_from_url() {
        assert_eq!(
            challenge_from_url("https://github.com/codecrafters-io/build-your-own-shell"),
            Some("shell".to_string())
        );
    }

    #[test]
    fn test_challenge_from_url_git_suffix() {
        assert_eq!(
            challenge_from_url("https://github.com/codecrafters-io/build-your-own-shell.git"),
            Some("shell".to_string())
        );
    }

    #[test]
    fn test_challenge_from_dirname() {
        assert_eq!(
            challenge_from_dirname("codecrafters-shell-rust"),
            Some("shell".to_string())
        );
    }

    #[test]
    fn test_challenge_from_dirname_multi_word_challenge() {
        assert_eq!(
            challenge_from_dirname("codecrafters-bytecode-interpreter-rust"),
            Some("bytecode-interpreter".to_string())
        );
    }
}
