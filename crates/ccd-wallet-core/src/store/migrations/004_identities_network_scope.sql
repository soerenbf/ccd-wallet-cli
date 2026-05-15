ALTER TABLE identities RENAME TO identities_old;

CREATE TABLE identities (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    network_genesis_hash TEXT    NOT NULL,
    seed_label           TEXT    NOT NULL,
    ip_identity          INTEGER NOT NULL,
    identity_index       INTEGER NOT NULL,
    label                TEXT    NOT NULL,
    status               TEXT    NOT NULL CHECK(status IN ('pending', 'done', 'error')),
    code_uri             TEXT,
    identity_object      TEXT,
    created_at           INTEGER NOT NULL,
    UNIQUE(network_genesis_hash, seed_label, ip_identity, identity_index),
    UNIQUE(network_genesis_hash, label)
);

INSERT INTO identities (
    id,
    network_genesis_hash,
    seed_label,
    ip_identity,
    identity_index,
    label,
    status,
    code_uri,
    identity_object,
    created_at
)
SELECT
    id,
    '',
    seed_label,
    ip_identity,
    identity_index,
    label,
    status,
    code_uri,
    identity_object,
    created_at
FROM identities_old;

DROP TABLE identities_old;

UPDATE schema_version SET version = 4;
