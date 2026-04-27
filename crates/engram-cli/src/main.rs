// engram-cli — Personal memory assistant CLI

mod awareness;
mod daemon;
mod install;
mod load;
mod mcp;
mod observe;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use clap::{Parser, Subcommand, ValueEnum};
use directories::UserDirs;
use engram_core::config::{EngramConfig, SyncMode, VaultAccess, VaultEntry, VaultSyncCredentials};
use engram_core::{store::MemoryStore, vault::Vault};
use engram_search::indexer::TantivyIndexer;
use engram_search::{SearchResult, SearchSource};
use std::path::{Path, PathBuf};

/// Search mode for the `search` subcommand.
#[derive(Clone, Debug, ValueEnum)]
enum SearchMode {
    /// Full-text BM25 search only
    Fulltext,
    /// Semantic vector (KNN) search only
    Vector,
    /// Hybrid search: RRF merge of full-text and vector results
    Hybrid,
}

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
    /// Initialise the vault: generate salt, prompt for passphrase, write config
    Init,
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
        /// Vault name to sync (defaults to the configured default vault)
        #[arg(long)]
        vault: Option<String>,
        /// Auto-approve sync changes without interactive review
        #[arg(long)]
        approve: bool,
    },
    /// Index vault markdown files for full-text search
    Index {
        /// Vault name (defaults to the configured default vault)
        #[arg(long)]
        vault: Option<String>,
        /// Force a full reindex by wiping the search index first
        #[arg(long)]
        force: bool,
    },
    /// Search the indexed vault
    Search {
        /// Query string
        query: String,
        /// Vault name to search (defaults to the configured default vault)
        #[arg(long)]
        vault: Option<String>,
        /// Maximum number of results to return
        #[arg(long, default_value_t = 10)]
        limit: usize,
        /// Search mode: fulltext (BM25), vector (KNN), or hybrid (RRF merge)
        #[arg(long, default_value = "hybrid")]
        mode: SearchMode,
    },
    /// Observe a session transcript and extract facts into memory
    Observe {
        /// Path to the session transcript file (JSONL)
        #[arg(value_name = "session-path")]
        session_path: PathBuf,
        /// Anthropic API key for LLM fact extraction
        #[arg(long, env = "ANTHROPIC_API_KEY")]
        api_key: Option<String>,
    },
    /// Load recent memories as an AI context block
    Load {
        /// Output format (context)
        #[arg(long, default_value = "context")]
        format: String,
    },
    /// Watch ~/.amplifier/projects for new session transcripts and process them automatically
    Daemon,
    /// Start the MCP stdio server (JSON-RPC 2.0 over stdin/stdout)
    Mcp,
    /// Install the engram daemon as a system service
    Install,
    /// Uninstall the engram daemon system service
    Uninstall,
    /// Run diagnostics on the engram installation
    Doctor,
    /// Show vault domain structure as an AI context block
    Awareness {
        /// Vault name or path (defaults to all configured vaults)
        #[arg(long)]
        vault: Option<String>,
        /// Show all vaults including inactive ones
        #[arg(long)]
        all: bool,
    },
    /// Manage vault configuration
    Vault {
        #[command(subcommand)]
        command: VaultCommands,
    },
}

#[derive(Subcommand)]
enum VaultCommands {
    /// List configured vaults
    List,
    /// Add a vault to the configuration
    Add {
        /// Name for the vault
        name: String,
        /// Filesystem path to the vault directory
        #[arg(long)]
        path: PathBuf,
        /// Access mode (read or read-write)
        #[arg(long, default_value = "read-write")]
        access: String,
        /// Sync mode (auto, approval, or manual)
        #[arg(long, default_value = "approval")]
        sync_mode: String,
        /// Set this vault as the default
        #[arg(long)]
        default: bool,
        /// Optional vault type tag
        #[arg(long)]
        vault_type: Option<String>,
    },
    /// Remove a vault from the configuration
    Remove {
        /// Name of the vault to remove
        name: String,
    },
    /// Set the default vault
    SetDefault {
        /// Name of the vault to make default
        name: String,
    },
}

#[derive(Subcommand)]
enum AuthCommands {
    /// Configure a sync backend (stores credentials in the credentials file)
    Add {
        #[command(subcommand)]
        backend: BackendCommands,
    },
    /// List configured sync backends
    List,
    /// Remove sync credentials for a vault from the credentials file
    Remove { vault: String },
}

#[derive(Subcommand)]
enum BackendCommands {
    /// S3-compatible storage (AWS S3, Cloudflare R2, MinIO, Backblaze B2)
    S3 {
        /// Vault name to configure (defaults to the configured default vault)
        #[arg(long, default_value = "")]
        vault: String,
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
        /// Vault name to configure (defaults to the configured default vault)
        #[arg(long, default_value = "")]
        vault: String,
        #[arg(long)]
        account: String,
        #[arg(long)]
        container: String,
    },
    /// Google Cloud Storage
    Gdrive {
        /// Vault name to configure (defaults to the configured default vault)
        #[arg(long, default_value = "")]
        vault: String,
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
        Commands::Init => run_init(),
        Commands::Auth { command } => match command {
            AuthCommands::Add { backend } => match backend {
                BackendCommands::S3 {
                    vault,
                    endpoint,
                    bucket,
                    access_key,
                    secret_key,
                } => {
                    run_auth_add_s3(
                        &vault,
                        &endpoint,
                        &bucket,
                        access_key.as_deref(),
                        secret_key.as_deref(),
                    );
                }
                BackendCommands::Onedrive { folder } => {
                    run_auth_add_onedrive(&folder);
                }
                BackendCommands::Azure {
                    vault,
                    account,
                    container,
                } => {
                    run_auth_add_azure(&vault, &account, &container);
                }
                BackendCommands::Gdrive {
                    vault,
                    bucket,
                    key_file,
                } => {
                    run_auth_add_gdrive(&vault, &bucket, &key_file);
                }
            },
            AuthCommands::List => run_auth_list(),
            AuthCommands::Remove { vault } => run_auth_remove(&vault),
        },
        Commands::Sync {
            backend,
            vault,
            approve,
        } => run_sync(backend.as_deref(), vault.as_deref(), approve),
        Commands::Index { vault, force } => run_index(vault.as_deref(), force),
        Commands::Search {
            query,
            vault,
            limit,
            mode,
        } => run_search(&query, vault.as_deref(), limit, &mode),
        Commands::Observe {
            session_path,
            api_key,
        } => run_observe(&session_path, api_key.as_deref()),
        Commands::Load { format } => run_load(&format),
        Commands::Daemon => run_daemon(),
        Commands::Mcp => run_mcp(),
        Commands::Install => run_install(),
        Commands::Uninstall => run_uninstall(),
        Commands::Doctor => run_doctor(),
        Commands::Awareness { vault, all } => run_awareness(vault.as_deref(), all),
        Commands::Vault { command } => match command {
            VaultCommands::List => run_vault_list(),
            VaultCommands::Add {
                name,
                path,
                access,
                sync_mode,
                default,
                vault_type,
            } => run_vault_add(
                &name,
                &path,
                &access,
                &sync_mode,
                default,
                vault_type.as_deref(),
            ),
            VaultCommands::Remove { name } => run_vault_remove(&name),
            VaultCommands::SetDefault { name } => run_vault_set_default(&name),
        },
    }
}

/// Resolve the vault encryption key using a three-tier fallback strategy.
///
/// Tier 1 — `ENGRAM_VAULT_KEY` env var: base64-encoded 32 bytes decoded directly
///   into an [`engram_core::crypto::EngramKey`].
/// Tier 2 — `ENGRAM_VAULT_PASSPHRASE` env var + salt from config: the passphrase is
///   derived using Argon2id with the salt stored in the engram config file.
/// Tier 3 — Interactive `rpassword` prompt + salt from config.
///
/// Never panics. Returns a human-friendly `Err(String)` on failure.
fn resolve_vault_key() -> Result<engram_core::crypto::EngramKey, String> {
    // ── Tier 1: ENGRAM_VAULT_KEY env var ─────────────────────────────────────
    if let Ok(encoded) = std::env::var("ENGRAM_VAULT_KEY") {
        let bytes = B64
            .decode(&encoded)
            .map_err(|e| format!("ENGRAM_VAULT_KEY: invalid base64: {}", e))?;
        let key_bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| "ENGRAM_VAULT_KEY must decode to exactly 32 bytes".to_string())?;
        return Ok(engram_core::crypto::EngramKey::from_bytes(key_bytes));
    }

    // Helper: load the 16-byte Argon2 salt from the engram config file.
    let load_salt = || -> Option<[u8; 16]> {
        let config = EngramConfig::load();
        let salt_b64 = config.key.salt?;
        let bytes = B64.decode(&salt_b64).ok()?;
        bytes.try_into().ok()
    };

    // ── Tier 2: ENGRAM_VAULT_PASSPHRASE env var + config salt ─────────────────
    if let Ok(passphrase) = std::env::var("ENGRAM_VAULT_PASSPHRASE") {
        let salt =
            load_salt().ok_or_else(|| "No salt found in config. Run: engram init".to_string())?;
        return engram_core::crypto::EngramKey::derive(passphrase.as_bytes(), &salt)
            .map_err(|e| format!("Key derivation failed: {}", e));
    }

    // ── Tier 3: interactive rpassword prompt + config salt ────────────────────
    let salt =
        load_salt().ok_or_else(|| "No salt found in config. Run: engram init".to_string())?;
    let passphrase = rpassword::prompt_password("Vault passphrase: ")
        .map_err(|e| format!("Failed to read passphrase: {}", e))?;
    engram_core::crypto::EngramKey::derive(passphrase.as_bytes(), &salt)
        .map_err(|e| format!("Key derivation failed: {}", e))
}

/// Initialise the vault: generate salt, prompt for passphrase, write config.
fn run_init() {
    use engram_core::crypto::{generate_salt, EngramKey};

    let mut config = EngramConfig::load();

    // Idempotency guard: if a salt already exists, nothing to do.
    if config.key.salt.is_some() {
        println!("Vault already initialized.");
        return;
    }

    // Resolve passphrase: env var or interactive prompt.
    let passphrase = if let Ok(p) = std::env::var("ENGRAM_VAULT_PASSPHRASE") {
        p
    } else {
        // Interactive path: prompt + confirmation, reject empty.
        let first = rpassword::prompt_password("Vault passphrase: ").unwrap_or_else(|e| {
            eprintln!("Failed to read passphrase: {}", e);
            std::process::exit(1);
        });
        if first.is_empty() {
            eprintln!("Passphrase must not be empty.");
            std::process::exit(1);
        }
        let confirm = rpassword::prompt_password("Confirm passphrase: ").unwrap_or_else(|e| {
            eprintln!("Failed to read passphrase confirmation: {}", e);
            std::process::exit(1);
        });
        if first != confirm {
            eprintln!("Passphrases do not match.");
            std::process::exit(1);
        }
        first
    };

    // Generate a fresh random salt.
    let salt = generate_salt();

    // Verify key derivation succeeds before persisting anything.
    if let Err(e) = EngramKey::derive(passphrase.as_bytes(), &salt) {
        eprintln!("Key derivation failed: {}", e);
        std::process::exit(1);
    }

    // Persist: store the base64-encoded salt in the config, then save.
    config.key.salt = Some(B64.encode(salt));
    let config_path = EngramConfig::config_path();
    if let Err(e) = config.save() {
        eprintln!("Failed to save config: {}", e);
        std::process::exit(1);
    }

    println!(
        "✓ Vault initialised. Config written to: {}",
        config_path.display()
    );
    println!("  Tip: set ENGRAM_VAULT_PASSPHRASE to avoid interactive prompts.");
}

fn run_mcp() {
    let store_path = default_store_path();
    let key = match resolve_vault_key() {
        Ok(k) => k,
        Err(e) => {
            eprintln!("Cannot access vault key: {}", e);
            eprintln!("Tip: run `engram init` to set up the vault");
            std::process::exit(1);
        }
    };
    let store = match MemoryStore::open(&store_path, &key) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to open memory store: {}", e);
            std::process::exit(1);
        }
    };
    if let Err(e) = mcp::run_mcp_server(&store) {
        eprintln!("MCP server error: {}", e);
        std::process::exit(1);
    }
}

fn run_auth_add_s3(
    vault_arg: &str,
    endpoint: &str,
    bucket: &str,
    access_key: Option<&str>,
    secret_key: Option<&str>,
) {
    use std::io::{self, Write};

    let ak = access_key.map(|s| s.to_string()).unwrap_or_else(|| {
        print!("Access key ID: ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        input.trim().to_string()
    });

    let sk = secret_key
        .map(|s| s.to_string())
        .unwrap_or_else(|| match rpassword::prompt_password("Secret access key: ") {
            Ok(s) if !s.is_empty() => s,
            Ok(_) => {
                eprintln!("Secret key must not be empty.");
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("Failed to read secret key: {}", e);
                std::process::exit(1);
            }
        });

    let vault_name = resolve_auth_vault_name(vault_arg);
    let config = EngramConfig::load();

    // Verify vault exists in config.
    if config.vaults.get(&vault_name).is_none() {
        eprintln!("Vault '{}' not found in config.", vault_name);
        std::process::exit(1);
    }

    let mut creds = EngramConfig::load_credentials();
    creds.vaults.insert(
        vault_name.clone(),
        VaultSyncCredentials {
            backend: "s3".to_string(),
            endpoint: Some(endpoint.to_string()),
            bucket: Some(bucket.to_string()),
            access_key: Some(ak),
            secret_key: Some(sk),
            ..Default::default()
        },
    );

    if let Err(e) = EngramConfig::save_credentials(&creds) {
        eprintln!("Failed to save credentials: {}", e);
        std::process::exit(1);
    }

    let creds_path = EngramConfig::credentials_path();
    println!("\u{2713} S3 backend configured for vault '{}'", vault_name);
    println!("  Endpoint:    {}", endpoint);
    println!("  Bucket:      {}", bucket);
    println!("  Credentials: {}", creds_path.display());
}

fn run_auth_add_onedrive(folder: &str) {
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
            (
                "redirect_uri",
                "https://login.microsoftonline.com/common/oauth2/nativeclient",
            ),
            ("scope", "Files.ReadWrite offline_access"),
        ])
        .send()
        .unwrap_or_else(|e| {
            eprintln!("Token exchange request failed: {}", e);
            std::process::exit(1);
        });

    let json: serde_json::Value = response.json().unwrap_or_else(|e| {
        eprintln!("Invalid token response: {}", e);
        std::process::exit(1);
    });
    let access_token = json["access_token"].as_str().unwrap_or_else(|| {
        eprintln!("No access_token in response");
        std::process::exit(1);
    });
    let refresh_token = json["refresh_token"].as_str().unwrap_or("").to_string();
    let access_token = access_token.to_string();

    // Resolve the default vault and write credentials to credentials file.
    let vault_name = resolve_auth_vault_name("");
    let config = EngramConfig::load();

    // Verify vault exists in config.
    if config.vaults.get(&vault_name).is_none() {
        eprintln!("Vault '{}' not found in config.", vault_name);
        std::process::exit(1);
    }

    let mut creds = EngramConfig::load_credentials();
    creds.vaults.insert(
        vault_name.clone(),
        VaultSyncCredentials {
            backend: "onedrive".to_string(),
            access_token: Some(access_token),
            refresh_token: Some(refresh_token),
            folder: Some(folder.to_string()),
            ..Default::default()
        },
    );

    if let Err(e) = EngramConfig::save_credentials(&creds) {
        eprintln!("Failed to save credentials: {}", e);
        std::process::exit(1);
    }

    let creds_path = EngramConfig::credentials_path();
    println!(
        "\u{2713} OneDrive backend configured for vault '{}'",
        vault_name
    );
    println!("  Folder:      {}", folder);
    println!("  Credentials: {}", creds_path.display());
}

fn run_auth_add_azure(vault_arg: &str, account: &str, container: &str) {
    let ak = match rpassword::prompt_password("Access key: ") {
        Ok(s) if !s.is_empty() => s,
        Ok(_) => {
            eprintln!("Access key must not be empty.");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Failed to read access key: {}", e);
            std::process::exit(1);
        }
    };

    let vault_name = resolve_auth_vault_name(vault_arg);
    let config = EngramConfig::load();

    // Verify vault exists in config.
    if config.vaults.get(&vault_name).is_none() {
        eprintln!("Vault '{}' not found in config.", vault_name);
        std::process::exit(1);
    }

    let mut creds = EngramConfig::load_credentials();
    creds.vaults.insert(
        vault_name.clone(),
        VaultSyncCredentials {
            backend: "azure".to_string(),
            account: Some(account.to_string()),
            container: Some(container.to_string()),
            access_key: Some(ak),
            ..Default::default()
        },
    );

    if let Err(e) = EngramConfig::save_credentials(&creds) {
        eprintln!("Failed to save credentials: {}", e);
        std::process::exit(1);
    }

    let creds_path = EngramConfig::credentials_path();
    println!(
        "\u{2713} Azure backend configured for vault '{}'",
        vault_name
    );
    println!("  Account:     {}", account);
    println!("  Container:   {}", container);
    println!("  Credentials: {}", creds_path.display());
}

fn run_auth_add_gdrive(vault_arg: &str, bucket: &str, key_file: &str) {
    let vault_name = resolve_auth_vault_name(vault_arg);
    let config = EngramConfig::load();

    // Verify vault exists in config.
    if config.vaults.get(&vault_name).is_none() {
        eprintln!("Vault '{}' not found in config.", vault_name);
        std::process::exit(1);
    }

    let mut creds = EngramConfig::load_credentials();
    creds.vaults.insert(
        vault_name.clone(),
        VaultSyncCredentials {
            backend: "gcs".to_string(),
            bucket: Some(bucket.to_string()),
            // Reuse access_key field for the key file path.
            access_key: Some(key_file.to_string()),
            ..Default::default()
        },
    );

    if let Err(e) = EngramConfig::save_credentials(&creds) {
        eprintln!("Failed to save credentials: {}", e);
        std::process::exit(1);
    }

    let creds_path = EngramConfig::credentials_path();
    println!("\u{2713} GCS backend configured for vault '{}'", vault_name);
    println!("  Bucket:      {}", bucket);
    println!("  Key file:    {}", key_file);
    println!("  Credentials: {}", creds_path.display());
}

fn run_auth_list() {
    let config = EngramConfig::load();
    let creds = EngramConfig::load_credentials();

    println!("{}", "─".repeat(41));
    println!("Vault sync backends:");
    println!();

    if config.vaults.is_empty() {
        println!("  No vaults configured.");
        println!();
        println!("  Run: engram vault add <name> --path <path>");
        println!();
        return;
    }

    let mut any_configured = false;
    for vault_name in config.vaults.keys() {
        if let Some(sync) = EngramConfig::credentials_for_vault(vault_name, &creds) {
            let details = match sync.backend.as_str() {
                "s3" => {
                    let endpoint = sync.endpoint.as_deref().unwrap_or("(none)");
                    let bucket = sync.bucket.as_deref().unwrap_or("(none)");
                    format!("endpoint={}, bucket={}", endpoint, bucket)
                }
                "onedrive" => {
                    let folder = sync.folder.as_deref().unwrap_or("(none)");
                    format!("folder={}", folder)
                }
                "azure" => {
                    let account = sync.account.as_deref().unwrap_or("(none)");
                    let container = sync.container.as_deref().unwrap_or("(none)");
                    format!("account={}, container={}", account, container)
                }
                "gcs" => {
                    let bucket = sync.bucket.as_deref().unwrap_or("(none)");
                    format!("bucket={}", bucket)
                }
                other => format!("backend={}", other),
            };
            println!("  \u{2713} {} \u{2014} {} ({})", vault_name, sync.backend, details);
            any_configured = true;
        } else {
            println!("  \u{00b7} {} \u{2014} no sync configured", vault_name);
        }
    }

    if !any_configured {
        println!();
        println!("  Run: engram auth add s3|onedrive|azure|gdrive --vault <name>");
    }
    println!();
    let creds_path = EngramConfig::credentials_path();
    println!("  Credentials file: {}", creds_path.display());
    println!();
}

fn run_auth_remove(vault_name: &str) {
    // Verify vault exists in config (graceful error if not registered).
    let config = EngramConfig::load();
    if config.vaults.get(vault_name).is_none() {
        eprintln!("Vault '{}' not found in config.", vault_name);
        std::process::exit(1);
    }

    let mut creds = EngramConfig::load_credentials();

    if creds.vaults.remove(vault_name).is_none() {
        println!("No sync credentials configured for vault '{}'", vault_name);
        return;
    }

    if let Err(e) = EngramConfig::save_credentials(&creds) {
        eprintln!("Failed to save credentials: {}", e);
        std::process::exit(1);
    }

    println!("\u{2713} Removed sync credentials for vault '{}'", vault_name);
}

/// Show a formatted git status summary for the vault directory.
///
/// Runs `git -C <vault_path> status --short` and prints each changed file
/// with a human-readable label (modified, new file, deleted, renamed, changed).
/// If the directory is not a git repository or git is unavailable, prints a
/// graceful message instead.
fn show_vault_diff(vault_path: &Path) {
    let output = std::process::Command::new("git")
        .args(["-C", &vault_path.to_string_lossy(), "status", "--short"])
        .output();

    match output {
        Err(_) => {
            println!("  (git not available — cannot show pending changes)");
        }
        Ok(out) if !out.status.success() => {
            println!("  (not a git repository — cannot show pending changes)");
        }
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if stdout.trim().is_empty() {
                println!("  (no pending changes)");
            } else {
                for line in stdout.lines() {
                    let label = if line.starts_with('M') || line.starts_with(" M") {
                        "modified"
                    } else if line.starts_with('A') || line.starts_with("??") {
                        "new file"
                    } else if line.starts_with('D') || line.starts_with(" D") {
                        "deleted"
                    } else if line.starts_with('R') {
                        "renamed"
                    } else {
                        "changed"
                    };
                    let file = line.get(3..).unwrap_or(line).trim_end();
                    println!("  {} {}", label, file);
                }
            }
        }
    }
}

fn run_sync(backend_name: Option<&str>, vault_arg: Option<&str>, approve: bool) {
    // Check write access before any backend logic.
    let vault_name = resolve_vault_name(vault_arg);
    check_write_access(&vault_name);

    // Resolve the vault path (used by the mode gate and the sync backend).
    let vault_path = resolve_vault(vault_arg);

    // ── Sync mode gate ──────────────────────────────────────────────────────
    // Look up the vault's sync_mode from config (if registered).
    let config = EngramConfig::load();
    let sync_mode = config
        .get_vault(&vault_name)
        .map(|e| e.sync_mode.clone())
        .unwrap_or(SyncMode::Auto);

    match (&sync_mode, approve) {
        // Manual mode: print informational message and return early.
        (SyncMode::Manual, _) => {
            println!("This vault uses manual sync mode. Sync is managed externally.");
            return;
        }
        // Approval mode without --approve: show diff and hint, then return.
        (SyncMode::Approval, false) => {
            println!("Pending changes (approval required):");
            show_vault_diff(&vault_path);
            println!();
            println!("To push: run `engram sync --approve` to push these changes.");
            return;
        }
        // Auto mode, or approval + --approve: fall through to sync backend.
        _ => {}
    }
    // ── End sync mode gate ──────────────────────────────────────────────────

    use engram_sync::{
        backend::SyncBackend, encrypt::encrypt_for_sync, onedrive::OneDriveBackend, s3::S3Backend,
    };

    let vault = Vault::new(&vault_path);

    let key = match resolve_vault_key() {
        Ok(k) => k,
        Err(e) => {
            eprintln!("Cannot access vault key: {}", e);
            eprintln!("Tip: set ENGRAM_VAULT_PASSPHRASE or run: engram init");
            std::process::exit(1);
        }
    };

    // Determine which backend to use from credentials file.
    let all_creds = EngramConfig::load_credentials();
    let creds = EngramConfig::credentials_for_vault(&vault_name, &all_creds);

    let creds = match creds {
        Some(c) => c,
        None => {
            eprintln!(
                "No sync backend configured for vault '{}'. Run: engram auth add s3|onedrive|azure|gdrive --vault {}",
                vault_name, vault_name
            );
            std::process::exit(1);
        }
    };

    // Use explicit backend arg, or fall back to configured backend.
    let effective_backend = backend_name.unwrap_or(creds.backend.as_str());

    let backend: Box<dyn SyncBackend> = match effective_backend {
        "s3" => {
            let endpoint = creds.endpoint.as_deref().unwrap_or_default();
            let bucket = creds.bucket.as_deref().unwrap_or_default();
            let ak = creds.access_key.as_deref().unwrap_or_default();
            let sk = creds.secret_key.as_deref().unwrap_or_default();
            match S3Backend::new(endpoint, bucket, ak, sk) {
                Ok(b) => Box::new(b),
                Err(e) => {
                    eprintln!("Failed to initialize S3 backend: {}", e);
                    eprintln!("Check the endpoint URL and credentials via: engram auth add s3 --vault <name>");
                    std::process::exit(1);
                }
            }
        }
        "onedrive" => {
            let token = creds.access_token.as_deref().unwrap_or_default();
            let folder = creds.folder.as_deref().unwrap_or_default();
            Box::new(OneDriveBackend::new(token, folder))
        }
        other => {
            eprintln!(
                "Backend '{}' is not yet supported in engram sync. Use: s3, onedrive",
                other
            );
            std::process::exit(1);
        }
    };

    let files = match vault.list_markdown() {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to list vault files: {}", e);
            std::process::exit(1);
        }
    };

    println!(
        "Syncing {} files via {} ...",
        files.len(),
        effective_backend
    );

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut success = 0usize;
    let mut errors = 0usize;

    for relative_path in &files {
        let content = match vault.read(relative_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("  ✗ {}: {}", relative_path, e);
                errors += 1;
                continue;
            }
        };
        let encrypted = match encrypt_for_sync(&key, content.as_bytes()) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("  ✗ {}: encryption failed — {}", relative_path, e);
                errors += 1;
                continue;
            }
        };
        match runtime.block_on(backend.push(relative_path, encrypted)) {
            Ok(_) => {
                success += 1;
            }
            Err(e) => {
                eprintln!("  ✗ {}: {}", relative_path, e);
                errors += 1;
            }
        }
    }

    println!("{}", "─".repeat(41));
    println!("Pushed:  {} files", success);
    if errors > 0 {
        eprintln!("Errors:  {} files", errors);
        std::process::exit(1);
    }
}

/// Returns the per-vault storage directory: `~/.engram/<vault_name>/`.
///
/// This directory is used to store vault-specific files such as the memory
/// database (`memory.db`).
fn vault_storage_dir(vault_name: &str) -> PathBuf {
    UserDirs::new()
        .map(|u| u.home_dir().join(".engram").join(vault_name))
        .unwrap_or_else(|| PathBuf::from(format!(".engram/{}", vault_name)))
}

/// Expand a leading `~` in `p` to the user's home directory using
/// `shellexpand::tilde`.
fn shellexpand_path(p: &str) -> PathBuf {
    PathBuf::from(shellexpand::tilde(p).as_ref())
}

/// Resolve the active vault path using the priority chain:
///
/// 1. Explicit `name_override` → look up in config; exit 1 if not found.
/// 2. Auto-detected project vault (`.lifeos/memory` in the current working directory).
/// 3. Config default vault (first entry marked `default = true`, or first alphabetically).
/// 4. Hardcoded fallback: `~/.lifeos/memory`.
fn resolve_vault(name_override: Option<&str>) -> PathBuf {
    let config = EngramConfig::load();

    // 1. Explicit name override — must be in config.
    if let Some(name) = name_override {
        match config.get_vault(name) {
            Some(entry) => return entry.path.clone(),
            None => {
                eprintln!("Vault '{}' not found in config", name);
                std::process::exit(1);
            }
        }
    }

    // 2. Auto-detect `.lifeos/memory` in the current working directory.
    if let Ok(cwd) = std::env::current_dir() {
        let project_vault = cwd.join(".lifeos/memory");
        if project_vault.exists() {
            return project_vault;
        }
    }

    // 3. Config default vault.
    if let Some((_, entry)) = config.default_vault() {
        return entry.path.clone();
    }

    // 4. Hardcoded fallback.
    UserDirs::new()
        .map(|u| u.home_dir().join(".lifeos/memory"))
        .unwrap_or_else(|| PathBuf::from(".lifeos/memory"))
}

/// Core logic for `default_vault_path`, accepting a pre-loaded config.
///
/// Extracted so callers that already hold a config can avoid a redundant
/// filesystem read.
fn default_vault_path_from_config(config: &EngramConfig) -> PathBuf {
    if let Some((_, entry)) = config.default_vault() {
        return entry.path.clone();
    }
    UserDirs::new()
        .map(|u| u.home_dir().join(".lifeos/memory"))
        .unwrap_or_else(|| PathBuf::from(".lifeos/memory"))
}

/// Returns the default vault path.
///
/// If the engram config has a default vault registered, that vault's path is
/// used.  Otherwise, falls back to `~/.lifeos/memory`.
///
/// Existing tests rely on the fallback path ending with `.lifeos/memory` when
/// no config file is present (e.g. on a clean CI machine).
#[allow(dead_code)]
fn default_vault_path() -> PathBuf {
    default_vault_path_from_config(&EngramConfig::load())
}

/// Core logic for `default_store_path`, accepting a pre-loaded config.
///
/// Extracted so callers that already hold a config can avoid a redundant
/// filesystem read.
fn default_store_path_from_config(config: &EngramConfig) -> PathBuf {
    if let Ok(p) = std::env::var("ENGRAM_STORE_PATH") {
        return PathBuf::from(p);
    }
    if let Some((name, _)) = config.default_vault() {
        return vault_storage_dir(name).join("memory.db");
    }
    UserDirs::new()
        .map(|u| u.home_dir().join(".engram/memory.db"))
        .unwrap_or_else(|| PathBuf::from(".engram/memory.db"))
}

/// Returns the memory store path.
///
/// Priority:
/// 1. `ENGRAM_STORE_PATH` environment variable — used directly.
/// 2. Config default vault's per-vault storage directory:
///    `~/.engram/<vault_name>/memory.db`.
/// 3. Legacy fallback: `~/.engram/memory.db`.
///
/// Existing tests rely on the fallback path ending with `.engram/memory.db`
/// when no config file is present (e.g. on a clean CI machine).
fn default_store_path() -> PathBuf {
    default_store_path_from_config(&EngramConfig::load())
}

/// Returns the default search index path: `~/.engram/search`.
#[allow(dead_code)]
fn default_search_dir() -> PathBuf {
    UserDirs::new()
        .map(|u| u.home_dir().join(".engram/search"))
        .unwrap_or_else(|| PathBuf::from(".engram/search"))
}

/// Returns the default vector index path: `~/.engram/vectors.db`.
#[allow(dead_code)]
fn default_vectors_path() -> PathBuf {
    UserDirs::new()
        .map(|u| u.home_dir().join(".engram/vectors.db"))
        .unwrap_or_else(|| PathBuf::from(".engram/vectors.db"))
}

/// Resolve the vault name for auth commands.
///
/// - If `vault_arg` is non-empty, return it directly.
/// - Otherwise, load the config and return the default vault name.
/// - Exit 1 with an error message if no default vault is configured.
fn resolve_auth_vault_name(vault_arg: &str) -> String {
    if !vault_arg.is_empty() {
        return vault_arg.to_string();
    }
    let config = EngramConfig::load();
    match config.default_vault() {
        Some((name, _)) => name.to_string(),
        None => {
            eprintln!(
                "No vault specified and no default vault configured. \
                 Use --vault <name> or run: engram vault add <name> --path <path> --default"
            );
            std::process::exit(1);
        }
    }
}

/// Resolve the vault name from an explicit argument, the config default, or the
/// literal fallback `"default"`.
///
/// Priority chain (highest → lowest):
/// 1. `vault_arg` — explicitly provided `--vault <name>` flag value.
/// 2. `EngramConfig::default_vault()` — the vault marked as default in the
///    user's config file.
/// 3. Literal `"default"` — hardcoded last-resort fallback.
fn resolve_vault_name(vault_arg: Option<&str>) -> String {
    vault_arg.map(|s| s.to_string()).unwrap_or_else(|| {
        let config = EngramConfig::load();
        config
            .default_vault()
            .map(|(n, _)| n.to_string())
            .unwrap_or_else(|| "default".to_string())
    })
}

/// Recursively compute the total size in bytes of all files under `path`.
/// Returns 0 if `path` does not exist or cannot be read.
fn dir_size_bytes(path: &Path) -> u64 {
    if !path.exists() {
        return 0;
    }
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                total += dir_size_bytes(&entry_path);
            } else if let Ok(meta) = entry.metadata() {
                total += meta.len();
            }
        }
    }
    total
}

/// Index vault markdown files for full-text search with content-hash deduplication,
/// and embed all files into the sqlite-vec vector store.
fn run_index(vault_arg: Option<&str>, force: bool) {
    use engram_search::embedder::Embedder;
    use engram_search::vector::VectorIndex;

    // Determine the vault name for per-vault storage directories.
    let vault_name = resolve_vault_name(vault_arg);

    // Resolve the actual filesystem path where markdown files live.
    let vault_path = resolve_vault(vault_arg);

    if !vault_path.exists() {
        eprintln!("Vault not found: {}", vault_path.display());
        std::process::exit(1);
    }

    let search_dir = vault_storage_dir(&vault_name).join("search");
    let vectors_path = vault_storage_dir(&vault_name).join("vectors.db");

    // --force: wipe both the search index and vector store so every file is
    // reindexed from scratch.
    if force {
        if search_dir.exists() {
            if let Err(e) = std::fs::remove_dir_all(&search_dir) {
                eprintln!("Failed to wipe search index: {}", e);
                std::process::exit(1);
            }
        }
        if vectors_path.exists() {
            if let Err(e) = std::fs::remove_file(&vectors_path) {
                eprintln!("Failed to wipe vector store: {}", e);
                std::process::exit(1);
            }
        }
    }

    let vault = Vault::new(&vault_path);

    let mut indexer = match TantivyIndexer::open(&search_dir) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Failed to open search index: {}", e);
            std::process::exit(1);
        }
    };

    let stats = match indexer.index_vault(&vault) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Indexing failed: {}", e);
            std::process::exit(1);
        }
    };

    // Vector embedding pass ────────────────────────────────────────────────
    // The vector store has no content-hash deduplication, so we always delete
    // and rebuild it to avoid accumulating duplicate vectors across runs.
    if vectors_path.exists() {
        if let Err(e) = std::fs::remove_file(&vectors_path) {
            eprintln!("Failed to clear vector store: {}", e);
            std::process::exit(1);
        }
    }

    println!("Loading embedding model (first run downloads ~90MB)...");

    let embedder = match Embedder::new() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Failed to load embedding model: {}", e);
            std::process::exit(1);
        }
    };

    let vector_index = match VectorIndex::open(&vectors_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to open vector store: {}", e);
            std::process::exit(1);
        }
    };

    let files = match vault.list_markdown() {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to list vault files: {}", e);
            std::process::exit(1);
        }
    };

    let mut vectors_indexed = 0usize;

    for rel_path in &files {
        let content = match vault.read(rel_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("  \u{2717} {}: read failed \u{2014} {}", rel_path, e);
                continue;
            }
        };
        let embedding = match embedder.embed(&content) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("  \u{2717} {}: embed failed \u{2014} {}", rel_path, e);
                continue;
            }
        };
        if let Err(e) = vector_index.insert(rel_path, &embedding) {
            eprintln!(
                "  \u{2717} {}: vector insert failed \u{2014} {}",
                rel_path, e
            );
        } else {
            vectors_indexed += 1;
        }
    }

    let index_size_mb = dir_size_bytes(&search_dir) as f64 / 1_048_576.0;

    println!("{}", "\u{2500}".repeat(41));
    println!("Indexed:    {}", stats.indexed);
    println!("Skipped:    {}", stats.skipped);
    println!("Total:      {}", stats.total);
    println!("Vectors:    {}", vectors_indexed);
    println!(
        "Index path: {} ({:.2} MB)",
        search_dir.display(),
        index_size_mb
    );
}

/// Search the indexed vault using the specified mode.
fn run_search(query: &str, vault_arg: Option<&str>, limit: usize, mode: &SearchMode) {
    use engram_search::embedder::Embedder;
    use engram_search::hybrid::HybridSearch;
    use engram_search::vector::VectorIndex;

    // Determine the vault name for per-vault storage directories.
    let vault_name = resolve_vault_name(vault_arg);

    let search_dir = vault_storage_dir(&vault_name).join("search");

    // Check search index exists.
    if !search_dir.join("meta.json").exists() {
        eprintln!("Search index not found. Run: engram index");
        std::process::exit(1);
    }

    let indexer = match TantivyIndexer::open(&search_dir) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Failed to open search index: {}", e);
            std::process::exit(1);
        }
    };

    let results: Vec<SearchResult> = match mode {
        SearchMode::Fulltext => match indexer.search(query, limit) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Full-text search failed: {}", e);
                std::process::exit(1);
            }
        },

        SearchMode::Vector => {
            let vectors_path = vault_storage_dir(&vault_name).join("vectors.db");
            let vector_index = match VectorIndex::open(&vectors_path) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Failed to open vector index: {}", e);
                    std::process::exit(1);
                }
            };
            let embedder = match Embedder::new() {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("Failed to load embedder: {}", e);
                    std::process::exit(1);
                }
            };
            let embedding = match embedder.embed(query) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Failed to embed query: {}", e);
                    std::process::exit(1);
                }
            };
            let knn = match vector_index.knn_search(&embedding, limit) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Vector search failed: {}", e);
                    std::process::exit(1);
                }
            };
            knn.into_iter()
                .map(|(path, dist)| SearchResult {
                    path,
                    snippet: String::new(),
                    score: 1.0 - dist,
                    source: SearchSource::Vector,
                })
                .collect()
        }

        SearchMode::Hybrid => {
            let vectors_path = vault_storage_dir(&vault_name).join("vectors.db");
            let vector_index = match VectorIndex::open(&vectors_path) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Failed to open vector index: {}", e);
                    std::process::exit(1);
                }
            };
            let embedder = match Embedder::new() {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("Failed to load embedder: {}", e);
                    std::process::exit(1);
                }
            };
            let hybrid = HybridSearch::new(indexer, vector_index, embedder);
            match hybrid.search(query, limit) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Hybrid search failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
    };

    // Print results header.
    let mode_label = match mode {
        SearchMode::Fulltext => "fulltext",
        SearchMode::Vector => "vector",
        SearchMode::Hybrid => "hybrid",
    };
    println!(
        "Results for \"{}\" [{}] — {} found",
        query,
        mode_label,
        results.len()
    );
    println!("{}", "─".repeat(49));

    if results.is_empty() {
        println!("No results found.");
        return;
    }

    for result in results {
        println!("{} (score: {:.2})", result.path, result.score);
        if !result.snippet.is_empty() {
            println!("  {}", result.snippet);
        }
    }
}

/// Load recent memories and emit them as a context block to stdout.
fn run_load(format: &str) {
    let store_path = default_store_path();
    let key = match resolve_vault_key() {
        Ok(k) => k,
        Err(e) => {
            eprintln!("Cannot access vault key: {}", e);
            eprintln!("Tip: run `engram init` to set up the vault");
            std::process::exit(1);
        }
    };
    let store = match MemoryStore::open(&store_path, &key) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to open memory store: {}", e);
            std::process::exit(1);
        }
    };
    match format {
        "context" => match load::load_context(&store) {
            Ok(output) => print!("{}", output),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        },
        other => {
            eprintln!("Unknown format: {}. Valid formats: context", other);
            std::process::exit(1);
        }
    }
}

/// Check that the named vault allows write access.
///
/// Loads the config and looks up `vault_name`:
/// - If the vault is registered with `access = "read"`, prints a clear error
///   message and exits with code 1.
/// - If the vault is not registered in the config (auto-detected project vault),
///   write access is always permitted.
/// - If the vault is registered with `access = "read-write"`, the function
///   returns normally.
fn check_write_access(vault_name: &str) {
    let config = EngramConfig::load();
    if let Some(entry) = config.get_vault(vault_name) {
        if entry.access == VaultAccess::Read {
            eprintln!(
                "Error: vault '{}' is read-only. \
                 To allow write operations, re-add the vault with: \
                 engram vault add {} --path <path> --access read-write",
                vault_name, vault_name
            );
            std::process::exit(1);
        }
    }
    // Auto-detected project vaults (not in config) always pass the access check.
}

/// Observe a session transcript: parse turns, extract facts via LLM, write to store.
fn run_observe(session_path: &Path, api_key: Option<&str>) {
    // Check write access before anything else (before API key validation).
    // observe uses resolve_vault(None) — resolve the vault name the same way.
    let vault_name = resolve_vault_name(None);
    check_write_access(&vault_name);

    // Resolve API key — required for LLM fact extraction.
    let api_key = match api_key {
        Some(k) if !k.is_empty() => k.to_string(),
        _ => {
            eprintln!(
                "Error: Anthropic API key is required. \
                 Set --api-key or ANTHROPIC_API_KEY environment variable."
            );
            std::process::exit(1);
        }
    };

    // Resolve the vault encryption key.
    let key = match resolve_vault_key() {
        Ok(k) => k,
        Err(e) => {
            eprintln!("Cannot access vault key: {}", e);
            eprintln!("Tip: run `engram init` to set up the vault");
            std::process::exit(1);
        }
    };

    // Open (or create) the memory store.
    let store_path = default_store_path();
    let store = match MemoryStore::open(&store_path, &key) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to open memory store: {}", e);
            std::process::exit(1);
        }
    };

    // Run the full observation pipeline.
    match observe::observe_session(session_path, &store, &api_key) {
        Ok(stats) => {
            println!("Observed:  {}", stats.session_path);
            println!("Extracted: {}", stats.facts_extracted);
            println!("Written:   {}", stats.facts_written);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

/// Build the search index status line for the given `search_dir`.
///
/// Returns one of:
/// - `"Search index: {path} ({count} files indexed, {size:.1} MB)"` when the
///   index exists and can be opened.
/// - `"Search index: {path} (error opening index)"` when meta.json exists but
///   `TantivyIndexer::open` fails.
/// - `"Search index: not built (run: engram index)"` when meta.json is absent.
fn search_index_status(search_dir: &Path) -> String {
    if search_dir.join("meta.json").exists() {
        match TantivyIndexer::open(search_dir) {
            Ok(indexer) => {
                let count = indexer.indexed_doc_count();
                let size_mb = dir_size_bytes(search_dir) as f64 / 1_048_576.0;
                format!(
                    "Search index: {} ({} files indexed, {:.1} MB)",
                    search_dir.display(),
                    count,
                    size_mb
                )
            }
            Err(_) => format!(
                "Search index: {} (error opening index)",
                search_dir.display()
            ),
        }
    } else {
        "Search index: not built (run: engram index)".to_string()
    }
}

/// Watch `~/.amplifier/projects` for session transcript changes and process them automatically.
fn run_daemon() {
    use daemon::watch_sessions;
    use std::sync::mpsc;

    // Resolve the vault encryption key.
    let key = match resolve_vault_key() {
        Ok(k) => k,
        Err(e) => {
            eprintln!("Cannot access vault key: {}", e);
            eprintln!("Tip: run `engram init` to set up the vault");
            std::process::exit(1);
        }
    };

    // Determine the watch directory: ~/.amplifier/projects
    let watch_dir = UserDirs::new()
        .map(|u| u.home_dir().join(".amplifier/projects"))
        .unwrap_or_else(|| PathBuf::from(".amplifier/projects"));

    // Create the watch directory if it doesn't exist.
    if let Err(e) = std::fs::create_dir_all(&watch_dir) {
        eprintln!(
            "Failed to create watch directory {}: {}",
            watch_dir.display(),
            e
        );
        std::process::exit(1);
    }

    // Read ANTHROPIC_API_KEY from environment; warn if empty but continue.
    let api_key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        eprintln!("Warning: ANTHROPIC_API_KEY is not set. Transcripts will be watched but facts cannot be extracted.");
    }

    // Create channel for transcript path events.
    let (tx, rx) = mpsc::channel::<PathBuf>();

    // Start the file watcher.
    let _watcher = match watch_sessions(&watch_dir, tx) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Failed to start file watcher: {}", e);
            std::process::exit(1);
        }
    };

    eprintln!("engram daemon started. Watching: {}", watch_dir.display());

    // Open the memory store for writing extracted facts.
    let store_path = default_store_path();
    let store = match MemoryStore::open(&store_path, &key) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to open memory store: {}", e);
            std::process::exit(1);
        }
    };

    // Block on the channel receiver; process each transcript as it arrives.
    for transcript_path in rx {
        eprintln!("Processing: {}", transcript_path.display());

        if api_key.is_empty() {
            eprintln!("Skipping fact extraction: ANTHROPIC_API_KEY is not set.");
            continue;
        }

        match observe::observe_session(&transcript_path, &store, &api_key) {
            Ok(stats) => {
                eprintln!(
                    "Extracted: {} facts, Written: {} facts from {}",
                    stats.facts_extracted, stats.facts_written, stats.session_path
                );
            }
            Err(e) => {
                eprintln!("Error processing {}: {}", transcript_path.display(), e);
            }
        }
    }
}

/// Install the engram daemon as a system service.
fn run_install() {
    match install::install_service() {
        Ok(()) => println!("\u{2713} engram daemon service installed"),
        Err(e) => {
            eprintln!("Failed to install service: {}", e);
            std::process::exit(1);
        }
    }
}

/// Uninstall the engram daemon system service.
fn run_uninstall() {
    match install::uninstall_service() {
        Ok(()) => println!("\u{2713} engram daemon service uninstalled"),
        Err(e) => {
            eprintln!("Failed to uninstall service: {}", e);
            std::process::exit(1);
        }
    }
}

/// Print diagnostic information about the engram installation.
fn run_doctor() {
    // Load config once to avoid redundant filesystem reads across helpers.
    let config = EngramConfig::load();

    let sep = "\u{2500}".repeat(41);

    println!("{}", sep);
    println!("engram doctor");
    println!("{}", sep);

    // ── Binary path ────────────────────────────────────────────────────────────
    let binary_path = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    println!("Binary:            {}", binary_path);

    // ── Vault status ───────────────────────────────────────────────────────────
    let vault_path = default_vault_path_from_config(&config);
    if vault_path.exists() {
        let vault = Vault::new(&vault_path);
        let count = vault.list_markdown().map(|f| f.len()).unwrap_or(0);
        println!(
            "Vault:             {} ({} files)",
            vault_path.display(),
            count
        );
    } else {
        println!("Vault:             {} (NOT FOUND)", vault_path.display());
    }

    // ── Key method ─────────────────────────────────────────────────────────────
    let key_method = if std::env::var("ENGRAM_VAULT_KEY").is_ok() {
        "ENGRAM_VAULT_KEY"
    } else if std::env::var("ENGRAM_VAULT_PASSPHRASE").is_ok() {
        "ENGRAM_VAULT_PASSPHRASE"
    } else if config.key.salt.is_some() {
        "config salt"
    } else {
        "not initialized"
    };
    println!("Key:               {}", key_method);

    // ── Memory store status ────────────────────────────────────────────────────
    let store_path = default_store_path_from_config(&config);
    let key_result = resolve_vault_key();
    if store_path.exists() {
        match &key_result {
            Ok(key) => match MemoryStore::open(&store_path, key) {
                Ok(store) => {
                    let count = store.record_count().unwrap_or(0);
                    println!(
                        "Store:             {} ({} records)",
                        store_path.display(),
                        count
                    );
                }
                Err(_) => println!("Store:             {} (wrong key)", store_path.display()),
            },
            Err(_) => println!("Store:             {} (no key)", store_path.display()),
        }
    } else {
        println!(
            "Store:             {} (not initialized)",
            store_path.display()
        );
    }

    // ── ANTHROPIC_API_KEY status ───────────────────────────────────────────────
    match std::env::var("ANTHROPIC_API_KEY") {
        Ok(v) if !v.is_empty() => println!("ANTHROPIC_API_KEY: set \u{2713}"),
        _ => println!("ANTHROPIC_API_KEY: not set"),
    }
}

/// Collect the active vaults to display in `engram awareness`.
///
/// Priority:
/// 1. If `vault_arg` is given and looks like a filesystem path (starts with `/` or `~`),
///    use it directly as a vault root.
/// 2. If `vault_arg` is given otherwise, look it up in the config by name; fall through
///    to treating it as a path if not found.
/// 3. If `vault_arg` is `None`: return all configured vaults, then the auto-detected
///    project vault (`.lifeos/memory` in cwd), then the hardcoded fallback if the
///    list would otherwise be empty.
///
/// Returns `Vec<(name, path, access_str)>`.
fn collect_active_vaults(vault_arg: Option<&str>) -> Vec<(String, PathBuf, String)> {
    if let Some(arg) = vault_arg {
        // Absolute paths and tilde-paths are used directly.
        if arg.starts_with('/') || arg.starts_with('~') {
            let path = shellexpand_path(arg);
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| arg.to_string());
            return vec![(name, path, "read-write".to_string())];
        }

        // Try a config name lookup first.
        let config = EngramConfig::load();
        if let Some(entry) = config.get_vault(arg) {
            let access_str = match &entry.access {
                VaultAccess::Read => "read".to_string(),
                VaultAccess::ReadWrite => "read-write".to_string(),
            };
            return vec![(arg.to_string(), entry.path.clone(), access_str)];
        }

        // Fall back to treating arg as a relative (or otherwise non-~ non-/) path.
        let path = PathBuf::from(arg);
        return vec![(arg.to_string(), path, "read-write".to_string())];
    }

    // No vault_arg: collect all configured vaults.
    let config = EngramConfig::load();
    let mut vaults: Vec<(String, PathBuf, String)> = config
        .vaults
        .iter()
        .map(|(name, entry)| {
            let access_str = match &entry.access {
                VaultAccess::Read => "read".to_string(),
                VaultAccess::ReadWrite => "read-write".to_string(),
            };
            (name.clone(), entry.path.clone(), access_str)
        })
        .collect();

    // Auto-detect `.lifeos/memory` in the current working directory.
    if let Ok(cwd) = std::env::current_dir() {
        let project_vault = cwd.join(".lifeos/memory");
        if project_vault.exists() {
            vaults.push((
                "project".to_string(),
                project_vault,
                "read-write".to_string(),
            ));
        }
    }

    // Hardcoded fallback when no vaults are found at all.
    if vaults.is_empty() {
        let fallback = UserDirs::new()
            .map(|u| u.home_dir().join(".lifeos/memory"))
            .unwrap_or_else(|| PathBuf::from(".lifeos/memory"));
        vaults.push(("default".to_string(), fallback, "read-write".to_string()));
    }

    vaults
}

/// Emit vault domain structure as an `<engram-context>` block.
///
/// For each collected vault:
/// - Prints a header line: `vault: <name> | <path> | <total> files | <access>`
/// - Prints a domains line if any domains were found: `domains: Domain1 (N) · Domain2 (M)`
fn run_awareness(vault_arg: Option<&str>, _all: bool) {
    let vaults = collect_active_vaults(vault_arg);
    println!("<engram-context>");
    for (name, path, access) in &vaults {
        let (total, domains) = awareness::vault_domain_summary(path);
        println!(
            "vault: {} | {} | {} files | {}",
            name,
            path.display(),
            total,
            access
        );
        if !domains.is_empty() {
            println!("domains: {}", domains);
        }
        let context = awareness::vault_context_files(path);
        if !context.is_empty() {
            println!("{}", context);
        }
        // Layer 3: recent facts from the per-vault memory store.
        let recent = awareness::vault_recent_facts(&vault_storage_dir(name), 10);
        if !recent.is_empty() {
            println!("{}", recent);
        }
    }
    println!("</engram-context>");
}

/// List configured vaults from the engram config file.
///
/// Prints a separator line, then lists each vault with:
/// - an exists marker (✓ if the path exists on disk, ✗ otherwise)
/// - the vault name
/// - a "(default)" tag if this is the default vault
/// - the filesystem path
/// - the access mode ("read" or "read-write")
/// - the sync mode ("auto", "approval", or "manual")
///
/// If the config has no vaults, prints "No vaults configured".
/// Also auto-detects `.lifeos/memory` in the current working directory.
fn run_vault_list() {
    let config = EngramConfig::load();

    println!("{}", "\u{2500}".repeat(41));

    // Auto-detect `.lifeos/memory` in cwd (informational only).
    let cwd_detected = std::env::current_dir()
        .ok()
        .map(|d| d.join(".lifeos/memory"))
        .filter(|p| p.exists());

    if config.vaults.is_empty() {
        println!("No vaults configured");
        if let Some(detected) = cwd_detected {
            println!();
            println!("  (auto-detected: {})", detected.display());
        }
        return;
    }

    let default_name = config.default_vault().map(|(n, _)| n.to_string());

    for (name, entry) in &config.vaults {
        let exists_marker = if entry.path.exists() {
            '\u{2713}' // ✓
        } else {
            '\u{2717}' // ✗
        };

        let is_default = default_name.as_deref() == Some(name.as_str());
        let default_tag = if is_default { " (default)" } else { "" };

        let access_str = match &entry.access {
            VaultAccess::Read => "read",
            VaultAccess::ReadWrite => "read-write",
        };

        let sync_str = match &entry.sync_mode {
            SyncMode::Auto => "auto",
            SyncMode::Approval => "approval",
            SyncMode::Manual => "manual",
        };

        println!(
            "  {} {}{} | {} | {} | {}",
            exists_marker,
            name,
            default_tag,
            entry.path.display(),
            access_str,
            sync_str
        );
    }
}

/// Add a vault entry to the engram config file.
///
/// - Parses `access` ("read" or "read-write") into [`VaultAccess`].
/// - Parses `sync_mode` ("auto", "approval", or "manual") into [`SyncMode`].
/// - Expands leading `~` in `path` via `shellexpand::tilde`.
/// - Loads the current config, calls `add_vault`, then saves atomically.
/// - If `default` is `true`, all other vaults have their default flag cleared.
fn run_vault_add(
    name: &str,
    path: &std::path::Path,
    access: &str,
    sync_mode: &str,
    default: bool,
    vault_type: Option<&str>,
) {
    let access_mode = match access {
        "read" => VaultAccess::Read,
        "read-write" => VaultAccess::ReadWrite,
        other => {
            eprintln!(
                "Invalid access mode: '{}'. Valid values: read, read-write",
                other
            );
            std::process::exit(1);
        }
    };

    let sync = match sync_mode {
        "auto" => SyncMode::Auto,
        "approval" => SyncMode::Approval,
        "manual" => SyncMode::Manual,
        other => {
            eprintln!(
                "Invalid sync mode: '{}'. Valid values: auto, approval, manual",
                other
            );
            std::process::exit(1);
        }
    };

    // Expand ~ to the home directory via the shared helper.
    let expanded_path = shellexpand_path(path.to_string_lossy().as_ref());

    let mut config = EngramConfig::load();
    let entry = VaultEntry {
        path: expanded_path,
        access: access_mode,
        sync_mode: sync,
        default,
        vault_type: vault_type.map(|s| s.to_string()),
    };
    config.add_vault(name.to_string(), entry);
    if let Err(e) = config.save() {
        eprintln!("Failed to save config: {}", e);
        std::process::exit(1);
    }
    println!("\u{2713} Vault '{}' added", name);
}

/// Remove a vault entry from the engram config file.
///
/// Exits with code 1 if no vault with the given name is registered.
fn run_vault_remove(name: &str) {
    let mut config = EngramConfig::load();
    if !config.remove_vault(name) {
        eprintln!("Vault '{}' not found", name);
        std::process::exit(1);
    }
    if let Err(e) = config.save() {
        eprintln!("Failed to save config: {}", e);
        std::process::exit(1);
    }
    println!("\u{2713} Vault '{}' removed", name);
}

/// Set the default vault in the engram config file.
///
/// Exits with code 1 if no vault with the given name is registered.
fn run_vault_set_default(name: &str) {
    let mut config = EngramConfig::load();
    if !config.set_default(name) {
        eprintln!("Vault '{}' not found", name);
        std::process::exit(1);
    }
    if let Err(e) = config.save() {
        eprintln!("Failed to save config: {}", e);
        std::process::exit(1);
    }
    println!("\u{2713} Default vault set to '{}'", name);
}

/// Print vault state, memory store stats, and keyring status to stdout.
fn run_status() {
    // Load config once to avoid redundant filesystem reads across helpers.
    let config = EngramConfig::load();

    // Separator line
    println!("{}", "\u{2500}".repeat(41));

    // ── Vault status ──────────────────────────────────────────────────────────
    if config.vaults.is_empty() {
        // Legacy path: show single default vault path with file count.
        let vault_path = default_vault_path_from_config(&config);
        if vault_path.exists() {
            let vault = Vault::new(&vault_path);
            let count = vault.list_markdown().map(|files| files.len()).unwrap_or(0);
            println!("Vault:        {} ({} files)", vault_path.display(), count);
        } else {
            println!("Vault:        {} (NOT FOUND)", vault_path.display());
        }
    } else {
        // Multi-vault path: print 'Vaults:' header then each configured vault.
        println!("Vaults:");
        let default_name = config.default_vault().map(|(n, _)| n.to_string());
        for (name, entry) in &config.vaults {
            let exists_marker = if entry.path.exists() {
                '\u{2713}' // ✓
            } else {
                '\u{2717}' // ✗
            };
            let is_default = default_name.as_deref() == Some(name.as_str());
            let default_tag = if is_default { " [default]" } else { "" };
            let access_str = match &entry.access {
                VaultAccess::Read => "read",
                VaultAccess::ReadWrite => "read-write",
            };
            let sync_str = match &entry.sync_mode {
                SyncMode::Auto => "auto",
                SyncMode::Approval => "approval",
                SyncMode::Manual => "manual",
            };
            let count = if entry.path.exists() {
                let vault = Vault::new(&entry.path);
                vault.list_markdown().map(|f| f.len()).unwrap_or(0)
            } else {
                0
            };
            println!(
                "  {} {}{} \u{2014} {} files  {} \u{00B7} {}",
                exists_marker, name, default_tag, count, access_str, sync_str
            );
        }
    }

    // ── Memory store status ───────────────────────────────────────────────────
    let store_path = default_store_path_from_config(&config);
    let key_result = resolve_vault_key();

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

    // ── Search index status ───────────────────────────────────────────────────
    // Use the same vault-aware path that `run_index` writes to, so that
    // `engram status` accurately reflects what `engram index` built.
    let vault_name = resolve_vault_name(None);
    let search_dir = vault_storage_dir(&vault_name).join("search");
    println!("{}", search_index_status(&search_dir));

    // ── Key status ───────────────────────────────────────────────────────
    match key_result {
        Ok(_) => println!("Key:          accessible \u{2713}"),
        Err(e) => println!("Key:          not accessible — {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

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
    #[serial]
    fn test_default_store_path_ends_with_engram_memory_db() {
        // Remove the env var first so the fallback path is tested deterministically.
        std::env::remove_var("ENGRAM_STORE_PATH");
        let path = default_store_path();
        let path_str = path.to_string_lossy();
        assert!(
            path_str.ends_with(".engram/memory.db"),
            "store path should end with .engram/memory.db, got: {}",
            path_str
        );
    }

    #[test]
    #[serial]
    fn test_default_store_path_uses_engram_store_path_env_var() {
        // When ENGRAM_STORE_PATH is set, default_store_path() must return it.
        std::env::set_var("ENGRAM_STORE_PATH", "/tmp/custom_engram_test_store.db");
        let path = default_store_path();
        std::env::remove_var("ENGRAM_STORE_PATH");
        assert_eq!(
            path.to_str().unwrap(),
            "/tmp/custom_engram_test_store.db",
            "default_store_path() should use ENGRAM_STORE_PATH env var when set"
        );
    }

    #[test]
    fn test_default_search_dir_ends_with_engram_search() {
        let path = default_search_dir();
        let path_str = path.to_string_lossy();
        assert!(
            path_str.ends_with(".engram/search"),
            "search dir should end with .engram/search, got: {}",
            path_str
        );
    }

    #[test]
    fn test_default_vectors_path_ends_with_engram_vectors_db() {
        let path = default_vectors_path();
        let path_str = path.to_string_lossy();
        assert!(
            path_str.ends_with(".engram/vectors.db"),
            "vectors path should end with .engram/vectors.db, got: {}",
            path_str
        );
    }

    #[test]
    fn test_dir_size_bytes_returns_zero_for_nonexistent_path() {
        let size = dir_size_bytes(std::path::Path::new("/tmp/nonexistent_engram_test_dir_xyz"));
        assert_eq!(size, 0, "nonexistent path should have size 0");
    }

    #[test]
    fn test_dir_size_bytes_sums_file_sizes() {
        use std::fs;
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("a.txt"), b"hello").unwrap(); // 5 bytes
        fs::write(dir.path().join("b.txt"), b"world!").unwrap(); // 6 bytes
        let size = dir_size_bytes(dir.path());
        assert_eq!(
            size, 11,
            "dir size should be sum of file sizes (5 + 6 = 11)"
        );
    }

    #[test]
    fn test_dir_size_bytes_recurses_into_subdirs() {
        use std::fs;
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("nested.txt"), b"abc").unwrap(); // 3 bytes
        fs::write(dir.path().join("top.txt"), b"xy").unwrap(); // 2 bytes
        let size = dir_size_bytes(dir.path());
        assert_eq!(size, 5, "should recurse into subdirs (3 + 2 = 5)");
    }

    // ── search_index_status unit tests ────────────────────────────────────────

    /// When no meta.json exists the status must be the "not built" message.
    #[test]
    fn test_search_index_status_not_built_when_no_meta_json() {
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        // Nothing created in `dir`, so meta.json is absent.
        let status = search_index_status(dir.path());
        assert_eq!(
            status, "Search index: not built (run: engram index)",
            "should return the 'not built' message when meta.json is absent"
        );
    }

    /// When a valid index exists the status must include the doc count and MB size.
    #[test]
    fn test_search_index_status_shows_doc_count_when_index_exists() {
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        // Create a real index with one document.
        {
            let mut indexer = TantivyIndexer::open(dir.path()).unwrap();
            indexer
                .index_file("note.md", "hello world content for status test")
                .unwrap();
        } // indexer (and its IndexWriter lock) is dropped here

        let status = search_index_status(dir.path());
        assert!(
            status.starts_with("Search index:"),
            "status should start with 'Search index:', got: {}",
            status
        );
        assert!(
            status.contains("files indexed"),
            "status should contain 'files indexed', got: {}",
            status
        );
        assert!(
            status.contains("MB"),
            "status should contain the MB size, got: {}",
            status
        );
    }

    /// The "not built" path must not mention a directory path.
    #[test]
    fn test_search_index_status_not_built_has_no_path() {
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let status = search_index_status(dir.path());
        // The "not built" message must not embed the search dir path.
        assert!(
            !status.contains(dir.path().to_str().unwrap()),
            "not-built message must not include the search dir path, got: {}",
            status
        );
    }

    // ── vault_storage_dir unit tests ──────────────────────────────────────────────

    /// `vault_storage_dir("personal")` must produce a path ending with `.engram/personal`.
    #[test]
    fn test_vault_storage_dir_ends_with_vault_name() {
        let path = vault_storage_dir("personal");
        let path_str = path.to_string_lossy();
        assert!(
            path_str.ends_with(".engram/personal"),
            "vault_storage_dir(\"personal\") should end with .engram/personal, got: {}",
            path_str
        );
    }

    /// `vault_storage_dir` with a different name must end with that name.
    #[test]
    fn test_vault_storage_dir_uses_provided_name() {
        let path = vault_storage_dir("work");
        let path_str = path.to_string_lossy();
        assert!(
            path_str.ends_with(".engram/work"),
            "vault_storage_dir(\"work\") should end with .engram/work, got: {}",
            path_str
        );
    }

    // ── shellexpand_path unit tests ───────────────────────────────────────────────

    /// `shellexpand_path("~/foo")` must expand the tilde to an absolute path.
    #[test]
    fn test_shellexpand_path_expands_tilde() {
        let path = shellexpand_path("~/foo");
        // After expansion the path must be absolute (no leading ~).
        let path_str = path.to_string_lossy();
        assert!(
            !path_str.starts_with('~'),
            "shellexpand_path should expand ~ to an absolute path, got: {}",
            path_str
        );
        assert!(
            path_str.ends_with("/foo"),
            "shellexpand_path should preserve the suffix /foo, got: {}",
            path_str
        );
    }

    /// `shellexpand_path` with an already-absolute path must return it unchanged.
    #[test]
    fn test_shellexpand_path_leaves_absolute_path_unchanged() {
        let path = shellexpand_path("/tmp/test-path");
        assert_eq!(
            path.to_str().unwrap(),
            "/tmp/test-path",
            "shellexpand_path should not alter an absolute path"
        );
    }

    // ── resolve_vault_key unit tests ─────────────────────────────────────────

    /// Tier 1: ENGRAM_VAULT_KEY env var with 32 bytes of 42u8 must resolve successfully.
    #[test]
    #[serial]
    fn test_resolve_key_from_vault_key_env_var() {
        let key_bytes = [42u8; 32];
        let encoded = B64.encode(key_bytes);

        std::env::remove_var("ENGRAM_VAULT_PASSPHRASE");
        std::env::set_var("ENGRAM_VAULT_KEY", &encoded);

        let result = resolve_vault_key();

        std::env::remove_var("ENGRAM_VAULT_KEY");

        assert!(
            result.is_ok(),
            "should resolve key from ENGRAM_VAULT_KEY env var, got: {:?}",
            result
        );
    }

    /// Tier 2: ENGRAM_VAULT_PASSPHRASE env var + zero salt in config must resolve
    /// deterministically.
    #[test]
    #[serial]
    fn test_resolve_key_from_passphrase_env_var() {
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.toml");

        // Zero salt = 16 bytes of 0x00.
        let zero_salt = B64.encode([0u8; 16]);
        std::fs::write(&config_path, format!("[key]\nsalt = \"{}\"\n", zero_salt)).unwrap();

        std::env::remove_var("ENGRAM_VAULT_KEY");
        std::env::set_var("ENGRAM_CONFIG_PATH", config_path.to_str().unwrap());
        std::env::set_var("ENGRAM_VAULT_PASSPHRASE", "test-passphrase");

        let result = resolve_vault_key();

        std::env::remove_var("ENGRAM_VAULT_PASSPHRASE");
        std::env::remove_var("ENGRAM_CONFIG_PATH");

        assert!(
            result.is_ok(),
            "should resolve key from ENGRAM_VAULT_PASSPHRASE env var, got: {:?}",
            result
        );
    }

    /// Tier 3 fallback when no env vars are set and config has no salt: must return
    /// an Err containing "engram init".
    #[test]
    #[serial]
    fn test_resolve_key_fails_gracefully_when_not_initialized() {
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        // Point config to a nonexistent file so EngramConfig::load() returns Default
        // (no salt).
        let config_path = dir.path().join("nonexistent-config.toml");

        std::env::remove_var("ENGRAM_VAULT_KEY");
        std::env::remove_var("ENGRAM_VAULT_PASSPHRASE");
        std::env::set_var("ENGRAM_CONFIG_PATH", config_path.to_str().unwrap());

        let result = resolve_vault_key();

        std::env::remove_var("ENGRAM_CONFIG_PATH");

        assert!(result.is_err(), "should fail when not initialized");
        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("engram init"),
            "error should mention 'engram init', got: {}",
            err_msg
        );
    }

    /// Tier 1 with invalid base64 in ENGRAM_VAULT_KEY must return Err containing "base64".
    #[test]
    #[serial]
    fn test_resolve_key_invalid_base64_vault_key_env() {
        std::env::remove_var("ENGRAM_VAULT_PASSPHRASE");
        std::env::set_var("ENGRAM_VAULT_KEY", "not-valid-base64!!!");

        let result = resolve_vault_key();

        std::env::remove_var("ENGRAM_VAULT_KEY");

        assert!(result.is_err(), "should fail with invalid base64");
        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("base64"),
            "error should mention 'base64', got: {}",
            err_msg
        );
    }

    // ── resolve_vault unit tests ──────────────────────────────────────────────────

    /// When no name override is given and no project vault exists in the cwd,
    /// `resolve_vault(None)` must return either the config default or the fallback
    /// `~/.lifeos/memory`.
    #[test]
    #[serial]
    fn test_resolve_vault_none_returns_fallback_when_no_config() {
        // Use a temp dir as the current directory so no .lifeos/memory is present.
        use tempfile::TempDir;
        let dir = TempDir::new().unwrap();
        // Save cwd, change to temp dir, restore after test.
        let original = std::env::current_dir().ok();
        std::env::set_current_dir(dir.path()).unwrap();
        // Point ENGRAM_CONFIG_PATH to a nonexistent file so config is empty.
        std::env::set_var("ENGRAM_CONFIG_PATH", dir.path().join("no-config.toml"));

        let path = resolve_vault(None);

        // Restore state.
        if let Some(orig) = original {
            let _ = std::env::set_current_dir(orig);
        }
        std::env::remove_var("ENGRAM_CONFIG_PATH");

        let path_str = path.to_string_lossy();
        assert!(
            path_str.ends_with(".lifeos/memory"),
            "resolve_vault(None) with no config should fall back to .lifeos/memory, got: {}",
            path_str
        );
    }
}
