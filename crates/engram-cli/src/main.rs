// engram-cli — Personal memory assistant CLI

mod daemon;
mod load;
mod observe;

use clap::{Parser, Subcommand, ValueEnum};
use directories::UserDirs;
use engram_core::{crypto::KeyStore, store::MemoryStore, vault::Vault};
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
    /// Index vault markdown files for full-text search
    Index {
        /// Vault path (defaults to ~/.lifeos/memory)
        #[arg(long)]
        vault: Option<PathBuf>,
        /// Force a full reindex by wiping the search index first
        #[arg(long)]
        force: bool,
    },
    /// Search the indexed vault
    Search {
        /// Query string
        query: String,
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
        Commands::Index { vault, force } => run_index(vault, force),
        Commands::Search { query, limit, mode } => run_search(&query, limit, &mode),
        Commands::Observe {
            session_path,
            api_key,
        } => run_observe(&session_path, api_key.as_deref()),
        Commands::Load { format } => run_load(&format),
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

    let ak = access_key.map(|s| s.to_string()).unwrap_or_else(|| {
        print!("Access key ID: ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        input.trim().to_string()
    });

    let sk = secret_key
        .map(|s| s.to_string())
        .unwrap_or_else(|| rpassword::prompt_password("Secret access key: ").unwrap_or_default());

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
            (
                "redirect_uri",
                "https://login.microsoftonline.com/common/oauth2/nativeclient",
            ),
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
        (
            "s3",
            &["access_key", "secret_key", "endpoint", "bucket"],
            &["endpoint", "bucket"],
        ),
        ("onedrive", &["access_token", "folder"], &["folder"]),
        (
            "azure",
            &["account", "container"],
            &["account", "container"],
        ),
        ("gcs", &["bucket", "key_file"], &["bucket", "key_file"]),
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
        (
            "s3",
            ["access_key", "secret_key", "endpoint", "bucket"].as_slice(),
        ),
        (
            "onedrive",
            ["access_token", "refresh_token", "folder"].as_slice(),
        ),
        ("azure", ["account", "access_key", "container"].as_slice()),
        ("gcs", ["bucket", "key_file"].as_slice()),
    ]
    .into_iter()
    .collect();

    match keys_by_backend.get(backend) {
        None => {
            eprintln!(
                "Unknown backend: {}. Valid options: s3, onedrive, azure, gcs",
                backend
            );
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

fn run_sync(backend_name: Option<&str>) {
    use engram_core::{crypto::KeyStore, vault::Vault};
    use engram_sync::{
        auth::AuthStore, backend::SyncBackend, encrypt::encrypt_for_sync,
        onedrive::OneDriveBackend, s3::S3Backend,
    };

    let vault_path = default_vault_path();
    let vault = Vault::new(&vault_path);
    let key_store = KeyStore::new("engram");

    let key = match key_store.retrieve() {
        Ok(k) => k,
        Err(_) => {
            eprintln!("No vault key found. Run: engram init");
            std::process::exit(1);
        }
    };

    // Determine which backend to use: explicit arg → first configured → error
    let effective_backend = backend_name.unwrap_or_else(|| {
        if AuthStore::is_configured("s3", &["access_key", "secret_key", "endpoint", "bucket"]) {
            "s3"
        } else if AuthStore::is_configured("onedrive", &["access_token", "folder"]) {
            "onedrive"
        } else if AuthStore::is_configured("azure", &["account", "container", "access_key"]) {
            "azure"
        } else if AuthStore::is_configured("gcs", &["bucket", "key_file"]) {
            "gcs"
        } else {
            eprintln!("No sync backend configured. Run: engram auth add s3|onedrive|azure|gdrive");
            std::process::exit(1);
        }
    });

    let backend: Box<dyn SyncBackend> = match effective_backend {
        "s3" => {
            let endpoint = AuthStore::retrieve("s3", "endpoint").unwrap();
            let bucket = AuthStore::retrieve("s3", "bucket").unwrap();
            let ak = AuthStore::retrieve("s3", "access_key").unwrap();
            let sk = AuthStore::retrieve("s3", "secret_key").unwrap();
            Box::new(S3Backend::new(&endpoint, &bucket, &ak, &sk).unwrap())
        }
        "onedrive" => {
            let token = AuthStore::retrieve("onedrive", "access_token").unwrap();
            let folder = AuthStore::retrieve("onedrive", "folder").unwrap();
            Box::new(OneDriveBackend::new(&token, &folder))
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

/// Returns the default search index path: `~/.engram/search`.
fn default_search_dir() -> PathBuf {
    UserDirs::new()
        .map(|u| u.home_dir().join(".engram/search"))
        .unwrap_or_else(|| PathBuf::from(".engram/search"))
}

/// Returns the default vector index path: `~/.engram/vectors.db`.
fn default_vectors_path() -> PathBuf {
    UserDirs::new()
        .map(|u| u.home_dir().join(".engram/vectors.db"))
        .unwrap_or_else(|| PathBuf::from(".engram/vectors.db"))
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
fn run_index(vault_path: Option<PathBuf>, force: bool) {
    use engram_search::embedder::Embedder;
    use engram_search::vector::VectorIndex;

    let vault_path = vault_path.unwrap_or_else(default_vault_path);

    if !vault_path.exists() {
        eprintln!("Vault not found: {}", vault_path.display());
        std::process::exit(1);
    }

    let search_dir = default_search_dir();
    let vectors_path = default_vectors_path();

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
            eprintln!("  \u{2717} {}: vector insert failed \u{2014} {}", rel_path, e);
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
fn run_search(query: &str, limit: usize, mode: &SearchMode) {
    use engram_search::embedder::Embedder;
    use engram_search::hybrid::HybridSearch;
    use engram_search::vector::VectorIndex;

    let search_dir = default_search_dir();

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
            let vectors_path = default_vectors_path();
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
            let vectors_path = default_vectors_path();
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
    let key_store = KeyStore::new("engram");
    let key = match key_store.retrieve() {
        Ok(k) => k,
        Err(_) => {
            eprintln!("No vault key found. Run: engram init");
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

/// Observe a session transcript: parse turns, extract facts via LLM, write to store.
fn run_observe(session_path: &Path, api_key: Option<&str>) {
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

    // Retrieve the vault encryption key from the system keyring.
    let key_store = KeyStore::new("engram");
    let key = match key_store.retrieve() {
        Ok(k) => k,
        Err(_) => {
            eprintln!("No vault key found. Run: engram init");
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

    // ── Search index status ───────────────────────────────────────────────────
    let search_dir = default_search_dir();
    println!("{}", search_index_status(&search_dir));

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
}
