ALTER TABLE identities ADD COLUMN expires_at INTEGER;

CREATE TABLE accounts (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    seed_id              TEXT    NOT NULL REFERENCES seeds(id) ON DELETE CASCADE,
    network_genesis_hash TEXT    NOT NULL,
    ip_identity          INTEGER NOT NULL,
    identity_index       INTEGER NOT NULL,
    credential_counter   INTEGER NOT NULL,
    label                TEXT    NOT NULL,
    status               TEXT    NOT NULL CHECK(status IN ('pending', 'finalized')),
    transaction_hash     TEXT,
    created_at           INTEGER NOT NULL,
    updated_at           INTEGER NOT NULL,
    UNIQUE(network_genesis_hash, seed_id, ip_identity, identity_index, credential_counter),
    UNIQUE(network_genesis_hash, label)
);

CREATE TABLE account_private_payloads (
    account_id     INTEGER PRIMARY KEY NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    cipher_version INTEGER NOT NULL,
    ciphertext     BLOB    NOT NULL,
    nonce          BLOB    NOT NULL
);

UPDATE schema_version SET version = 2;
