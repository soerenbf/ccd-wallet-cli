use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, params};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityRecord {
    pub id: i64,
    pub network_genesis_hash: String,
    pub seed_label: String,
    pub ip_identity: u32,
    pub identity_index: u32,
    pub label: String,
    pub status: IdentityStatus,
    pub code_uri: Option<String>,
    pub identity_object: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityStatus {
    Pending,
    Done,
    Error,
}

impl IdentityStatus {
    fn as_str(self) -> &'static str {
        match self {
            IdentityStatus::Pending => "pending",
            IdentityStatus::Done => "done",
            IdentityStatus::Error => "error",
        }
    }

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "pending" => Ok(Self::Pending),
            "done" => Ok(Self::Done),
            "error" => Ok(Self::Error),
            other => bail!("unsupported identity status '{other}'"),
        }
    }
}

pub fn next_index(
    conn: &Connection,
    network_genesis_hash: &str,
    seed_label: &str,
    ip_identity: u32,
) -> Result<u32> {
    let max_index: Option<u32> = conn
        .query_row(
            "SELECT MAX(identity_index) FROM identities WHERE network_genesis_hash = ?1 AND seed_label = ?2 AND ip_identity = ?3",
            params![network_genesis_hash, seed_label, ip_identity],
            |row| row.get(0),
        )
        .context("failed to query next identity index")?;

    Ok(max_index.map(|idx| idx + 1).unwrap_or(0))
}

pub fn insert_pending(
    conn: &Connection,
    network_genesis_hash: &str,
    seed_label: &str,
    ip_identity: u32,
    identity_index: u32,
    label: &str,
    code_uri: &str,
) -> Result<i64> {
    if find_by_network_and_label(conn, network_genesis_hash, label)?.is_some() {
        bail!("identity label '{label}' already exists for network '{network_genesis_hash}'");
    }

    if find_by_network_seed_ip_and_index(
        conn,
        network_genesis_hash,
        seed_label,
        ip_identity,
        identity_index,
    )?
    .is_some()
    {
        bail!(
            "identity index {identity_index} for provider {ip_identity} already exists for seed '{seed_label}' on network '{network_genesis_hash}'"
        );
    }

    let created_at = now_unix_seconds()?;
    conn.execute(
        "INSERT INTO identities (
            network_genesis_hash, seed_label, ip_identity, identity_index, label, status, code_uri, identity_object, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, ?8)",
        params![
            network_genesis_hash,
            seed_label,
            ip_identity,
            identity_index,
            label,
            IdentityStatus::Pending.as_str(),
            code_uri,
            created_at,
        ],
    )
    .with_context(|| format!("failed to insert identity '{label}'"))?;

    Ok(conn.last_insert_rowid())
}

pub fn set_done(conn: &Connection, id: i64, identity_object_json: &str) -> Result<()> {
    let affected = conn
        .execute(
            "UPDATE identities SET status = ?1, identity_object = ?2 WHERE id = ?3",
            params![IdentityStatus::Done.as_str(), identity_object_json, id],
        )
        .with_context(|| format!("failed to mark identity {id} done"))?;

    if affected == 0 {
        bail!("identity {id} is not configured");
    }

    Ok(())
}

pub fn set_error(conn: &Connection, id: i64) -> Result<()> {
    let affected = conn
        .execute(
            "UPDATE identities SET status = ?1 WHERE id = ?2",
            params![IdentityStatus::Error.as_str(), id],
        )
        .with_context(|| format!("failed to mark identity {id} error"))?;

    if affected == 0 {
        bail!("identity {id} is not configured");
    }

    Ok(())
}

pub fn find_by_network_and_label(
    conn: &Connection,
    network_genesis_hash: &str,
    label: &str,
) -> Result<Option<IdentityRecord>> {
    conn.query_row(
        "SELECT id, network_genesis_hash, seed_label, ip_identity, identity_index, label, status, code_uri, identity_object, created_at
         FROM identities WHERE network_genesis_hash = ?1 AND label = ?2",
        params![network_genesis_hash, label],
        map_identity_row,
    )
    .optional()
    .with_context(|| {
        format!(
            "failed to query identity '{label}' for network '{network_genesis_hash}'"
        )
    })
}

pub fn find_by_network_seed_ip_and_index(
    conn: &Connection,
    network_genesis_hash: &str,
    seed_label: &str,
    ip_identity: u32,
    identity_index: u32,
) -> Result<Option<IdentityRecord>> {
    conn.query_row(
        "SELECT id, network_genesis_hash, seed_label, ip_identity, identity_index, label, status, code_uri, identity_object, created_at
         FROM identities WHERE network_genesis_hash = ?1 AND seed_label = ?2 AND ip_identity = ?3 AND identity_index = ?4",
        params![network_genesis_hash, seed_label, ip_identity, identity_index],
        map_identity_row,
    )
    .optional()
    .with_context(|| {
        format!(
            "failed to query identity index {identity_index} for seed '{seed_label}', provider {ip_identity}, network '{network_genesis_hash}'"
        )
    })
}

fn map_identity_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<IdentityRecord> {
    let status: String = row.get(6)?;
    Ok(IdentityRecord {
        id: row.get(0)?,
        network_genesis_hash: row.get(1)?,
        seed_label: row.get(2)?,
        ip_identity: row.get(3)?,
        identity_index: row.get(4)?,
        label: row.get(5)?,
        status: IdentityStatus::from_str(&status).map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(
                6,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    err.to_string(),
                )),
            )
        })?,
        code_uri: row.get(7)?,
        identity_object: row.get(8)?,
        created_at: row.get(9)?,
    })
}

fn now_unix_seconds() -> Result<i64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before Unix epoch")?;
    Ok(duration.as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::migrations;

    const MAINNET: &str = "mainnet-hash";
    const TESTNET: &str = "testnet-hash";

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn
    }

    #[test]
    fn next_index_starts_at_zero_and_increments_per_network_seed_and_provider() {
        let conn = conn();
        assert_eq!(next_index(&conn, MAINNET, "seed_a", 7).unwrap(), 0);

        let id = insert_pending(&conn, MAINNET, "seed_a", 7, 0, "id-1", "https://code").unwrap();
        assert!(id > 0);
        assert_eq!(next_index(&conn, MAINNET, "seed_a", 7).unwrap(), 1);
        assert_eq!(next_index(&conn, MAINNET, "seed_a", 8).unwrap(), 0);
        assert_eq!(next_index(&conn, MAINNET, "seed_b", 7).unwrap(), 0);
        assert_eq!(next_index(&conn, TESTNET, "seed_a", 7).unwrap(), 0);
    }

    #[test]
    fn duplicate_network_label_pair_is_rejected() {
        let conn = conn();
        insert_pending(&conn, MAINNET, "seed_a", 7, 0, "identity", "https://code-1").unwrap();

        let err = insert_pending(&conn, MAINNET, "seed_b", 8, 0, "identity", "https://code-2")
            .unwrap_err();
        assert!(err.to_string().contains("already exists"));

        insert_pending(&conn, TESTNET, "seed_b", 8, 0, "identity", "https://code-3").unwrap();
    }

    #[test]
    fn duplicate_network_seed_ip_index_tuple_is_rejected() {
        let conn = conn();
        insert_pending(
            &conn,
            MAINNET,
            "seed_a",
            7,
            0,
            "identity-1",
            "https://code-1",
        )
        .unwrap();

        let err = insert_pending(
            &conn,
            MAINNET,
            "seed_a",
            7,
            0,
            "identity-2",
            "https://code-2",
        )
        .unwrap_err();
        assert!(err.to_string().contains("already exists"));

        insert_pending(
            &conn,
            TESTNET,
            "seed_a",
            7,
            0,
            "identity-3",
            "https://code-3",
        )
        .unwrap();
    }

    #[test]
    fn status_transitions_to_done_and_error() {
        let conn = conn();
        let id =
            insert_pending(&conn, MAINNET, "seed_a", 7, 0, "identity", "https://code").unwrap();

        set_done(&conn, id, r#"{"identityObject":{}}"#).unwrap();
        let record = find_by_network_and_label(&conn, MAINNET, "identity")
            .unwrap()
            .unwrap();
        assert_eq!(record.status, IdentityStatus::Done);
        assert_eq!(
            record.identity_object.as_deref(),
            Some(r#"{"identityObject":{}}"#)
        );

        set_error(&conn, id).unwrap();
        let record = find_by_network_and_label(&conn, MAINNET, "identity")
            .unwrap()
            .unwrap();
        assert_eq!(record.status, IdentityStatus::Error);
    }
}
