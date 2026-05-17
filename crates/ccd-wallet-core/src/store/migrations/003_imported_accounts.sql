PRAGMA foreign_keys = OFF;

ALTER TABLE account_private_payloads RENAME TO account_private_payloads_v2_old;
ALTER TABLE accounts RENAME TO accounts_v2_old;

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
    seed_id              TEXT    REFERENCES seeds(id) ON DELETE CASCADE,
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
            AND seed_id IS NOT NULL
            AND ip_identity IS NOT NULL
            AND identity_index IS NOT NULL
            AND credential_counter IS NOT NULL
            AND imported_vault_id IS NULL)
        OR
        (source_kind = 'imported'
            AND seed_id IS NULL
            AND ip_identity IS NULL
            AND identity_index IS NULL
            AND credential_counter IS NULL
            AND imported_vault_id IS NOT NULL)
    )
);

CREATE UNIQUE INDEX accounts_derived_tuple_unique
ON accounts(network_genesis_hash, seed_id, ip_identity, identity_index, credential_counter)
WHERE source_kind = 'derived';

INSERT INTO accounts (
    id, network_genesis_hash, label, status, source_kind,
    seed_id, ip_identity, identity_index, credential_counter,
    transaction_hash, created_at, updated_at
)
SELECT
    id, network_genesis_hash, label, status, 'derived',
    seed_id, ip_identity, identity_index, credential_counter,
    transaction_hash, created_at, updated_at
FROM accounts_v2_old;

CREATE TABLE account_private_payloads (
    account_id     INTEGER PRIMARY KEY NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    cipher_version INTEGER NOT NULL,
    ciphertext     BLOB    NOT NULL,
    nonce          BLOB    NOT NULL
);

INSERT INTO account_private_payloads (account_id, cipher_version, ciphertext, nonce)
SELECT account_id, cipher_version, ciphertext, nonce
FROM account_private_payloads_v2_old;

DROP TABLE account_private_payloads_v2_old;
DROP TABLE accounts_v2_old;

CREATE TABLE imported_account_payloads (
    account_id     INTEGER PRIMARY KEY NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    vault_id       TEXT    NOT NULL REFERENCES imported_account_vaults(id) ON DELETE CASCADE,
    cipher_version INTEGER NOT NULL,
    ciphertext     BLOB    NOT NULL,
    nonce          BLOB    NOT NULL
);

PRAGMA foreign_keys = ON;

UPDATE schema_version SET version = 3;
