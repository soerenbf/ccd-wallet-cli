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
    seed_id            TEXT PRIMARY KEY NOT NULL REFERENCES seeds(id) ON DELETE CASCADE,
    kdf_algorithm      TEXT NOT NULL,
    kdf_params_json    TEXT NOT NULL,
    salt               BLOB NOT NULL,
    encrypted_dek      BLOB NOT NULL,
    dek_nonce          BLOB NOT NULL,
    cipher_version     INTEGER NOT NULL DEFAULT 1,
    payload_ciphertext BLOB NOT NULL,
    payload_nonce      BLOB NOT NULL
);

CREATE TABLE identities (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    seed_id              TEXT    NOT NULL REFERENCES seeds(id) ON DELETE CASCADE,
    network_genesis_hash TEXT    NOT NULL,
    ip_identity          INTEGER NOT NULL,
    identity_index       INTEGER NOT NULL,
    label                TEXT    NOT NULL,
    status               TEXT    NOT NULL CHECK(status IN ('pending', 'done')),
    created_at           INTEGER NOT NULL,
    UNIQUE(network_genesis_hash, seed_id, ip_identity, identity_index),
    UNIQUE(network_genesis_hash, label)
);

CREATE TABLE identity_private_payloads (
    identity_id    INTEGER PRIMARY KEY NOT NULL REFERENCES identities(id) ON DELETE CASCADE,
    cipher_version INTEGER NOT NULL,
    ciphertext     BLOB    NOT NULL,
    nonce          BLOB    NOT NULL
);

INSERT INTO schema_version (version) VALUES (1);
