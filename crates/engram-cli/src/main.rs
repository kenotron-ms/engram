// engram-cli — Personal memory assistant CLI

use clap::{Parser, Subcommand};
use directories::UserDirs;
use engram_core::{crypto::KeyStore, store::MemoryStore, vault::Vault};
use std::path::PathBuf;

// Pull in the engram-sync crate so its modules (e.g. auth) are accessible.
#[allow(unused_imports)]
use engram_sync;

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
    /// Manage sync backend authentication
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },
    /// Sync vault with configured backend
    Sync {
        /// Force a specific backend (s3, onedrive, azure, gcs)
        #[arg(long)]
        backend: Option<String>,
    },
}

#[derive(Subcommand)]
enum AuthCommands {
    /// Configure a sync backend (stores credentials in keychain)
    Add {
        #[command(subcommand)]
        backend: BackendCommands,
    },
    /// List configured sync backends
    List,
    /// Remove a backend's credentials from the keychain
    Remove { backend: String },
}

#[derive(Subcommand)]
enum BackendCommands {
    /// S3-compatible storage (AWS S3, Cloudflare R2, MinIO, Backblaze B2)
    S3 {
        #[arg(long)]
        endpoint: String,
        #[arg(long)]
        bucket: String,
        /// If omitted, prompts interactively
        #[arg(long)]
        access_key: Option<String>,
        /// If omitted, prompts securely (no echo)
        #[arg(long)]
        secret_key: Option<String>,
    },
    /// Microsoft OneDrive (OAuth2 browser flow)
    Onedrive {
        #[arg(long, default_value = "/Apps/Engram/vault")]
        folder: String,
    },
    /// Azure Blob Storage
    Azure {
        #[arg(long)]
        account: String,
        #[arg(long)]
        container: String,
    },
    /// Google Cloud Storage
    Gdrive {
        #[arg(long)]
        bucket: String,
        #[arg(long)]
        key_file: String,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => run_status(),
        Commands::Auth { command } => match command {
            AuthCommands::Add { backend } => match backend {
                BackendCommands::S3 {
                    endpoint,
                    bucket,
                    access_key,
                    secret_key,
                } => {
                    run_auth_add_s3(
                        &endpoint,
                        &bucket,
                        access_key.as_deref(),
                        secret_key.as_deref(),
                    );
                }
                BackendCommands::Onedrive { folder } => {
                    run_auth_add_onedrive(&folder);
                }
                BackendCommands::Azure { account, container } => {
                    run_auth_add_azure(&account, &container);
                }
                BackendCommands::Gdrive { bucket, key_file } => {
                    run_auth_add_gdrive(&bucket, &key_file);
                }
            },
            AuthCommands::List => run_auth_list(),
            AuthCommands::Remove { backend } => run_auth_remove(&backend),
        },
        Commands::Sync { backend } => run_sync(backend.as_deref()),
    }
}

fn run_auth_add_s3(
    endpoint: &str,
    bucket: &str,
    access_key: Option<&str>,
    secret_key: Option<&str>,
) {
    use engram_sync::auth::AuthStore;
    use std::io::{self, Write};

    let ak = access_key
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            print!("Access key ID: ");
            io::stdout().flush().unwrap();
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            input.trim().to_string()
        });

    let sk = secret_key
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            rpassword::prompt_password("Secret access key: ").unwrap_or_default()
        });

    AuthStore::store("s3", "access_key", &ak).unwrap();
    AuthStore::store("s3", "secret_key", &sk).unwrap();
    AuthStore::store("s3", "endpoint", endpoint).unwrap();
    AuthStore::store("s3", "bucket", bucket).unwrap();

    println!("\u{2713} S3 backend configured");
    println!("  Endpoint: {}", endpoint);
    println!("  Bucket:   {}", bucket);
}

fn run_auth_add_onedrive(_folder: &str) {
    todo!("implemented in Task 9")
}

fn run_auth_add_azure(_account: &str, _container: &str) {
    todo!("implemented in Task 9")
}

fn run_auth_add_gdrive(_bucket: &str, _key_file: &str) {
    todo!("implemented in Task 9")
}

fn run_auth_list() {
    todo!("implemented in Task 10")
}

fn run_auth_remove(_backend: &str) {
    todo!("implemented in Task 10")
}

fn run_sync(_backend: Option<&str>) {
    todo!("implemented in Task 11")
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
    println!("{}", "\u{2500}".repeat(41));

    // ── Vault status ──────────────────────────────────────────────────────────
    let vault_path = default_vault_path();
    if vault_path.exists() {
        let vault = Vault::new(&vault_path);
        let count = vault.list_markdown().map(|files| files.len()).unwrap_or(0);
        println!("Vault:        {} ({} files)", vault_path.display(), count);
    } else {
        println!("Vault:        {} (NOT FOUND)", vault_path.display());
    }

    // ── Memory store status ───────────────────────────────────────────────────
    let store_path = default_store_path();
    let key_store = KeyStore::new("engram");
    let key_result = key_store.retrieve();

    if store_path.exists() {
        match &key_result {
            Ok(key) => match MemoryStore::open(&store_path, key) {
                Ok(store) => {
                    let count = store.record_count().unwrap_or(0);
                    println!(
                        "Memory store: {} (present, {} records)",
                        store_path.display(),
                        count
                    );
                }
                Err(_) => {
                    println!("Memory store: {} (wrong key)", store_path.display());
                }
            },
            Err(_) => {
                println!("Memory store: {} (present, no key)", store_path.display());
            }
        }
    } else {
        println!("Memory store: {} (not initialized)", store_path.display());
    }

    // ── Keyring status ────────────────────────────────────────────────────────
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
