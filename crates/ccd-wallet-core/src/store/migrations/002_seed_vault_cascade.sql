PRAGMA foreign_keys = OFF;

ALTER TABLE seed_vaults RENAME TO seed_vaults_old;

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

INSERT INTO seed_vaults (
    seed_id,
    kdf_algorithm,
    kdf_params_json,
    salt,
    encrypted_dek,
    dek_nonce,
    cipher_version,
    payload_ciphertext,
    payload_nonce
)
SELECT
    seed_id,
    kdf_algorithm,
    kdf_params_json,
    salt,
    encrypted_dek,
    dek_nonce,
    cipher_version,
    payload_ciphertext,
    payload_nonce
FROM seed_vaults_old;

DROP TABLE seed_vaults_old;

UPDATE schema_version SET version = 2;

PRAGMA foreign_keys = ON;
