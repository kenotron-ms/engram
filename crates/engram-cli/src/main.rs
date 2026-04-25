// engram-cli — Personal memory assistant CLI

use clap::{Parser, Subcommand};
use directories::UserDirs;
use engram_core::{crypto::KeyStore, store::MemoryStore, vault::Vault};
use std::path::PathBuf;

/// Personal memory assistant
#[derive(Parser)]
#[command(name = "engram", about = "Personal memory assistant")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Print vault state, memory store stats, and keyring status
    Status,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => run_status(),
    }
}

/// Returns the default vault path: `~/.lifeos/memory`.
fn default_vault_path() -> PathBuf {
    UserDirs::new()
        .map(|u| u.home_dir().join(".lifeos/memory"))
        .unwrap_or_else(|| PathBuf::from(".lifeos/memory"))
}

/// Returns the default memory store path: `~/.engram/memory.db`.
fn default_store_path() -> PathBuf {
    UserDirs::new()
        .map(|u| u.home_dir().join(".engram/memory.db"))
        .unwrap_or_else(|| PathBuf::from(".engram/memory.db"))
}

/// Print vault state, memory store stats, and keyring status to stdout.
fn run_status() {
    // Separator line
    println!("{}", "─".repeat(41));

    // ── Vault status ────────────────────────────────────────────────────────
    let vault_path = default_vault_path();
    if vault_path.exists() {
        let vault = Vault::new(&vault_path);
        let count = vault.list_markdown().map(|files| files.len()).unwrap_or(0);
        println!("Vault:        {} ({} files)", vault_path.display(), count);
    } else {
        println!("Vault:        {} (NOT FOUND)", vault_path.display());
    }

    // ── Memory store status ─────────────────────────────────────────────────
    let store_path = default_store_path();
    let key_store = KeyStore::new("engram");
    let key_result = key_store.retrieve();

    if store_path.exists() {
        match &key_result {
            Ok(key) => {
                match MemoryStore::open(&store_path, key) {
                    Ok(store) => {
                        let count = store.record_count().unwrap_or(0);
                        println!(
                            "Memory store: {} (present, {} records)",
                            store_path.display(),
                            count
                        );
                    }
                    Err(_) => {
                        println!(
                            "Memory store: {} (wrong key)",
                            store_path.display()
                        );
                    }
                }
            }
            Err(_) => {
                println!(
                    "Memory store: {} (present, no key)",
                    store_path.display()
                );
            }
        }
    } else {
        println!(
            "Memory store: {} (not initialized)",
            store_path.display()
        );
    }

    // ── Keyring status ──────────────────────────────────────────────────────
    match key_result {
        Ok(_) => println!("Key:          present \u{2713}"),
        Err(_) => println!("Key:          not set"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_vault_path_ends_with_lifeos_memory() {
        let path = default_vault_path();
        let path_str = path.to_string_lossy();
        assert!(
            path_str.ends_with(".lifeos/memory"),
            "vault path should end with .lifeos/memory, got: {}",
            path_str
        );
    }

    #[test]
    fn test_default_store_path_ends_with_engram_memory_db() {
        let path = default_store_path();
        let path_str = path.to_string_lossy();
        assert!(
            path_str.ends_with(".engram/memory.db"),
            "store path should end with .engram/memory.db, got: {}",
            path_str
        );
    }
}
