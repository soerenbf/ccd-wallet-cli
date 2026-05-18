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

UPDATE schema_version SET version = 4;
