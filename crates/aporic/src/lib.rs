//! Local-first continuity and agent-system hub for Aporic.

pub mod context;
pub mod domain;
pub mod eval;
pub mod git;
pub mod hook;
pub mod hub;
pub mod kernel;
pub mod mcp;
pub mod recovery;
pub mod runner;
pub mod store;

mod bounded;
mod git_process;

use std::path::PathBuf;

pub use hook::read_hook_input;
pub use hub::Hub;
pub use mcp::AporicMcp;

/// Resolves the platform-native database path without placing runtime state in
/// either a governed workspace or the Codex configuration directory.
pub fn default_database_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("APORIC_DATABASE") {
        return Ok(PathBuf::from(path));
    }

    directories::ProjectDirs::from("dev", "Aporic", "Aporic")
        .map(|directories| directories.data_dir().join("aporic.sqlite3"))
        .ok_or_else(|| "could not resolve the Aporic data directory".to_owned())
}
