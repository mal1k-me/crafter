//! Runtime bootstrap and top-level command dispatch.

use super::args::{resolve_output_args, Cli};
use super::args::Commands;

pub(crate) fn run(cli: Cli) -> i32 {
    let output_args = resolve_output_args(&cli.command);

    let output_config = build_output_config(output_args);

    crafter::output::compat::configure(output_config);

    if output_args.verbose {
        crafter::output::compat::set_verbose(true);
    }

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(err) => {
            use crafter::output::compat as output;
            output::error(&format!("Failed to create async runtime: {err}"));
            return crafter::error::codes::GENERAL_ERROR;
        }
    };
    let result = runtime.block_on(async {
        match cli.command {
            Commands::Base { action, .. } => super::base::handle_base(action),
            Commands::Challenge { action } => super::challenge::handle_challenge(action).await,
            Commands::Config { action } => super::config::handle_config(action),
            Commands::Test {
                stage,
                all,
                continue_on_failure,
                skip_validation,
                ..
            } => super::test::handle_test(stage, all, continue_on_failure, skip_validation).await,
            Commands::Tester { action } => super::tester::handle_tester(action).await,
        }
    });

    match result {
        Ok(()) => crafter::error::codes::SUCCESS,
        Err(e) => {
            use crafter::output::compat as output;
            output::error(&format!("{e}"));

            if let Some(suggestion) = e.suggestion() {
                output::suggestion(&suggestion);
            }

            map_error_to_exit_code(&e)
        }
    }
}

/// Build effective output configuration.
fn build_output_config(
    output_args: super::args::ResolvedOutputArgs,
) -> crafter::output::OutputPolicy {
    crafter::output::ConfigLoader::load_output_config(crafter::output::CliOutputArgs {
        format: output_args.format,
        verbosity: if output_args.quiet {
            crafter::output::CliVerbosity::Quiet
        } else if output_args.verbose {
            crafter::output::CliVerbosity::Verbose
        } else {
            crafter::output::CliVerbosity::Default
        },
        raw_sizes: if output_args.raw_sizes {
            crafter::output::CliFlag::Enabled
        } else {
            crafter::output::CliFlag::Default
        },
        full_paths: if output_args.full_paths {
            crafter::output::CliFlag::Enabled
        } else {
            crafter::output::CliFlag::Default
        },
    })
}

const fn map_error_to_exit_code(err: &crafter::types::CrafterError) -> i32 {
    use crafter::error::codes;
    use crafter::types::CrafterError;

    match err {
        CrafterError::Docker(_) => codes::DOCKER_ERROR,
        CrafterError::Http(_) => codes::NETWORK_ERROR,
        CrafterError::ChallengeNotFound(_) => codes::NOT_FOUND,
        CrafterError::Io(_)
        | CrafterError::Config(_)
        | CrafterError::Git(_)
        | CrafterError::Tester(_)
        | CrafterError::Json(_)
        | CrafterError::Yaml(_)
        | CrafterError::InvalidPath(_)
        | CrafterError::NotInitialized
        | CrafterError::Other(_)
        | CrafterError::WithSuggestion { .. }
        | CrafterError::CommandFailed(_) => codes::GENERAL_ERROR,
    }
}