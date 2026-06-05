CREATE TABLE schema_version (
    version INTEGER NOT NULL
);

CREATE TABLE wallet_state (
    key   TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);

CREATE TABLE signer_owners (
    id         TEXT PRIMARY KEY NOT NULL,
    owner_kind TEXT NOT NULL CHECK(owner_kind IN ('seed', 'ledger')),
    label      TEXT UNIQUE NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE signer_owner_vaults (
    signer_owner_id TEXT PRIMARY KEY NOT NULL REFERENCES signer_owners(id) ON DELETE CASCADE,
    kdf_algorithm   TEXT NOT NULL,
    kdf_params_json TEXT NOT NULL,
    salt            BLOB NOT NULL,
    encrypted_dek   BLOB NOT NULL,
    dek_nonce       BLOB NOT NULL,
    cipher_version  INTEGER NOT NULL DEFAULT 1,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

CREATE TABLE seed_owner_secrets (
    signer_owner_id   TEXT PRIMARY KEY NOT NULL REFERENCES signer_owners(id) ON DELETE CASCADE,
    cipher_version    INTEGER NOT NULL,
    payload_ciphertext BLOB NOT NULL,
    payload_nonce      BLOB NOT NULL
);

CREATE TABLE ledger_owner_details (
    signer_owner_id      TEXT PRIMARY KEY NOT NULL REFERENCES signer_owners(id) ON DELETE CASCADE,
    canonical_public_key BLOB UNIQUE NOT NULL,
    fingerprint          TEXT NOT NULL,
    enrollment_path      TEXT NOT NULL,
    app_name             TEXT,
    created_at           INTEGER NOT NULL,
    updated_at           INTEGER NOT NULL,
    last_seen_at         INTEGER
);

CREATE TABLE identities (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    signer_owner_id      TEXT    NOT NULL REFERENCES signer_owners(id) ON DELETE CASCADE,
    network_genesis_hash TEXT    NOT NULL,
    ip_identity          INTEGER NOT NULL,
    identity_index       INTEGER NOT NULL,
    label                TEXT    NOT NULL,
    status               TEXT    NOT NULL CHECK(status IN ('pending', 'done')),
    created_at           INTEGER NOT NULL,
    expires_at           INTEGER,
    UNIQUE(network_genesis_hash, signer_owner_id, ip_identity, identity_index),
    UNIQUE(network_genesis_hash, label)
);

CREATE TABLE identity_private_payloads (
    identity_id    INTEGER PRIMARY KEY NOT NULL REFERENCES identities(id) ON DELETE CASCADE,
    cipher_version INTEGER NOT NULL,
    ciphertext     BLOB    NOT NULL,
    nonce          BLOB    NOT NULL
);

CREATE TABLE imported_account_vaults (
    id                   TEXT PRIMARY KEY NOT NULL,
    network_genesis_hash TEXT UNIQUE NOT NULL,
    kdf_algorithm        TEXT NOT NULL,
    kdf_params_json      TEXT NOT NULL,
    salt                 BLOB NOT NULL,
    encrypted_dek        BLOB NOT NULL,
    dek_nonce            BLOB NOT NULL,
    cipher_version       INTEGER NOT NULL DEFAULT 1,
    created_at           INTEGER NOT NULL,
    updated_at           INTEGER NOT NULL
);

CREATE TABLE accounts (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    network_genesis_hash TEXT    NOT NULL,
    label                TEXT    NOT NULL,
    status               TEXT    NOT NULL CHECK(status IN ('pending', 'finalized')),
    source_kind          TEXT    NOT NULL CHECK(source_kind IN ('derived', 'imported')) DEFAULT 'derived',
    signer_owner_id      TEXT    REFERENCES signer_owners(id) ON DELETE CASCADE,
    ip_identity          INTEGER,
    identity_index       INTEGER,
    credential_counter   INTEGER,
    imported_vault_id    TEXT    REFERENCES imported_account_vaults(id) ON DELETE CASCADE,
    import_kind          TEXT,
    source_metadata_json TEXT,
    transaction_hash     TEXT,
    created_at           INTEGER NOT NULL,
    updated_at           INTEGER NOT NULL,
    UNIQUE(network_genesis_hash, label),
    CHECK(
        (source_kind = 'derived'
            AND signer_owner_id IS NOT NULL
            AND ip_identity IS NOT NULL
            AND identity_index IS NOT NULL
            AND credential_counter IS NOT NULL
            AND imported_vault_id IS NULL)
        OR
        (source_kind = 'imported'
            AND signer_owner_id IS NULL
            AND ip_identity IS NULL
            AND identity_index IS NULL
            AND credential_counter IS NULL
            AND imported_vault_id IS NOT NULL)
    )
);

CREATE UNIQUE INDEX accounts_derived_tuple_unique
ON accounts(network_genesis_hash, signer_owner_id, ip_identity, identity_index, credential_counter)
WHERE source_kind = 'derived';

CREATE TABLE derived_account_private_payloads (
    account_id     INTEGER PRIMARY KEY NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    cipher_version INTEGER NOT NULL,
    ciphertext     BLOB    NOT NULL,
    nonce          BLOB    NOT NULL
);

CREATE TABLE imported_account_payloads (
    account_id     INTEGER PRIMARY KEY NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    vault_id       TEXT    NOT NULL REFERENCES imported_account_vaults(id) ON DELETE CASCADE,
    cipher_version INTEGER NOT NULL,
    ciphertext     BLOB    NOT NULL,
    nonce          BLOB    NOT NULL
);

CREATE TABLE governance_key_vaults (
    id                   TEXT PRIMARY KEY NOT NULL,
    network_genesis_hash TEXT UNIQUE NOT NULL,
    kdf_algorithm        TEXT NOT NULL,
    kdf_params_json      TEXT NOT NULL,
    salt                 BLOB NOT NULL,
    encrypted_dek        BLOB NOT NULL,
    dek_nonce            BLOB NOT NULL,
    cipher_version       INTEGER NOT NULL DEFAULT 1,
    created_at           INTEGER NOT NULL,
    updated_at           INTEGER NOT NULL
);

CREATE TABLE governance_keys (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    network_genesis_hash TEXT NOT NULL,
    vault_id             TEXT NOT NULL REFERENCES governance_key_vaults(id) ON DELETE CASCADE,
    created_at           INTEGER NOT NULL,
    updated_at           INTEGER NOT NULL
);

CREATE TABLE governance_key_payloads (
    governance_key_id INTEGER PRIMARY KEY NOT NULL REFERENCES governance_keys(id) ON DELETE CASCADE,
    cipher_version    INTEGER NOT NULL,
    ciphertext        BLOB    NOT NULL,
    nonce             BLOB    NOT NULL
);

INSERT INTO schema_version (version) VALUES (1);
