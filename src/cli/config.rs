//! Configuration command handlers.

use crafter::core::config::ConfigManager;
use crafter::constants::output_keys as keys;
use serde::Serialize;

use super::args::ConfigAction;
use super::respond;

#[derive(Debug, Serialize)]
struct SuccessResponse {
    success: bool,
}

#[derive(Debug, Serialize)]
struct ConfigGetResponse {
    key: String,
    value: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct ConfigSetResponse {
    success: bool,
    key: String,
    value: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct ConfigPathResponse {
    path: String,
}

pub fn handle_config(action: ConfigAction) -> crafter::types::Result<()> {
    use crafter::output::compat;

    let config_mgr = ConfigManager::new()?;

    match action {
        ConfigAction::Show { .. } => {
            let config = config_mgr.get_config()?;

            respond::json_or_when_not_quiet(&config, || {
                use crafter::output::formatter::Formatter;
                compat::with_stdout(|stdout| {
                    build_config_formatter(&config_mgr, &config).format(stdout)
                })?;
                Ok(())
            })?;

            Ok(())
        }
        ConfigAction::Get { key, .. } => {
            let config = config_mgr.get_config()?;
            let config_key = ConfigKey::parse(&key)?;

            respond::json_or_when_not_quiet(
                &ConfigGetResponse {
                    key: config_key.as_str().to_string(),
                    value: config_key.value_json(&config),
                },
                || {
                    let value = config_key.value_display(&config);
                    compat::list(&[value]);
                    Ok(())
                },
            )?;

            Ok(())
        }
        ConfigAction::Set { key, value, .. } => {
            let mut config = config_mgr.get_config()?;
            let config_key = ConfigKey::parse(&key)?;
            config_key.set_value(&mut config, &value)?;
            config_mgr.save_config(&config)?;

            respond::json_or_when_not_quiet(
                &ConfigSetResponse {
                    success: true,
                    key: config_key.as_str().to_string(),
                    value: config_key.value_json(&config),
                },
                || {
                    let display = config_key.value_display(&config);
                    compat::success(&format!("Set {} = {display}", config_key.as_str()));
                    Ok(())
                },
            )?;

            Ok(())
        }
        ConfigAction::Reset { .. } => {
            config_mgr.save_config(&crafter::types::Config::default())?;

            respond::json_or_when_not_quiet(&SuccessResponse { success: true }, || {
                compat::success("Configuration reset to defaults");
                Ok(())
            })?;

            Ok(())
        }
        ConfigAction::Path { .. } => {
            let config_path = config_mgr.config_path();
            let rendered_path = crafter::output::utils::format_path(
                &config_path,
                compat::use_full_paths(),
            );

            respond::json_or_when_not_quiet(
                &ConfigPathResponse {
                    path: rendered_path.clone(),
                },
                || {
                    compat::list(&[rendered_path]);
                    Ok(())
                },
            )?;

            Ok(())
        }
    }
}

pub fn build_config_formatter(
    config_mgr: &ConfigManager,
    config: &crafter::types::Config,
) -> crafter::output::formatters::ConfigFormatter {
    use crafter::output::compat;

    let mut custom_settings: Vec<(String, String)> = config
        .settings
        .iter()
        .map(|(k, v)| (k.clone(), v.to_string()))
        .collect();
    custom_settings.sort_by(|a, b| a.0.cmp(&b.0));

    let mut formatter = crafter::output::formatters::ConfigFormatter::new(config_mgr.config_path())
        .with_path_display_full(compat::use_full_paths())
        .with_auto_update(config.auto_update)
        .with_output_format(config.output.format.clone())
        .with_output_verbosity(config.output.verbosity.clone())
        .with_raw_sizes(config.output.raw_sizes)
        .with_full_paths(config.output.full_paths)
        .with_force_color(config.output.force_color)
        .with_custom_settings(custom_settings);

    if compat::get_level() >= crafter::output::Level::Verbose {
        formatter = formatter.with_effective_output(
            compat::get_format().to_string(),
            compat::get_level().to_string(),
            compat::use_raw_sizes(),
            compat::use_full_paths(),
        );
    }

    formatter
}

#[derive(Debug, Clone, Copy)]
enum ConfigKey {
    AutoUpdate,
    OutputFormat,
    OutputVerbosity,
    OutputRawSizes,
    OutputFullPaths,
    OutputForceColor,
}

impl ConfigKey {
    const VALID_KEYS: &'static [&'static str] = keys::VALID_CONFIG_KEYS;

    fn parse(key: &str) -> crafter::types::Result<Self> {
        let parsed = match key {
            keys::KEY_AUTO_UPDATE => Self::AutoUpdate,
            keys::KEY_OUTPUT_FORMAT => Self::OutputFormat,
            keys::KEY_OUTPUT_VERBOSITY => Self::OutputVerbosity,
            keys::KEY_OUTPUT_RAW_SIZES => Self::OutputRawSizes,
            keys::KEY_OUTPUT_FULL_PATHS => Self::OutputFullPaths,
            keys::KEY_OUTPUT_FORCE_COLOR => Self::OutputForceColor,
            _ => {
                return Err(crafter::types::CrafterError::with_suggestion(
                    format!("Unknown config key: {key}"),
                    format!("Valid keys: {}", Self::VALID_KEYS.join(", ")),
                ));
            }
        };
        Ok(parsed)
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::AutoUpdate => keys::KEY_AUTO_UPDATE,
            Self::OutputFormat => keys::KEY_OUTPUT_FORMAT,
            Self::OutputVerbosity => keys::KEY_OUTPUT_VERBOSITY,
            Self::OutputRawSizes => keys::KEY_OUTPUT_RAW_SIZES,
            Self::OutputFullPaths => keys::KEY_OUTPUT_FULL_PATHS,
            Self::OutputForceColor => keys::KEY_OUTPUT_FORCE_COLOR,
        }
    }

    fn value_json(self, config: &crafter::types::Config) -> serde_json::Value {
        match self {
            Self::AutoUpdate => serde_json::json!(config.auto_update),
            Self::OutputFormat => config
                .output
                .format
                .as_ref()
                .map_or(serde_json::Value::Null, |v| serde_json::json!(v)),
            Self::OutputVerbosity => config
                .output
                .verbosity
                .as_ref()
                .map_or(serde_json::Value::Null, |v| serde_json::json!(v)),
            Self::OutputRawSizes => serde_json::json!(config.output.raw_sizes),
            Self::OutputFullPaths => serde_json::json!(config.output.full_paths),
            Self::OutputForceColor => serde_json::json!(config.output.force_color),
        }
    }

    fn value_display(self, config: &crafter::types::Config) -> String {
        match self {
            Self::AutoUpdate => config.auto_update.to_string(),
            Self::OutputFormat => config
                .output
                .format
                .clone()
                .unwrap_or_else(|| "<unset>".to_string()),
            Self::OutputVerbosity => config
                .output
                .verbosity
                .clone()
                .unwrap_or_else(|| "<unset>".to_string()),
            Self::OutputRawSizes => config.output.raw_sizes.to_string(),
            Self::OutputFullPaths => config.output.full_paths.to_string(),
            Self::OutputForceColor => config.output.force_color.to_string(),
        }
    }

    fn set_value(self, config: &mut crafter::types::Config, value: &str) -> crafter::types::Result<()> {
        match self {
            Self::AutoUpdate => {
                config.auto_update = parse_bool(value)?;
            }
            Self::OutputFormat => {
                config.output.format = parse_optional_output_format(value)?;
            }
            Self::OutputVerbosity => {
                config.output.verbosity = parse_optional_verbosity(value)?;
            }
            Self::OutputRawSizes => {
                config.output.raw_sizes = parse_bool(value)?;
            }
            Self::OutputFullPaths => {
                config.output.full_paths = parse_bool(value)?;
            }
            Self::OutputForceColor => {
                config.output.force_color = parse_bool(value)?;
            }
        }
        Ok(())
    }
}

fn parse_bool(value: &str) -> crafter::types::Result<bool> {
    value.trim().parse().map_err(|_| {
        crafter::types::CrafterError::Config(format!(
            "Invalid boolean value: {value}. Use true or false."
        ))
    })
}

fn parse_optional_output_format(value: &str) -> crafter::types::Result<Option<String>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    if trimmed.eq_ignore_ascii_case("human") {
        Ok(Some("human".to_string()))
    } else if trimmed.eq_ignore_ascii_case("simple") {
        Ok(Some("simple".to_string()))
    } else if trimmed.eq_ignore_ascii_case("json") {
        Ok(Some("json".to_string()))
    } else {
        Err(crafter::types::CrafterError::Config(format!(
            "Invalid output.format value: {value}. Valid values: human, simple, json."
        )))
    }
}

fn parse_optional_verbosity(value: &str) -> crafter::types::Result<Option<String>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    if trimmed.eq_ignore_ascii_case("silent") || trimmed.eq_ignore_ascii_case("quiet") {
        Ok(Some("silent".to_string()))
    } else if trimmed.eq_ignore_ascii_case("normal") {
        Ok(Some("normal".to_string()))
    } else if trimmed.eq_ignore_ascii_case("verbose") {
        Ok(Some("verbose".to_string()))
    } else if trimmed.eq_ignore_ascii_case("debug") {
        Ok(Some("debug".to_string()))
    } else {
        Err(crafter::types::CrafterError::Config(format!(
            "Invalid output.verbosity value: {value}. Valid values: silent, normal, verbose, debug."
        )))
    }
}