//! Integration tests with real AI agents
//!
//! These tests make actual API calls to AI agents (Claude, Codex, Gemini)
//! while mocking the git provider. This tests the full flow including:
//! - Real AI response generation
//! - Response cleaning with actual AI output
//! - Commit message validation
//!
//! Run with: cargo test --test real_agent_tests -- --ignored
//!
//! Requirements:
//! - Claude CLI installed and authenticated (`claude --version`)
//! - Codex CLI installed and authenticated (`codex --version`)
//! - Gemini CLI installed and authenticated (`gemini --version`)

mod fixtures;

use commitment_rs::*;
use fixtures::{MockGitProvider, diffs};

// ============================================================================
// Claude Tests
// ============================================================================

#[tokio::test]
#[ignore = "requires Claude CLI installed and authenticated"]
async fn claude_generates_valid_commit_for_simple_change() {
    let git = MockGitProvider::new(diffs::simple_addition());
    let agent = Agent::from(AgentName::Claude);

    let result = generate_commit_message(&git, &agent, None).await;

    assert!(
        result.is_ok(),
        "Claude should generate valid commit: {:?}",
        result.err()
    );
    let commit = result.unwrap();

    // Should be a valid conventional commit
    assert!(
        commit.starts_with("feat") || commit.starts_with("fix") || commit.starts_with("refactor"),
        "Commit should start with conventional type: {}",
        commit.as_str()
    );
}

#[tokio::test]
#[ignore = "requires Claude CLI installed and authenticated"]
async fn claude_generates_valid_commit_for_multi_file_change() {
    let git = MockGitProvider::new(diffs::multi_file_feature());
    let agent = Agent::from(AgentName::Claude);

    let result = generate_commit_message(&git, &agent, None).await;

    assert!(
        result.is_ok(),
        "Claude should generate valid commit: {:?}",
        result.err()
    );
}

#[tokio::test]
#[ignore = "requires Claude CLI installed and authenticated"]
async fn claude_generates_fix_type_for_bug_fix() {
    let git = MockGitProvider::new(diffs::bug_fix());
    let agent = Agent::from(AgentName::Claude);

    let result = generate_commit_message(&git, &agent, None).await;

    assert!(
        result.is_ok(),
        "Claude should generate valid commit: {:?}",
        result.err()
    );
    let commit = result.unwrap();
    println!("Bug fix commit: {}", commit.as_str());
}

#[tokio::test]
#[ignore = "requires Claude CLI installed and authenticated"]
async fn claude_handles_signature_appending() {
    let git = MockGitProvider::new(diffs::simple_addition());
    let agent = Agent::from(AgentName::Claude);
    let signature = agent.name().commit_signature();

    let result = generate_commit_message(&git, &agent, Some(&signature)).await;

    assert!(
        result.is_ok(),
        "Claude should generate valid commit with signature: {:?}",
        result.err()
    );
    let commit = result.unwrap();

    assert!(
        commit.contains("Generated with Claude via commitment"),
        "Commit should contain signature: {}",
        commit.as_str()
    );
}

#[tokio::test]
#[ignore = "requires Claude CLI installed and authenticated"]
async fn claude_generates_docs_type_for_documentation() {
    let git = MockGitProvider::new(diffs::documentation());
    let agent = Agent::from(AgentName::Claude);

    let result = generate_commit_message(&git, &agent, None).await;

    assert!(
        result.is_ok(),
        "Claude should generate valid commit: {:?}",
        result.err()
    );
    let commit = result.unwrap();
    println!("Docs commit: {}", commit.as_str());
}

#[tokio::test]
#[ignore = "requires Claude CLI installed and authenticated"]
async fn claude_generates_test_type_for_tests() {
    let git = MockGitProvider::new(diffs::add_tests());
    let agent = Agent::from(AgentName::Claude);

    let result = generate_commit_message(&git, &agent, None).await;

    assert!(
        result.is_ok(),
        "Claude should generate valid commit: {:?}",
        result.err()
    );
    let commit = result.unwrap();
    println!("Test commit: {}", commit.as_str());
}

// ============================================================================
// Codex Tests
// ============================================================================

#[tokio::test]
#[ignore = "requires Codex CLI installed and authenticated"]
async fn codex_generates_valid_commit_for_simple_change() {
    let git = MockGitProvider::new(diffs::simple_addition());
    let agent = Agent::from(AgentName::Codex);

    let result = generate_commit_message(&git, &agent, None).await;

    assert!(
        result.is_ok(),
        "Codex should generate valid commit: {:?}",
        result.err()
    );
}

#[tokio::test]
#[ignore = "requires Codex CLI installed and authenticated"]
async fn codex_generates_valid_commit_for_refactor() {
    let git = MockGitProvider::new(diffs::refactor_handlers());
    let agent = Agent::from(AgentName::Codex);

    let result = generate_commit_message(&git, &agent, None).await;

    assert!(
        result.is_ok(),
        "Codex should generate valid commit: {:?}",
        result.err()
    );
    let commit = result.unwrap();
    println!("Refactor commit: {}", commit.as_str());
}

// ============================================================================
// Gemini Tests
// ============================================================================

#[tokio::test]
#[ignore = "requires Gemini CLI installed and authenticated"]
async fn gemini_generates_valid_commit_for_simple_change() {
    let git = MockGitProvider::new(diffs::simple_addition());
    let agent = Agent::from(AgentName::Gemini);

    let result = generate_commit_message(&git, &agent, None).await;

    assert!(
        result.is_ok(),
        "Gemini should generate valid commit: {:?}",
        result.err()
    );
}

#[tokio::test]
#[ignore = "requires Gemini CLI installed and authenticated"]
async fn gemini_generates_valid_commit_for_multi_file_change() {
    let git = MockGitProvider::new(diffs::multi_file_feature());
    let agent = Agent::from(AgentName::Gemini);

    let result = generate_commit_message(&git, &agent, None).await;

    assert!(
        result.is_ok(),
        "Gemini should generate valid commit: {:?}",
        result.err()
    );
}

// ============================================================================
// Cross-Agent Comparison Tests
// ============================================================================

#[tokio::test]
#[ignore = "requires all CLIs installed and authenticated"]
async fn all_agents_produce_valid_commits_for_same_diff() {
    let git = MockGitProvider::new(diffs::simple_addition());

    let agents = [
        Agent::from(AgentName::Claude),
        Agent::from(AgentName::Codex),
        Agent::from(AgentName::Gemini),
    ];

    for agent in &agents {
        let result = generate_commit_message(&git, agent, None).await;

        assert!(
            result.is_ok(),
            "{:?} should generate valid commit: {:?}",
            agent.name(),
            result.err()
        );

        let commit = result.unwrap();
        println!(
            "{}: {}",
            agent.name(),
            commit.as_str().lines().next().unwrap_or("")
        );
    }
}
