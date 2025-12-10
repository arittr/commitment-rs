use crate::agents::Agent;
use crate::error::{AgentError, GeneratorError, GitError};
use crate::git::{GitProvider, RealGitProvider};
use crate::hooks::{HookManager, detect_hook_manager, install_hook};
use crate::types::AgentName;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;

/// AI-powered commit message generator
#[derive(Parser, Debug)]
#[command(name = "commitment")]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// AI agent to use for generation
    #[arg(long, default_value = "claude", global = true)]
    pub agent: String,

    /// Generate message without committing
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Output raw message only (no formatting, for piping)
    #[arg(long, global = true)]
    pub message_only: bool,

    /// Suppress progress output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Show debug output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Working directory
    #[arg(long, default_value = ".", global = true)]
    pub cwd: PathBuf,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Initialize git hooks for automatic commit message generation
    Init {
        /// Override hook manager auto-detection
        #[arg(long)]
        hook_manager: Option<String>,

        /// Default agent for hooks
        #[arg(long, default_value = "claude")]
        agent: String,
    },
}

impl Cli {
    /// Run the CLI application
    pub async fn run(self) -> Result<()> {
        match self.command {
            Some(Command::Init {
                hook_manager,
                agent,
            }) => run_init(hook_manager, agent).await,
            None => {
                // Default command: generate
                run_generate(GenerateArgs {
                    agent: self.agent,
                    dry_run: self.dry_run,
                    message_only: self.message_only,
                    quiet: self.quiet,
                    verbose: self.verbose,
                    cwd: self.cwd,
                })
                .await
            }
        }
    }
}

/// Arguments for generate command
#[derive(Debug)]
pub struct GenerateArgs {
    pub agent: String,
    pub dry_run: bool,
    pub message_only: bool,
    pub quiet: bool,
    pub verbose: bool,
    pub cwd: PathBuf,
}

/// Run the generate command
pub async fn run_generate(args: GenerateArgs) -> Result<()> {
    // Parse agent name
    let agent_name: AgentName = args
        .agent
        .parse()
        .context(format!("Invalid agent name '{}'", args.agent))?;

    if args.verbose {
        eprintln!("{} Using agent: {}", style("debug:").cyan(), agent_name);
        eprintln!(
            "{} Working directory: {}",
            style("debug:").cyan(),
            args.cwd.display()
        );
    }

    // Create git provider
    let git = RealGitProvider::new(args.cwd.clone());

    // Create agent
    let agent = Agent::from(agent_name);

    // Generate default signature based on agent
    let signature = format!(
        "🤖 Generated with {} via commitment",
        match agent_name {
            AgentName::Claude => "Claude",
            AgentName::Codex => "Codex",
            AgentName::Gemini => "Gemini",
        }
    );

    // Show spinner unless quiet or message-only mode
    let spinner = if !args.quiet && !args.message_only {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
                .template("{spinner:.cyan} {msg}")
                .expect("valid template"),
        );
        pb.set_message("Generating commit message...");
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(pb)
    } else {
        None
    };

    // Generate commit message
    let result = crate::generate_commit_message(&git, &agent, Some(&signature)).await;

    // Stop spinner
    if let Some(pb) = spinner {
        pb.finish_and_clear();
    }

    // Handle result
    match result {
        Ok(commit) => {
            if args.message_only {
                // Just print raw message for piping
                println!("{}", commit.as_str());
            } else if args.dry_run {
                // Print formatted message without committing
                if !args.quiet {
                    eprintln!("{} Generated commit message:", style("✓").green().bold());
                    eprintln!();
                }
                println!("{}", commit.as_str());
                if !args.quiet {
                    eprintln!();
                    eprintln!("{} Run without --dry-run to commit", style("→").blue());
                }
            } else {
                // Commit with generated message
                git.commit(commit.as_str())
                    .context("Failed to create commit")?;

                if !args.quiet {
                    eprintln!("{} Commit created successfully", style("✓").green().bold());
                    eprintln!();
                    println!("{}", commit.as_str());
                }
            }
            Ok(())
        }
        Err(e) => {
            format_error(&e, &args);
            Err(anyhow::anyhow!("generation failed"))
        }
    }
}

/// Run the init command
pub async fn run_init(hook_manager: Option<String>, agent: String) -> Result<()> {
    // Parse agent name
    let agent_name: AgentName = agent
        .parse()
        .context(format!("Invalid agent name '{}'", agent))?;

    // Determine hook manager (detect or use specified)
    let manager = if let Some(manager_str) = hook_manager {
        // User specified a manager
        manager_str
            .parse::<HookManager>()
            .context(format!("Invalid hook manager '{}'", manager_str))?
    } else {
        // Auto-detect hook manager
        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        detect_hook_manager(&cwd).unwrap_or(HookManager::PlainGit)
    };

    eprintln!(
        "{} Installing {} hook for agent {}...",
        style("→").blue(),
        manager,
        agent_name
    );

    // Install hook
    let cwd = std::env::current_dir().context("Failed to get current directory")?;
    install_hook(manager, &cwd, &agent_name).context("Failed to install hook")?;

    eprintln!("{} Hook installed successfully", style("✓").green().bold());
    eprintln!();
    eprintln!("  Manager: {}", manager);
    eprintln!("  Agent: {}", agent_name);
    eprintln!();
    eprintln!(
        "{} Commit messages will now be generated automatically",
        style("→").blue()
    );

    Ok(())
}

/// Format error messages with helpful hints
fn format_error(error: &GeneratorError, args: &GenerateArgs) {
    eprintln!("{} {}", style("error:").red().bold(), error);
    eprintln!();

    // Add context-specific hints
    match error {
        GeneratorError::Agent(agent_err) => match agent_err {
            AgentError::NotFound { agent } => {
                eprintln!("{} Installation instructions:", style("hint:").yellow());
                match agent {
                    AgentName::Claude => {
                        eprintln!("  Claude CLI: https://docs.anthropic.com/en/docs/claude-cli");
                    }
                    AgentName::Codex => {
                        eprintln!("  Codex CLI: https://github.com/phughk/codex");
                    }
                    AgentName::Gemini => {
                        eprintln!("  Gemini CLI: https://github.com/google/generative-ai-cli");
                    }
                }
            }
            AgentError::ExecutionFailed { agent, stderr } => {
                eprintln!("{} Agent execution details:", style("hint:").yellow());
                eprintln!("  Agent: {}", agent);
                eprintln!("  Error: {}", stderr);
                if args.verbose {
                    eprintln!();
                    eprintln!(
                        "{} Try running the agent manually to verify it works:",
                        style("hint:").yellow()
                    );
                    eprintln!("  {} --version", agent);
                }
            }
            AgentError::Timeout {
                agent,
                timeout_secs,
            } => {
                eprintln!("{} Agent timed out:", style("hint:").yellow());
                eprintln!("  Agent: {}", agent);
                eprintln!("  Timeout: {}s", timeout_secs);
                eprintln!();
                eprintln!("  This usually means the AI is taking too long to respond.");
                eprintln!("  Try again, or check your network connection.");
            }
            AgentError::InvalidResponse { reason } => {
                eprintln!("{} Invalid response from AI:", style("hint:").yellow());
                eprintln!("  {}", reason);
                if args.verbose {
                    eprintln!();
                    eprintln!("  The AI response couldn't be parsed as a commit message.");
                    eprintln!("  This is usually a temporary issue - try again.");
                }
            }
        },
        GeneratorError::Git(git_err) => match git_err {
            GitError::NoStagedChanges => {
                eprintln!("{} No changes to commit:", style("hint:").yellow());
                eprintln!("  Stage your changes first:");
                eprintln!("    git add <files>");
                eprintln!();
                eprintln!("  Or stage all changes:");
                eprintln!("    git add -A");
            }
            GitError::CommandFailed { command, stderr } => {
                eprintln!("{} Git command failed:", style("hint:").yellow());
                eprintln!("  Command: {}", command);
                eprintln!("  Error: {}", stderr);
                if args.verbose {
                    eprintln!();
                    eprintln!(
                        "{} Working directory: {}",
                        style("debug:").cyan(),
                        args.cwd.display()
                    );
                }
            }
            GitError::WorktreeResolution { path } => {
                eprintln!(
                    "{} Git worktree resolution failed:",
                    style("hint:").yellow()
                );
                eprintln!("  Path: {}", path);
                eprintln!();
                eprintln!("  This usually means you're not in a git repository.");
                eprintln!("  Make sure you're running this command in a git repository.");
            }
            GitError::Io(io_err) => {
                eprintln!("{} I/O error:", style("hint:").yellow());
                eprintln!("  {}", io_err);
                if args.verbose {
                    eprintln!();
                    eprintln!(
                        "{} Working directory: {}",
                        style("debug:").cyan(),
                        args.cwd.display()
                    );
                }
            }
        },
        GeneratorError::Validation(reason) => {
            eprintln!("{} Commit validation failed:", style("hint:").yellow());
            eprintln!("  {}", reason);
            eprintln!();
            eprintln!("  The AI generated an invalid commit message.");
            eprintln!("  Expected format: <type>(<scope>): <description>");
            eprintln!();
            eprintln!(
                "  Valid types: feat, fix, docs, style, refactor, test, chore, perf, build, ci, revert"
            );
            if args.verbose {
                eprintln!();
                eprintln!("  This is unusual - the AI should generate valid messages.");
                eprintln!("  Try again or report this issue.");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parses_with_defaults() {
        let cli = Cli::parse_from(["commitment"]);
        assert!(cli.command.is_none());
        assert_eq!(cli.agent, "claude");
        assert!(!cli.dry_run);
        assert!(!cli.message_only);
        assert!(!cli.quiet);
        assert!(!cli.verbose);
        assert_eq!(cli.cwd, PathBuf::from("."));
    }

    #[test]
    fn cli_parses_agent_flag() {
        let cli = Cli::parse_from(["commitment", "--agent", "codex"]);
        assert_eq!(cli.agent, "codex");
    }

    #[test]
    fn cli_parses_dry_run_flag() {
        let cli = Cli::parse_from(["commitment", "--dry-run"]);
        assert!(cli.dry_run);
    }

    #[test]
    fn cli_parses_message_only_flag() {
        let cli = Cli::parse_from(["commitment", "--message-only"]);
        assert!(cli.message_only);
    }

    #[test]
    fn cli_parses_quiet_flag() {
        let cli = Cli::parse_from(["commitment", "-q"]);
        assert!(cli.quiet);
    }

    #[test]
    fn cli_parses_verbose_flag() {
        let cli = Cli::parse_from(["commitment", "-v"]);
        assert!(cli.verbose);
    }

    #[test]
    fn cli_parses_cwd_flag() {
        let cli = Cli::parse_from(["commitment", "--cwd", "/tmp"]);
        assert_eq!(cli.cwd, PathBuf::from("/tmp"));
    }

    #[test]
    fn cli_parses_init_command() {
        let cli = Cli::parse_from(["commitment", "init"]);
        assert!(matches!(cli.command, Some(Command::Init { .. })));
    }

    #[test]
    fn cli_parses_init_with_hook_manager() {
        let cli = Cli::parse_from(["commitment", "init", "--hook-manager", "lefthook"]);
        match cli.command {
            Some(Command::Init { hook_manager, .. }) => {
                assert_eq!(hook_manager, Some("lefthook".to_string()));
            }
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn cli_parses_init_with_agent() {
        let cli = Cli::parse_from(["commitment", "init", "--agent", "gemini"]);
        match cli.command {
            Some(Command::Init { agent, .. }) => {
                assert_eq!(agent, "gemini");
            }
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn cli_parses_combined_flags() {
        let cli = Cli::parse_from([
            "commitment",
            "--agent",
            "codex",
            "--dry-run",
            "--quiet",
            "--cwd",
            "/tmp",
        ]);
        assert_eq!(cli.agent, "codex");
        assert!(cli.dry_run);
        assert!(cli.quiet);
        assert_eq!(cli.cwd, PathBuf::from("/tmp"));
    }

    #[test]
    fn generate_args_construction() {
        let args = GenerateArgs {
            agent: "claude".to_string(),
            dry_run: true,
            message_only: false,
            quiet: false,
            verbose: true,
            cwd: PathBuf::from("."),
        };
        assert_eq!(args.agent, "claude");
        assert!(args.dry_run);
        assert!(!args.message_only);
        assert!(!args.quiet);
        assert!(args.verbose);
    }
}
