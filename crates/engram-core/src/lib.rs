// engram-core: personal memory infrastructure

// Suppress Clippy lints triggered by uniffi-generated scaffolding code.
#![allow(clippy::empty_line_after_doc_comments)]

// UniFFI scaffolding — generated from src/engram_core.udl by build.rs.
// Must appear before module declarations that use uniffi derives.
uniffi::include_scaffolding!("engram_core");

pub mod crypto;
pub mod ffi;
pub mod store;
pub mod vault;

// Re-export the FFI surface to the crate root.
// The generated scaffolding resolves free functions and types by their
// unqualified names; this pub use makes them visible at crate root.
pub use ffi::{
    decrypt_bytes, derive_key, encrypt_bytes, generate_salt, vault_list_markdown, vault_read,
    vault_write, EngramError, MemoryRecord, MemoryStoreHandle,
};
