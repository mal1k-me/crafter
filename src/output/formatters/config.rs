//! Formatter for `config show` output.

use crate::constants::output_keys as keys;
use crate::output::formatter::Formatter;
use crate::output::primitives::{KeyValueList, Section};
use crate::output::utils;
use std::io;
use std::path::PathBuf;
use termcolor::WriteColor;

#[derive(Debug, Clone, Copy)]
struct OutputFlags {
    raw_sizes: bool,
    full_paths: bool,
    force_color: bool,
}

#[derive(Debug, Clone)]
struct EffectiveOutput {
    format: String,
    verbosity: String,
    raw_sizes: bool,
    full_paths: bool,
}

/// Formats configuration display.
pub struct ConfigFormatter {
    config_path: PathBuf,
    display_path_full: bool,
    auto_update: bool,
    output_format: Option<String>,
    output_verbosity: Option<String>,
    output_flags: OutputFlags,
    custom_settings: Vec<(String, String)>,
    effective_output: Option<EffectiveOutput>,
}

impl ConfigFormatter {
    /// Create formatter with config path.
    #[must_use] 
    pub const fn new(config_path: PathBuf) -> Self {
        Self {
            config_path,
            display_path_full: false,
            auto_update: false,
            output_format: None,
            output_verbosity: None,
            output_flags: OutputFlags {
                raw_sizes: false,
                full_paths: false,
                force_color: false,
            },
            custom_settings: Vec::new(),
            effective_output: None,
        }
    }

    #[must_use]
    pub const fn with_path_display_full(mut self, v: bool) -> Self {
        self.display_path_full = v;
        self
    }

    #[must_use] 
    pub const fn with_auto_update(mut self, v: bool) -> Self {
        self.auto_update = v;
        self
    }

    #[must_use] 
    pub fn with_output_format(mut self, v: Option<String>) -> Self {
        self.output_format = v;
        self
    }

    #[must_use] 
    pub fn with_output_verbosity(mut self, v: Option<String>) -> Self {
        self.output_verbosity = v;
        self
    }

    #[must_use] 
    pub const fn with_raw_sizes(mut self, v: bool) -> Self {
        self.output_flags.raw_sizes = v;
        self
    }

    #[must_use] 
    pub const fn with_full_paths(mut self, v: bool) -> Self {
        self.output_flags.full_paths = v;
        self
    }

    #[must_use] 
    pub const fn with_force_color(mut self, v: bool) -> Self {
        self.output_flags.force_color = v;
        self
    }

    #[must_use]
    pub fn with_custom_settings(mut self, settings: Vec<(String, String)>) -> Self {
        self.custom_settings = settings;
        self
    }

    #[must_use]
    pub fn with_effective_output(
        mut self,
        format: String,
        verbosity: String,
        raw_sizes: bool,
        full_paths: bool,
    ) -> Self {
        self.effective_output = Some(EffectiveOutput {
            format,
            verbosity,
            raw_sizes,
            full_paths,
        });
        self
    }
}

impl Formatter for ConfigFormatter {
    fn format(&self, w: &mut dyn WriteColor) -> io::Result<()> {
        // Header: configuration file path
        Section::new(format!(
            "Configuration file: {}",
            utils::format_path(&self.config_path, self.display_path_full)
        ))
        .write(w)?;

        // Settings key-value pairs
        let mut list = KeyValueList::new();

        list.add_mut(keys::KEY_AUTO_UPDATE, self.auto_update.to_string());

        list.add_mut(
            keys::KEY_OUTPUT_FORMAT,
            self.output_format
                .as_deref()
                .unwrap_or("<default: auto>"),
        );
        list.add_mut(
            keys::KEY_OUTPUT_VERBOSITY,
            self.output_verbosity
                .as_deref()
                .unwrap_or("<default: normal>"),
        );
        list.add_mut(keys::KEY_OUTPUT_RAW_SIZES, self.output_flags.raw_sizes.to_string());
        list.add_mut(keys::KEY_OUTPUT_FULL_PATHS, self.output_flags.full_paths.to_string());
        list.add_mut(keys::KEY_OUTPUT_FORCE_COLOR, self.output_flags.force_color.to_string());

        if self.custom_settings.is_empty() {
            list.add_mut("settings", "<empty>");
        } else {
            for (key, value) in &self.custom_settings {
                list.add_mut(format!("settings.{key}"), value);
            }
        }

        list.write(w)?;

        if let Some(ref effective) = self.effective_output {
            writeln!(w)?;
            Section::new("Effective output (after CLI/env/config merge)").write(w)?;

            let mut effective_list = KeyValueList::new();
            effective_list.add_mut(keys::KEY_EFFECTIVE_OUTPUT_FORMAT, &effective.format);
            effective_list.add_mut(keys::KEY_EFFECTIVE_OUTPUT_VERBOSITY, &effective.verbosity);
            effective_list.add_mut(keys::KEY_EFFECTIVE_OUTPUT_RAW_SIZES, effective.raw_sizes.to_string());
            effective_list.add_mut(keys::KEY_EFFECTIVE_OUTPUT_FULL_PATHS, effective.full_paths.to_string());
            effective_list.write(w)?;
        }

        Ok(())
    }
}
