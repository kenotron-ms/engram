# Kotlin / Android Usage — EngramCore

    > Generated bindings are in `bindings/kotlin/`. Copy the `uniffi/engram_core/`
    > directory into your Android project's `src/main/java/` tree and include
    > `libengram_core.so` in `src/main/jniLibs/arm64-v8a/`
    > (build with `cargo build --target aarch64-linux-android --release`).

    ## Key derivation and encryption

    ```kotlin
    import uniffi.engram_core.*

    // Derive a 32-byte key from the user's password
    val salt = generateSalt()
    val keyBytes = deriveKey("user-passphrase", salt)

    // Encrypt
    val plaintext = "Hello, engram!".toByteArray().toList()
    val ciphertext = encryptBytes(keyBytes, plaintext)

    // Decrypt
    val decrypted = decryptBytes(keyBytes, ciphertext)
    assert(decrypted == plaintext)
    ```

    ## Vault operations

    ```kotlin
    val vaultPath = "${System.getProperty("user.home")}/.engram/vault"

    // Write a markdown file (parent directories created automatically)
    vaultWrite(vaultPath, "People/Sofia.md", "# Sofia\n\n- dietary: vegetarian")

    // Read it back
    val content: String = vaultRead(vaultPath, "People/Sofia.md")

    // List all .md files recursively
    val files: List<String> = vaultListMarkdown(vaultPath)
    ```

    ## Memory store

    ```kotlin
    val dbPath = "${System.getProperty("user.home")}/.engram/memory.db"

    // Open (or create) the encrypted SQLite memory store
    val store = MemoryStoreHandle(dbPath, keyBytes)

    // Insert an atomic fact
    store.insertMemory("Sofia", "dietary", "vegetarian", "2026-04-14 transcript")

    // Query all facts for an entity (newest first)
    val memories: List<MemoryRecord> = store.findByEntity("Sofia")
    memories.forEach { println("${it.entity} — ${it.attribute}: ${it.value}") }

    // Fetch by UUID
    val record: MemoryRecord? = store.getMemory(memories[0].id)
    println("Source: ${record?.source}")

    println("Total: ${store.recordCount()}")
    ```
    