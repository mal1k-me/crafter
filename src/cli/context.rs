use crafter::core::challenge::ChallengeManager;
use crafter::core::config::ConfigManager;
use crafter::core::git::GitManager;
use crafter::core::tester::TesterManager;
use std::sync::Arc;

pub struct CliContext {
    pub git_mgr: Arc<GitManager>,
    pub challenge_mgr: Arc<ChallengeManager>,
    pub tester_mgr: Arc<TesterManager>,
}

pub fn build_cli_context() -> crafter::types::Result<CliContext> {
    let config_mgr = Arc::new(ConfigManager::new()?);
    let git_mgr = Arc::new(GitManager::new());
    let challenge_mgr = Arc::new(ChallengeManager::new(config_mgr.clone(), git_mgr.clone()));
    let tester_mgr = Arc::new(TesterManager::new(config_mgr));

    Ok(CliContext {
        git_mgr,
        challenge_mgr,
        tester_mgr,
    })
}
