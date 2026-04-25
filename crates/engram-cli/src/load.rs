// load.rs — load recent memories as an AI context block

use engram_core::store::{MemoryStore, StoreError};
use thiserror::Error;

/// Errors that can occur during context loading.
#[derive(Debug, Error)]
pub enum LoadError {
    #[error("store error: {0}")]
    Store(#[from] StoreError),
}

/// 30 days in milliseconds.
pub const THIRTY_DAYS_MS: i64 = 30 * 24 * 60 * 60 * 1000;

/// Query recent memories (last 30 days, up to 20) and format them as an
/// `<engram-context>` XML block grouped by entity.
///
/// Each entity's attributes appear on one line:
/// `- Entity: attr1: val1, attr2: val2`
///
/// Returns `"<engram-context>\nNo recent memories.\n</engram-context>"` when
/// the store contains no recent memories.
pub fn load_context(store: &MemoryStore) -> Result<String, LoadError> {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock before epoch")
        .as_millis() as i64;

    let since_ms = now_ms - THIRTY_DAYS_MS;
    let memories = store.list_recent(since_ms, 20)?;

    if memories.is_empty() {
        return Ok("<engram-context>\nNo recent memories.\n</engram-context>".to_string());
    }

    // Group by entity, preserving chronological (oldest-first) insertion order.
    // list_recent returns DESC order, so reverse first.
    let mut entity_order: Vec<String> = Vec::new();
    let mut entity_attrs: std::collections::HashMap<String, Vec<(String, String)>> =
        std::collections::HashMap::new();

    for memory in memories.iter().rev() {
        if !entity_attrs.contains_key(&memory.entity) {
            entity_order.push(memory.entity.clone());
            entity_attrs.insert(memory.entity.clone(), Vec::new());
        }
        entity_attrs
            .get_mut(&memory.entity)
            .unwrap()
            .push((memory.attribute.clone(), memory.value.clone()));
    }

    let mut lines = vec!["<engram-context>".to_string()];
    for entity in &entity_order {
        let attrs = &entity_attrs[entity];
        let attr_str = attrs
            .iter()
            .map(|(a, v)| format!("{}: {}", a, v))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("- {}: {}", entity, attr_str));
    }
    lines.push("</engram-context>".to_string());

    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use engram_core::crypto::EngramKey;
    use engram_core::store::{Memory, MemoryStore};
    use tempfile::TempDir;

    fn test_key() -> EngramKey {
        EngramKey::derive(b"testpassword", &[0u8; 16]).expect("key derivation failed")
    }

    fn temp_store() -> (TempDir, MemoryStore) {
        let dir = TempDir::new().expect("create temp dir failed");
        let path = dir.path().join("test.db");
        let store = MemoryStore::open(&path, &test_key()).expect("open store failed");
        (dir, store)
    }

    #[test]
    fn test_load_context_contains_engram_context_tags() {
        let (_dir, store) = temp_store();
        let m = Memory::new("Sofia", "dietary", "vegetarian", None);
        store.insert(&m).expect("insert failed");
        let ctx = load_context(&store).expect("load_context failed");
        assert!(
            ctx.contains("<engram-context>"),
            "output should contain opening tag, got: {}",
            ctx
        );
        assert!(
            ctx.contains("</engram-context>"),
            "output should contain closing tag, got: {}",
            ctx
        );
    }

    #[test]
    fn test_load_context_empty_store_shows_no_recent_memories() {
        let (_dir, store) = temp_store();
        let ctx = load_context(&store).expect("load_context failed");
        assert!(
            ctx.contains("No recent memories."),
            "empty store should show 'No recent memories.', got: {}",
            ctx
        );
    }

    #[test]
    fn test_load_context_groups_by_entity() {
        let (_dir, store) = temp_store();
        let m1 = Memory::new("Sofia", "dietary", "vegetarian", None);
        store.insert(&m1).expect("insert m1 failed");
        let m2 = Memory::new("Sofia", "role", "engineer", None);
        store.insert(&m2).expect("insert m2 failed");
        let m3 = Memory::new("Chris", "role", "manager", None);
        store.insert(&m3).expect("insert m3 failed");

        let ctx = load_context(&store).expect("load_context failed");
        let lines: Vec<&str> = ctx.lines().collect();

        // Find the line for Sofia
        let sofia_line = lines
            .iter()
            .find(|l| l.starts_with("- Sofia:"))
            .copied()
            .expect("should have a Sofia line");

        // Sofia's line should contain both attributes
        assert!(
            sofia_line.contains("dietary: vegetarian"),
            "Sofia's line should contain 'dietary: vegetarian', got: {}",
            sofia_line
        );
        assert!(
            sofia_line.contains("role: engineer"),
            "Sofia's line should contain 'role: engineer', got: {}",
            sofia_line
        );

        // Chris should be on a separate line
        let chris_line = lines
            .iter()
            .find(|l| l.starts_with("- Chris:"))
            .copied()
            .expect("should have a Chris line");
        assert!(
            chris_line.contains("role: manager"),
            "Chris's line should contain 'role: manager', got: {}",
            chris_line
        );
    }

    #[test]
    fn test_load_context_two_entities_two_lines() {
        let (_dir, store) = temp_store();
        let m1 = Memory::new("Alice", "hobby", "painting", None);
        store.insert(&m1).expect("insert m1 failed");
        let m2 = Memory::new("Bob", "hobby", "cycling", None);
        store.insert(&m2).expect("insert m2 failed");

        let ctx = load_context(&store).expect("load_context failed");
        let entity_lines: Vec<&str> = ctx.lines().filter(|l| l.starts_with("- ")).collect();
        assert_eq!(
            entity_lines.len(),
            2,
            "two entities should produce two entity lines, got: {:?}",
            entity_lines
        );
    }
}
