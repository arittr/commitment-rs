use crate::error::HookError;
use crate::types::AgentName;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// Install Lefthook hook
///
/// Updates or creates lefthook.yml with prepare-commit-msg hook
pub fn install_lefthook(cwd: &Path, agent: &AgentName) -> Result<(), HookError> {
    // Find existing lefthook config file
    let config_files = [
        "lefthook.yml",
        ".lefthook.yml",
        "lefthook.yaml",
        ".lefthook.yaml",
    ];

    let config_path = config_files
        .iter()
        .map(|name| cwd.join(name))
        .find(|path| path.exists())
        .unwrap_or_else(|| cwd.join("lefthook.yml"));

    // Read existing config or create new one
    let mut config: LefthookConfig = if config_path.exists() {
        let content = fs::read_to_string(&config_path).map_err(HookError::Io)?;
        serde_yaml::from_str(&content).map_err(|e| HookError::ConfigParseFailed {
            reason: e.to_string(),
        })?
    } else {
        LefthookConfig::default()
    };

    // Add prepare-commit-msg hook
    let hook_entry = LefthookHook {
        commands: {
            let mut commands = HashMap::new();
            commands.insert(
                "commitment".to_string(),
                LefthookCommand {
                    run: format!("commitment --agent {} --message-only", agent),
                },
            );
            commands
        },
    };

    config
        .hooks
        .insert("prepare-commit-msg".to_string(), hook_entry);

    // Write updated config
    let yaml = serde_yaml::to_string(&config).map_err(|e| HookError::ConfigWriteFailed {
        reason: e.to_string(),
    })?;

    fs::write(&config_path, yaml).map_err(|_| HookError::ScriptCreationFailed {
        path: config_path.display().to_string(),
    })?;

    Ok(())
}

/// Install Husky hook
///
/// Creates .husky/prepare-commit-msg script
pub fn install_husky(cwd: &Path, agent: &AgentName) -> Result<(), HookError> {
    let husky_dir = cwd.join(".husky");

    // Create .husky directory if it doesn't exist
    if !husky_dir.exists() {
        fs::create_dir_all(&husky_dir).map_err(HookError::Io)?;
    }

    let hook_path = husky_dir.join("prepare-commit-msg");

    // Create hook script
    let script = format!(
        r#"#!/usr/bin/env sh
. "$(dirname -- "$0")/_/husky.sh"

commitment --agent {} --message-only
"#,
        agent
    );

    fs::write(&hook_path, script).map_err(|_| HookError::ScriptCreationFailed {
        path: hook_path.display().to_string(),
    })?;

    // Make executable
    let mut perms = fs::metadata(&hook_path)
        .map_err(HookError::Io)?
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&hook_path, perms).map_err(|_| HookError::ChmodFailed {
        path: hook_path.display().to_string(),
    })?;

    Ok(())
}

/// Install simple-git-hooks hook
///
/// Updates package.json with simple-git-hooks configuration
pub fn install_simple_git_hooks(cwd: &Path, agent: &AgentName) -> Result<(), HookError> {
    let package_json = cwd.join("package.json");

    if !package_json.exists() {
        return Err(HookError::ConfigNotFound {
            path: package_json.display().to_string(),
        });
    }

    // Read package.json
    let content = fs::read_to_string(&package_json).map_err(HookError::Io)?;
    let mut json: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| HookError::ConfigParseFailed {
            reason: e.to_string(),
        })?;

    // Add or update simple-git-hooks section
    let hook_command = format!("commitment --agent {} --message-only", agent);

    if let Some(obj) = json.as_object_mut() {
        let hooks = obj
            .entry("simple-git-hooks")
            .or_insert_with(|| serde_json::json!({}));

        if let Some(hooks_obj) = hooks.as_object_mut() {
            hooks_obj.insert(
                "prepare-commit-msg".to_string(),
                serde_json::Value::String(hook_command),
            );
        }
    }

    // Write back with pretty printing
    let updated =
        serde_json::to_string_pretty(&json).map_err(|e| HookError::ConfigWriteFailed {
            reason: e.to_string(),
        })?;

    fs::write(&package_json, updated).map_err(|e| HookError::ConfigWriteFailed {
        reason: e.to_string(),
    })?;

    Ok(())
}

/// Install plain git hook
///
/// Creates .git/hooks/prepare-commit-msg script
/// Handles git worktrees by resolving .git file's gitdir reference
pub fn install_plain_git(cwd: &Path, agent: &AgentName) -> Result<(), HookError> {
    let git_dir = resolve_git_dir(cwd)?;
    let hooks_dir = git_dir.join("hooks");

    // Create hooks directory if it doesn't exist
    if !hooks_dir.exists() {
        fs::create_dir_all(&hooks_dir).map_err(HookError::Io)?;
    }

    let hook_path = hooks_dir.join("prepare-commit-msg");

    // Create hook script
    let script = format!(
        r#"#!/usr/bin/env sh
# commitment hook

commitment --agent {} --message-only
"#,
        agent
    );

    fs::write(&hook_path, script).map_err(|_| HookError::ScriptCreationFailed {
        path: hook_path.display().to_string(),
    })?;

    // Make executable
    let mut perms = fs::metadata(&hook_path)
        .map_err(HookError::Io)?
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&hook_path, perms).map_err(|_| HookError::ChmodFailed {
        path: hook_path.display().to_string(),
    })?;

    Ok(())
}

/// Resolve git directory, handling worktrees
///
/// If .git is a directory, return it
/// If .git is a file (worktree), parse gitdir reference and return that
fn resolve_git_dir(cwd: &Path) -> Result<std::path::PathBuf, HookError> {
    let git_path = cwd.join(".git");

    if !git_path.exists() {
        return Err(HookError::GitDirResolutionFailed);
    }

    // Check if it's a directory (normal repo)
    if git_path.is_dir() {
        return Ok(git_path);
    }

    // It's a file (worktree) - read gitdir reference
    let content = fs::read_to_string(&git_path).map_err(HookError::Io)?;

    for line in content.lines() {
        if let Some(gitdir) = line.strip_prefix("gitdir: ") {
            let resolved = if std::path::Path::new(gitdir).is_absolute() {
                std::path::PathBuf::from(gitdir)
            } else {
                cwd.join(gitdir)
            };
            return Ok(resolved);
        }
    }

    Err(HookError::GitDirResolutionFailed)
}

// Lefthook config structures
#[derive(Debug, Serialize, Deserialize, Default)]
struct LefthookConfig {
    #[serde(flatten)]
    hooks: HashMap<String, LefthookHook>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LefthookHook {
    commands: HashMap<String, LefthookCommand>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LefthookCommand {
    run: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn install_lefthook_creates_new_config() {
        let dir = TempDir::new().unwrap();
        install_lefthook(dir.path(), &AgentName::Claude).unwrap();

        let config_path = dir.path().join("lefthook.yml");
        assert!(config_path.exists());

        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("prepare-commit-msg"));
        assert!(content.contains("commitment"));
        assert!(content.contains("--agent claude"));
    }

    #[test]
    fn install_lefthook_updates_existing_config() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("lefthook.yml");

        // Create existing config
        fs::write(
            &config_path,
            r#"
pre-commit:
  commands:
    lint:
      run: npm run lint
"#,
        )
        .unwrap();

        install_lefthook(dir.path(), &AgentName::Codex).unwrap();

        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("pre-commit"));
        assert!(content.contains("prepare-commit-msg"));
        assert!(content.contains("--agent codex"));
    }

    #[test]
    fn install_husky_creates_directory() {
        let dir = TempDir::new().unwrap();
        install_husky(dir.path(), &AgentName::Gemini).unwrap();

        let hook_path = dir.path().join(".husky/prepare-commit-msg");
        assert!(hook_path.exists());

        let content = fs::read_to_string(&hook_path).unwrap();
        assert!(content.contains("#!/usr/bin/env sh"));
        assert!(content.contains("commitment --agent gemini"));

        // Check executable
        let metadata = fs::metadata(&hook_path).unwrap();
        let permissions = metadata.permissions();
        assert!(permissions.mode() & 0o111 != 0);
    }

    #[test]
    fn install_simple_git_hooks_updates_package_json() {
        let dir = TempDir::new().unwrap();
        let package_json = dir.path().join("package.json");

        // Create package.json
        fs::write(&package_json, r#"{"name": "test"}"#).unwrap();

        install_simple_git_hooks(dir.path(), &AgentName::Claude).unwrap();

        let content = fs::read_to_string(&package_json).unwrap();
        assert!(content.contains("simple-git-hooks"));
        assert!(content.contains("prepare-commit-msg"));
        assert!(content.contains("commitment --agent claude"));
    }

    #[test]
    fn install_simple_git_hooks_fails_without_package_json() {
        let dir = TempDir::new().unwrap();
        let result = install_simple_git_hooks(dir.path(), &AgentName::Claude);
        assert!(matches!(result, Err(HookError::ConfigNotFound { .. })));
    }

    #[test]
    fn install_plain_git_creates_hook() {
        let dir = TempDir::new().unwrap();
        let git_dir = dir.path().join(".git");
        fs::create_dir(&git_dir).unwrap();

        install_plain_git(dir.path(), &AgentName::Claude).unwrap();

        let hook_path = git_dir.join("hooks/prepare-commit-msg");
        assert!(hook_path.exists());

        let content = fs::read_to_string(&hook_path).unwrap();
        assert!(content.contains("#!/usr/bin/env sh"));
        assert!(content.contains("commitment --agent claude"));

        // Check executable
        let metadata = fs::metadata(&hook_path).unwrap();
        let permissions = metadata.permissions();
        assert!(permissions.mode() & 0o111 != 0);
    }

    #[test]
    fn resolve_git_dir_handles_directory() {
        let dir = TempDir::new().unwrap();
        let git_dir = dir.path().join(".git");
        fs::create_dir(&git_dir).unwrap();

        let resolved = resolve_git_dir(dir.path()).unwrap();
        assert_eq!(resolved, git_dir);
    }

    #[test]
    fn resolve_git_dir_handles_worktree_file() {
        let dir = TempDir::new().unwrap();
        let git_file = dir.path().join(".git");
        let actual_git_dir = dir.path().join("../.git/worktrees/test");

        fs::write(&git_file, format!("gitdir: {}", actual_git_dir.display())).unwrap();

        let resolved = resolve_git_dir(dir.path()).unwrap();
        assert!(resolved.ends_with(".git/worktrees/test"));
    }

    #[test]
    fn resolve_git_dir_handles_absolute_path() {
        let dir = TempDir::new().unwrap();
        let git_file = dir.path().join(".git");
        let actual_git_dir = TempDir::new().unwrap();

        fs::write(
            &git_file,
            format!("gitdir: {}", actual_git_dir.path().display()),
        )
        .unwrap();

        let resolved = resolve_git_dir(dir.path()).unwrap();
        assert_eq!(resolved, actual_git_dir.path());
    }

    #[test]
    fn resolve_git_dir_fails_without_git_dir() {
        let dir = TempDir::new().unwrap();
        let result = resolve_git_dir(dir.path());
        assert!(matches!(result, Err(HookError::GitDirResolutionFailed)));
    }
}
