CREATE TABLE schema_version (
    version INTEGER NOT NULL
);

CREATE TABLE wallet_state (
    key   TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);

CREATE TABLE seeds (
    id         TEXT PRIMARY KEY NOT NULL,
    label      TEXT UNIQUE NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE seed_vaults (
    seed_id            TEXT PRIMARY KEY NOT NULL REFERENCES seeds(id),
    kdf_algorithm      TEXT NOT NULL,
    kdf_params_json    TEXT NOT NULL,
    salt               BLOB NOT NULL,
    encrypted_dek      BLOB NOT NULL,
    dek_nonce          BLOB NOT NULL,
    cipher_version     INTEGER NOT NULL DEFAULT 1,
    payload_ciphertext BLOB NOT NULL,
    payload_nonce      BLOB NOT NULL
);

INSERT INTO schema_version (version) VALUES (1);
