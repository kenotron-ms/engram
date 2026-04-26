# Swift / iOS Usage — EngramCore

    > Generated bindings are in `bindings/swift/`. Copy `engram_core.swift` and
    > `engram_coreFFI.h` into your Xcode project and link against `libengram_core.a`
    > (build with `cargo build --target aarch64-apple-ios --release`).

    ## Key derivation and encryption

    ```swift
    import Foundation

    // 1. Generate a random salt and derive a 32-byte key from the user's password.
    //    The salt is not secret — store it alongside the encrypted data.
    let salt = generateSalt()
    let keyBytes = try deriveKey(password: "user-passphrase", salt: salt)

    // 2. Encrypt arbitrary bytes
    let plaintext: [UInt8] = Array("Hello, engram!".utf8)
    let ciphertext = try encryptBytes(keyBytes: keyBytes, plaintext: plaintext)

    // 3. Decrypt
    let decrypted = try decryptBytes(keyBytes: keyBytes, ciphertext: ciphertext)
    assert(decrypted == plaintext)
    ```

    ## Vault operations

    ```swift
    let vaultPath = "\(NSHomeDirectory())/.engram/vault"

    // Write a markdown file into the vault (creates parent directories automatically)
    try vaultWrite(
        vaultPath: vaultPath,
        relativePath: "People/Sofia.md",
        content: "# Sofia\n\n- dietary: vegetarian"
    )

    // Read it back
    let content = try vaultRead(vaultPath: vaultPath, relativePath: "People/Sofia.md")

    // List all .md files recursively
    let files: [String] = try vaultListMarkdown(vaultPath: vaultPath)
    ```

    ## Memory store

    ```swift
    let dbPath = "\(NSHomeDirectory())/.engram/memory.db"

    // Open (or create) the encrypted SQLite memory store
    let store = try MemoryStoreHandle(dbPath: dbPath, keyBytes: keyBytes)

    // Insert an atomic fact
    try store.insertMemory(
        entity: "Sofia",
        attribute: "dietary",
        value: "vegetarian",
        source: "2026-04-14 transcript"
    )

    // Query all facts for an entity (newest first)
    let memories: [MemoryRecord] = try store.findByEntity(entity: "Sofia")
    for m in memories {
        print("\(m.entity) — \(m.attribute): \(m.value)")
    }

    // Fetch by UUID
    if let record = try store.getMemory(id: memories[0].id) {
        print("Found: \(record.value)")
    }

    print("Total records: \(try store.recordCount())")
    ```
    