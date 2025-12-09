pub mod managers;

// Hooks module - stubs for Phase 1

// HookManager enum placeholder
pub enum HookManager {
    Lefthook,
    Husky,
    SimpleGitHooks,
    PlainGit,
}

impl HookManager {
    pub fn detect() -> Option<Self> {
        todo!("implement in Phase 5")
    }

    pub fn install(&self) -> Result<(), Box<dyn std::error::Error>> {
        todo!("implement in Phase 5")
    }
}
