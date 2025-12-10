pub mod managers;

use crate::error::HookError;
use crate::types::AgentName;
use std::fmt;
use std::path::Path;
use std::str::FromStr;

/// Hook manager types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookManager {
    Lefthook,
    Husky,
    SimpleGitHooks,
    PlainGit,
}

impl FromStr for HookManager {
    type Err = HookManagerParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "lefthook" => Ok(Self::Lefthook),
            "husky" => Ok(Self::Husky),
            "simple-git-hooks" => Ok(Self::SimpleGitHooks),
            "plain" | "git" | "plaingit" => Ok(Self::PlainGit),
            _ => Err(HookManagerParseError {
                invalid: s.to_string(),
            }),
        }
    }
}

impl fmt::Display for HookManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lefthook => write!(f, "lefthook"),
            Self::Husky => write!(f, "husky"),
            Self::SimpleGitHooks => write!(f, "simple-git-hooks"),
            Self::PlainGit => write!(f, "plain git hooks"),
        }
    }
}

/// Error when parsing hook manager name from string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookManagerParseError {
    invalid: String,
}

impl fmt::Display for HookManagerParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid hook manager '{}' (expected: lefthook, husky, simple-git-hooks, plain)",
            self.invalid
        )
    }
}

impl std::error::Error for HookManagerParseError {}

/// Detect hook manager in the given directory
///
/// Checks for hook manager configuration files in this order:
/// 1. Lefthook (lefthook.yml, .lefthook.yml, lefthook.yaml, .lefthook.yaml)
/// 2. Husky (.husky directory)
/// 3. simple-git-hooks (package.json with simple-git-hooks field)
/// 4. None (caller should use PlainGit as fallback)
pub fn detect_hook_manager(cwd: &Path) -> Option<HookManager> {
    // Check for Lefthook config files
    let lefthook_files = [
        "lefthook.yml",
        ".lefthook.yml",
        "lefthook.yaml",
        ".lefthook.yaml",
    ];
    for filename in &lefthook_files {
        if cwd.join(filename).exists() {
            return Some(HookManager::Lefthook);
        }
    }

    // Check for Husky directory
    if cwd.join(".husky").is_dir() {
        return Some(HookManager::Husky);
    }

    // Check for simple-git-hooks in package.json
    let package_json = cwd.join("package.json");
    if package_json.exists()
        && let Ok(content) = std::fs::read_to_string(&package_json)
        && let Ok(json) = serde_json::from_str::<serde_json::Value>(&content)
        && json.get("simple-git-hooks").is_some()
    {
        return Some(HookManager::SimpleGitHooks);
    }

    None
}

/// Install hook for the specified manager
///
/// Dispatches to the appropriate manager-specific installation function
pub fn install_hook(manager: HookManager, cwd: &Path, agent: &AgentName) -> Result<(), HookError> {
    match manager {
        HookManager::Lefthook => managers::install_lefthook(cwd, agent),
        HookManager::Husky => managers::install_husky(cwd, agent),
        HookManager::SimpleGitHooks => managers::install_simple_git_hooks(cwd, agent),
        HookManager::PlainGit => managers::install_plain_git(cwd, agent),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn hook_manager_from_str_lefthook() {
        assert_eq!(
            "lefthook".parse::<HookManager>().unwrap(),
            HookManager::Lefthook
        );
        assert_eq!(
            "Lefthook".parse::<HookManager>().unwrap(),
            HookManager::Lefthook
        );
        assert_eq!(
            "LEFTHOOK".parse::<HookManager>().unwrap(),
            HookManager::Lefthook
        );
    }

    #[test]
    fn hook_manager_from_str_husky() {
        assert_eq!("husky".parse::<HookManager>().unwrap(), HookManager::Husky);
        assert_eq!("Husky".parse::<HookManager>().unwrap(), HookManager::Husky);
    }

    #[test]
    fn hook_manager_from_str_simple_git_hooks() {
        assert_eq!(
            "simple-git-hooks".parse::<HookManager>().unwrap(),
            HookManager::SimpleGitHooks
        );
    }

    #[test]
    fn hook_manager_from_str_plain_git() {
        assert_eq!(
            "plain".parse::<HookManager>().unwrap(),
            HookManager::PlainGit
        );
        assert_eq!("git".parse::<HookManager>().unwrap(), HookManager::PlainGit);
        assert_eq!(
            "plaingit".parse::<HookManager>().unwrap(),
            HookManager::PlainGit
        );
    }

    #[test]
    fn hook_manager_from_str_invalid() {
        let result = "invalid".parse::<HookManager>();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("invalid"));
        assert!(err.to_string().contains("lefthook"));
    }

    #[test]
    fn hook_manager_display() {
        assert_eq!(HookManager::Lefthook.to_string(), "lefthook");
        assert_eq!(HookManager::Husky.to_string(), "husky");
        assert_eq!(HookManager::SimpleGitHooks.to_string(), "simple-git-hooks");
        assert_eq!(HookManager::PlainGit.to_string(), "plain git hooks");
    }

    #[test]
    fn detect_lefthook_yml() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("lefthook.yml"), "").unwrap();
        assert_eq!(detect_hook_manager(dir.path()), Some(HookManager::Lefthook));
    }

    #[test]
    fn detect_lefthook_dot_yml() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".lefthook.yml"), "").unwrap();
        assert_eq!(detect_hook_manager(dir.path()), Some(HookManager::Lefthook));
    }

    #[test]
    fn detect_lefthook_yaml() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("lefthook.yaml"), "").unwrap();
        assert_eq!(detect_hook_manager(dir.path()), Some(HookManager::Lefthook));
    }

    #[test]
    fn detect_husky() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join(".husky")).unwrap();
        assert_eq!(detect_hook_manager(dir.path()), Some(HookManager::Husky));
    }

    #[test]
    fn detect_simple_git_hooks() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"simple-git-hooks": {"pre-commit": "test"}}"#,
        )
        .unwrap();
        assert_eq!(
            detect_hook_manager(dir.path()),
            Some(HookManager::SimpleGitHooks)
        );
    }

    #[test]
    fn detect_none() {
        let dir = TempDir::new().unwrap();
        assert_eq!(detect_hook_manager(dir.path()), None);
    }

    #[test]
    fn detect_priority_lefthook_over_husky() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("lefthook.yml"), "").unwrap();
        fs::create_dir(dir.path().join(".husky")).unwrap();
        assert_eq!(detect_hook_manager(dir.path()), Some(HookManager::Lefthook));
    }

    #[test]
    fn detect_priority_husky_over_simple_git_hooks() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join(".husky")).unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"simple-git-hooks": {}}"#,
        )
        .unwrap();
        assert_eq!(detect_hook_manager(dir.path()), Some(HookManager::Husky));
    }
}
