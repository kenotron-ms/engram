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

fn run_auth_add_onedrive(folder: &str) {
    use engram_sync::auth::AuthStore;
    use std::io::{self, Write};

    // Microsoft Identity platform — Azure CLI public client ID (public, no secret required)
    let client_id = "04b07795-8ddb-461a-bbee-02f9e1bf7b46";
    let auth_url = format!(
        "https://login.microsoftonline.com/common/oauth2/v2.0/authorize?\
         client_id={}&response_type=code\
         &redirect_uri=https://login.microsoftonline.com/common/oauth2/nativeclient\
         &scope=Files.ReadWrite%20offline_access&response_mode=query",
        client_id
    );

    println!("Opening browser for Microsoft authentication...");
    println!("If browser doesn't open, visit:\n{}", auth_url);
    open::that(&auth_url).ok();

    print!("\nPaste the authorization code from the redirect URL: ");
    io::stdout().flush().unwrap();
    let mut code = String::new();
    io::stdin().read_line(&mut code).unwrap();
    let code = code.trim().to_string();

    // Exchange authorization code for tokens
    let client = reqwest::blocking::Client::new();
    let response = client
        .post("https://login.microsoftonline.com/common/oauth2/v2.0/token")
        .form(&[
            ("client_id", client_id),
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", "https://login.microsoftonline.com/common/oauth2/nativeclient"),
            ("scope", "Files.ReadWrite offline_access"),
        ])
        .send()
        .expect("Token exchange request failed");

    let json: serde_json::Value = response.json().expect("Invalid token response");
    let access_token = json["access_token"]
        .as_str()
        .expect("No access_token in response");
    let refresh_token = json["refresh_token"].as_str().unwrap_or("");

    AuthStore::store("onedrive", "access_token", access_token).unwrap();
    AuthStore::store("onedrive", "refresh_token", refresh_token).unwrap();
    AuthStore::store("onedrive", "folder", folder).unwrap();

    println!("\u{2713} OneDrive backend configured");
    println!("  Folder: {}", folder);
}

fn run_auth_add_azure(account: &str, container: &str) {
    use engram_sync::auth::AuthStore;
    use std::io::{self, Write};

    print!("Azure Storage access key: ");
    io::stdout().flush().unwrap();
    let ak = rpassword::prompt_password("Access key: ").unwrap_or_default();

    AuthStore::store("azure", "account", account).unwrap();
    AuthStore::store("azure", "container", container).unwrap();
    AuthStore::store("azure", "access_key", &ak).unwrap();

    println!("\u{2713} Azure backend configured");
    println!("  Account:   {}", account);
    println!("  Container: {}", container);
}

fn run_auth_add_gdrive(bucket: &str, key_file: &str) {
    use engram_sync::auth::AuthStore;

    AuthStore::store("gcs", "bucket", bucket).unwrap();
    AuthStore::store("gcs", "key_file", key_file).unwrap();

    println!("\u{2713} GCS backend configured");
    println!("  Bucket:   {}", bucket);
    println!("  Key file: {}", key_file);
}

fn run_auth_list() {
    use engram_sync::auth::AuthStore;

    // (backend_name, required_keys_for_is_configured, display_keys_non_sensitive)
    let backends: &[(&str, &[&str], &[&str])] = &[
        ("s3",       &["access_key", "secret_key", "endpoint", "bucket"], &["endpoint", "bucket"]),
        ("onedrive", &["access_token", "folder"],                         &["folder"]),
        ("azure",    &["account", "container"],                           &["account", "container"]),
        ("gcs",      &["bucket", "key_file"],                             &["bucket", "key_file"]),
    ];

    println!("{}", "─".repeat(41));
    println!("Configured sync backends:");
    println!();

    let mut any_configured = false;
    for (backend, required, display_keys) in backends {
        if AuthStore::is_configured(backend, required) {
            let details = display_keys
                .iter()
                .filter_map(|k| {
                    AuthStore::retrieve(backend, k)
                        .ok()
                        .map(|v| format!("{}={}", k, v))
                })
                .collect::<Vec<_>>()
                .join(", ");
            println!("  ✓ {} ({})", backend, details);
            any_configured = true;
        }
    }

    if !any_configured {
        println!("  No backends configured.");
        println!();
        println!("  Run: engram auth add s3|onedrive|azure|gdrive");
    }
    println!();
}

fn run_auth_remove(backend: &str) {
    use engram_sync::auth::AuthStore;
    use std::collections::HashMap;

    let keys_by_backend: HashMap<&str, &[&str]> = [
        ("s3",       ["access_key", "secret_key", "endpoint", "bucket"].as_slice()),
        ("onedrive", ["access_token", "refresh_token", "folder"].as_slice()),
        ("azure",    ["account", "access_key", "container"].as_slice()),
        ("gcs",      ["bucket", "key_file"].as_slice()),
    ]
    .into_iter()
    .collect();

    match keys_by_backend.get(backend) {
        None => {
            eprintln!("Unknown backend: {}. Valid options: s3, onedrive, azure, gcs", backend);
            std::process::exit(1);
        }
        Some(keys) => {
            let removed = keys
                .iter()
                .filter(|k| AuthStore::delete(backend, k).is_ok())
                .count();
            if removed > 0 {
                println!("✓ Removed {} backend credentials", backend);
            } else {
                println!("No credentials found for {}", backend);
            }
        }
    }
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
