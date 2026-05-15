CREATE TABLE identities (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    seed_label      TEXT    NOT NULL,
    ip_identity     INTEGER NOT NULL,
    identity_index  INTEGER NOT NULL,
    label           TEXT    NOT NULL,
    status          TEXT    NOT NULL CHECK(status IN ('pending', 'done', 'error')),
    code_uri        TEXT,
    identity_object TEXT,
    created_at      INTEGER NOT NULL,
    UNIQUE(seed_label, ip_identity, identity_index),
    UNIQUE(seed_label, label)
);

UPDATE schema_version SET version = 3;
