use crafter::core::config::ConfigManager;
use serde::Serialize;

use super::args::BaseAction;
use super::respond;

#[derive(Debug, Serialize)]
struct SuccessResponse {
    success: bool,
}

#[derive(Debug, Serialize)]
struct BaseStatusResponse {
    initialized: bool,
    config: crafter::types::Config,
}

pub fn handle_base(action: BaseAction) -> crafter::types::Result<()> {
    match action {
        BaseAction::Setup => {
            use crafter::core::config::InitResult;
            use crafter::output::compat as output;
            let config_mgr = ConfigManager::new()?;
            let InitResult { config_dir, data_dir } = config_mgr.initialize()?;

            respond::json_or_when_not_quiet(&SuccessResponse { success: true }, || {
                output::success(&format!("Config directory: {}", output::format_path(&config_dir)));
                output::success(&format!("Data directory:   {}", output::format_path(&data_dir)));
                output::success("Setup complete!");
                output::detail("crafter challenge list           # See available challenges");
                output::detail("crafter challenge init shell go  # Start a challenge");
                Ok(())
            })?;

            Ok(())
        }
        BaseAction::Status => {
            use crafter::output::compat as output;
            let config_mgr = ConfigManager::new()?;

            if output::is_json() {
                let config = config_mgr.get_config()?;
                output::emit_json(&BaseStatusResponse {
                    initialized: config_mgr.is_initialized(),
                    config,
                })?;
            } else if config_mgr.is_initialized() {
                respond::when_not_quiet(|| {
                    use crafter::output::formatter::Formatter;
                    let config = config_mgr.get_config()?;
                    output::success("Crafter is initialized");
                    output::with_stdout(|stdout| {
                        super::config::build_config_formatter(&config_mgr, &config)
                            .format(stdout)
                    })?;
                    Ok(())
                })?;
            } else {
                output::error("Crafter is not initialized");
                output::suggestion("Run 'crafter base setup' to initialize");
            }
            Ok(())
        }
    }
}