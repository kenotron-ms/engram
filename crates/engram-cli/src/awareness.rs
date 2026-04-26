// awareness.rs — vault domain structure and context file helpers

use engram_core::crypto::KeyStore;
use engram_core::store::MemoryStore;
use engram_core::vault::Vault;
use std::collections::BTreeMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Count markdown files by top-level directory, returning (total, domain_summary).
///
/// - Calls `Vault::new(vault_path).list_markdown()` to enumerate all `.md` files.
/// - Excludes any top-level directory whose name starts with `_` or `.`.
/// - Uses a `BTreeMap` to produce sorted output.
/// - Returns `(total_count, "Domain1 (N) · Domain2 (M)")`.
///   `total_count` is the count of all markdown files (including excluded dirs).
///   Root-level files (no parent directory) are counted in total but not in any domain.
pub fn vault_domain_summary(vault_path: &Path) -> (usize, String) {
    let vault = Vault::new(vault_path);
    let files = match vault.list_markdown() {
        Ok(f) => f,
        Err(_) => return (0, String::new()),
    };

    let total = files.len();
    let mut domain_counts: BTreeMap<String, usize> = BTreeMap::new();

    for file in &files {
        // Split on the first '/' to get the top-level component.
        // Files without a '/' are root-level — they don't belong to any domain.
        let mut parts = file.splitn(2, '/');
        let top_level = parts.next().unwrap_or("");
        let rest = parts.next();

        // Only count files that are inside a top-level directory (not root-level).
        if rest.is_none() {
            continue;
        }

        // Skip directories starting with '_' or '.'
        if top_level.starts_with('_') || top_level.starts_with('.') {
            continue;
        }

        *domain_counts.entry(top_level.to_string()).or_insert(0) += 1;
    }

    let domains_str = domain_counts
        .iter()
        .map(|(name, count)| format!("{} ({})", name, count))
        .collect::<Vec<_>>()
        .join(" \u{00b7} "); // U+00B7 MIDDLE DOT

    (total, domains_str)
}

/// Read all `_context/*.md` files, sorted alphabetically, and concatenate their contents.
///
/// - Returns an empty string if no `_context` directory exists.
/// - Trims each file's content; skips files whose trimmed content is empty.
/// - Joins non-empty trimmed contents with a double newline (`\n\n`).
///
/// NOTE: Wired into `run_awareness` output in Layer 2.
pub fn vault_context_files(vault_path: &Path) -> String {
    let context_dir = vault_path.join("_context");
    if !context_dir.exists() {
        return String::new();
    }

    let mut md_files: Vec<std::path::PathBuf> = match std::fs::read_dir(&context_dir) {
        Ok(entries) => entries
            .flatten()
            .filter(|e| {
                e.path()
                    .extension()
                    .map_or(false, |ext| ext.eq_ignore_ascii_case("md"))
            })
            .map(|e| e.path())
            .collect(),
        Err(_) => return String::new(),
    };

    // Sort alphabetically for deterministic ordering.
    md_files.sort();

    let contents: Vec<String> = md_files
        .iter()
        .filter_map(|f| std::fs::read_to_string(f).ok())
        .filter_map(|content| {
            let trimmed = content.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
        .collect();

    contents.join("\n\n")
}

/// Query the engram memory store for recently-observed facts and format them
/// as a "Top of mind" block.
///
/// - Retrieves the vault encryption key via `KeyStore::new("engram").retrieve()`;
///   returns empty if unavailable (no key in keychain).
/// - Checks that `engram_dir/memory.db` exists; returns empty if not.
/// - Opens the MemoryStore; returns empty on error.
/// - Queries facts observed in the last 30 days (30-day cutoff in ms) with
///   the given `limit`.
/// - Groups results by entity into a `BTreeMap`.
/// - Formats as `"Top of mind:\n- entity: value1, value2"` lines, taking up
///   to `limit` entities.
/// - Returns an empty string when no facts are found.
///
/// NOTE: Wired into `run_awareness` output in Layer 3.
pub fn vault_recent_facts(engram_dir: &Path, limit: usize) -> String {
    // Step 1: Retrieve keychain key — return empty on error.
    let key = match KeyStore::new("engram").retrieve() {
        Ok(k) => k,
        Err(_) => return String::new(),
    };

    // Step 2: Check memory.db exists — return empty if not.
    let db_path = engram_dir.join("memory.db");
    if !db_path.exists() {
        return String::new();
    }

    // Step 3: Open MemoryStore — return empty on error.
    let store = match MemoryStore::open(&db_path, &key) {
        Ok(s) => s,
        Err(_) => return String::new(),
    };

    // Step 4: 30-day cutoff in milliseconds.
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    let thirty_days_ms: i64 = 30 * 24 * 60 * 60 * 1_000;
    let since_ms = now_ms - thirty_days_ms;

    let memories = match store.list_recent(since_ms, limit) {
        Ok(m) => m,
        Err(_) => return String::new(),
    };

    if memories.is_empty() {
        return String::new();
    }

    // Step 5: Group results by entity into BTreeMap.
    let mut entity_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for memory in memories {
        entity_map.entry(memory.entity).or_default().push(memory.value);
    }

    // Step 6: Format as 'Top of mind:\n- entity: value1, value2' (up to limit entities).
    let lines: Vec<String> = entity_map
        .iter()
        .take(limit)
        .map(|(entity, values)| format!("- {}: {}", entity, values.join(", ")))
        .collect();

    if lines.is_empty() {
        return String::new();
    }

    format!("Top of mind:\n{}", lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_tmp_vault() -> TempDir {
        TempDir::new().unwrap()
    }

    // ── vault_domain_summary ──────────────────────────────────────────────────

    #[test]
    fn test_domain_summary_empty_vault() {
        let tmp = make_tmp_vault();
        let (total, domains) = vault_domain_summary(tmp.path());
        assert_eq!(total, 0);
        assert_eq!(domains, "");
    }

    #[test]
    fn test_domain_summary_nonexistent_vault() {
        let path = std::path::PathBuf::from("/tmp/nonexistent_awareness_vault_xyz");
        let (total, domains) = vault_domain_summary(&path);
        assert_eq!(total, 0);
        assert_eq!(domains, "");
    }

    #[test]
    fn test_domain_summary_counts_by_top_level_dir() {
        let tmp = make_tmp_vault();
        let work = tmp.path().join("Work");
        let people = tmp.path().join("People");
        fs::create_dir_all(&work).unwrap();
        fs::create_dir_all(&people).unwrap();
        fs::write(work.join("a.md"), "a").unwrap();
        fs::write(work.join("b.md"), "b").unwrap();
        fs::write(people.join("c.md"), "c").unwrap();

        let (total, domains) = vault_domain_summary(tmp.path());
        assert_eq!(total, 3);
        // BTreeMap sorts: People comes before Work alphabetically
        assert!(domains.contains("Work (2)"), "got: {}", domains);
        assert!(domains.contains("People (1)"), "got: {}", domains);
    }

    #[test]
    fn test_domain_summary_excludes_underscore_dirs() {
        let tmp = make_tmp_vault();
        let ctx = tmp.path().join("_context");
        fs::create_dir_all(&ctx).unwrap();
        fs::write(ctx.join("ctx.md"), "context").unwrap();

        let work = tmp.path().join("Work");
        fs::create_dir_all(&work).unwrap();
        fs::write(work.join("note.md"), "note").unwrap();

        let (total, domains) = vault_domain_summary(tmp.path());
        // total includes _context file
        assert_eq!(total, 2);
        // domains must not list _context
        assert!(!domains.contains("_context"), "got: {}", domains);
        assert!(domains.contains("Work (1)"), "got: {}", domains);
    }

    #[test]
    fn test_domain_summary_excludes_dot_dirs() {
        let tmp = make_tmp_vault();
        let hidden = tmp.path().join(".hidden");
        fs::create_dir_all(&hidden).unwrap();
        fs::write(hidden.join("x.md"), "x").unwrap();

        let (total, domains) = vault_domain_summary(tmp.path());
        assert_eq!(total, 1);
        assert_eq!(domains, "", "dot-dirs must not appear in domain list");
    }

    #[test]
    fn test_domain_summary_uses_btreemap_sorted_order() {
        let tmp = make_tmp_vault();
        for dir in &["Zebra", "Alpha", "Middle"] {
            let d = tmp.path().join(dir);
            fs::create_dir_all(&d).unwrap();
            fs::write(d.join("f.md"), "content").unwrap();
        }

        let (_total, domains) = vault_domain_summary(tmp.path());
        // BTreeMap guarantees: Alpha, Middle, Zebra
        let alpha_pos = domains.find("Alpha").unwrap();
        let middle_pos = domains.find("Middle").unwrap();
        let zebra_pos = domains.find("Zebra").unwrap();
        assert!(
            alpha_pos < middle_pos && middle_pos < zebra_pos,
            "domains must be alphabetically sorted, got: {}",
            domains
        );
    }

    // ── vault_context_files ───────────────────────────────────────────────────

    #[test]
    fn test_context_files_empty_when_no_context_dir() {
        let tmp = make_tmp_vault();
        let result = vault_context_files(tmp.path());
        assert_eq!(result, "");
    }

    #[test]
    fn test_context_files_concatenates_sorted() {
        let tmp = make_tmp_vault();
        let ctx = tmp.path().join("_context");
        fs::create_dir_all(&ctx).unwrap();
        fs::write(ctx.join("b_second.md"), "  Content B  ").unwrap();
        fs::write(ctx.join("a_first.md"), "Content A").unwrap();

        let result = vault_context_files(tmp.path());
        // Alphabetically: a_first then b_second
        assert!(result.contains("Content A"), "got: {}", result);
        assert!(result.contains("Content B"), "got: {}", result);
        let a_pos = result.find("Content A").unwrap();
        let b_pos = result.find("Content B").unwrap();
        assert!(a_pos < b_pos, "a_first.md should appear before b_second.md");
    }

    #[test]
    fn test_context_files_skips_empty_files() {
        let tmp = make_tmp_vault();
        let ctx = tmp.path().join("_context");
        fs::create_dir_all(&ctx).unwrap();
        fs::write(ctx.join("empty.md"), "   \n  \n  ").unwrap();
        fs::write(ctx.join("real.md"), "Real content").unwrap();

        let result = vault_context_files(tmp.path());
        assert!(result.contains("Real content"), "got: {}", result);
        assert!(!result.contains("empty"), "got: {}", result);
    }

    #[test]
    fn test_context_files_joined_with_double_newline() {
        let tmp = make_tmp_vault();
        let ctx = tmp.path().join("_context");
        fs::create_dir_all(&ctx).unwrap();
        fs::write(ctx.join("a.md"), "First").unwrap();
        fs::write(ctx.join("b.md"), "Second").unwrap();

        let result = vault_context_files(tmp.path());
        assert!(result.contains("First\n\nSecond"), "got: {}", result);
    }
}
