//! Browser connect command orchestration and feature wiring.

mod account;
mod contract_init;
mod contract_update;
mod deploy_module;
mod pairing;
mod shared;

use crate::cli::ConnectArgs;
use anyhow::Result;
use ccd_wallet_connect::{ConnectServer, PairingRejection};
use futures_util::FutureExt;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

pub async fn run(conn: Connection, args: ConnectArgs) -> Result<()> {
    let conn = Arc::new(Mutex::new(conn));
    let server_conn = Arc::clone(&conn);
    let account_conn = Arc::clone(&conn);
    let init_conn = Arc::clone(&conn);
    let update_conn = Arc::clone(&conn);
    let deploy_conn = Arc::clone(&conn);
    let server = ConnectServer::new(
        move |request| {
            let conn = Arc::clone(&server_conn);
            async move {
                let conn = conn.lock().map_err(|_| {
                    PairingRejection::new("wallet database connection is unavailable")
                })?;
                match pairing::approve_pairing(&conn, request) {
                    Ok(approval) => Ok(approval),
                    Err(err) => {
                        let message = err.to_string();
                        let _ = cliclack::log::error(&message);
                        Err(PairingRejection::new(message))
                    }
                }
            }
            .boxed()
        },
        move |request| {
            let conn = Arc::clone(&account_conn);
            async move {
                let conn = conn.lock().map_err(|_| {
                    PairingRejection::new("wallet database connection is unavailable")
                })?;
                match account::approve_account_request(&conn, request) {
                    Ok(approval) => Ok(approval),
                    Err(err) => {
                        let message = err.to_string();
                        let _ = cliclack::log::error(&message);
                        Err(PairingRejection::new(message))
                    }
                }
            }
            .boxed()
        },
        move |request| {
            let conn = Arc::clone(&init_conn);
            async move {
                let prepared = contract_init::prepare_request(request)?;
                contract_init::submit_request(Arc::clone(&conn), prepared).await
            }
            .boxed()
        },
        move |request| {
            let conn = Arc::clone(&update_conn);
            async move {
                let prepared = contract_update::prepare_request(request)?;
                contract_update::submit_request(Arc::clone(&conn), prepared).await
            }
            .boxed()
        },
        move |request| {
            let conn = Arc::clone(&deploy_conn);
            async move { deploy_module::submit_request(Arc::clone(&conn), request).await }.boxed()
        },
    );

    cliclack::log::info(format!(
        "Starting ccd-wallet browser pairing session on ws://{}",
        args.bind
    ))?;
    cliclack::log::info("Press Ctrl-C to stop the connect session.")?;

    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        let _ = shutdown_tx.send(());
    });

    server.serve(args.bind, shutdown_rx).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
    use ccd_wallet_connect::ContractUpdateRequest;
    use concordium_rust_sdk::smart_contracts::common::{
        schema::{ContractV1, FunctionV1, ModuleV1, Type, VersionedModuleSchema},
        to_bytes,
    };
    use std::collections::BTreeMap;

    fn isolated_home(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "ccd-wallet-connect-test-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(path.join(".config/ccd-wallet")).unwrap();
        unsafe {
            std::env::set_var("HOME", &path);
        }
        path
    }

    fn write_config(home: &std::path::Path, body: &str) {
        std::fs::write(home.join(".config/ccd-wallet/config.json"), body).unwrap();
    }

    #[test]
    fn resolves_network_entry_by_genesis_hash() {
        let home = isolated_home("resolve-network");
        write_config(
            &home,
            r#"{
                "version": 1,
                "networks": {
                    "alpha": { "node_endpoint": "http://alpha.example", "genesis_hash": "same", "wallet_proxy": null },
                    "beta": { "node_endpoint": "http://beta.example", "genesis_hash": "same", "wallet_proxy": null },
                    "gamma": { "node_endpoint": "http://gamma.example", "genesis_hash": "other", "wallet_proxy": null }
                }
            }"#,
        );

        let (name, entry) = shared::resolve_network_entry("same").unwrap();
        assert_eq!(name, "alpha");
        assert_eq!(entry.node_endpoint, "http://alpha.example");
        assert!(shared::resolve_network_entry("missing").is_err());
    }

    #[test]
    fn parameter_display_decodes_with_schema_and_falls_back_to_hex() {
        let mut receive = BTreeMap::new();
        receive.insert("set".to_owned(), FunctionV1::Parameter(Type::U8));
        let mut contracts = BTreeMap::new();
        contracts.insert(
            "my".to_owned(),
            ContractV1 {
                init: Some(FunctionV1::Parameter(Type::U8)),
                receive,
            },
        );
        let schema = VersionedModuleSchema::V1(ModuleV1 { contracts });
        let schema_base64 = BASE64.encode(to_bytes(&schema));

        let update = ContractUpdateRequest {
            origin: "https://example.com".to_owned(),
            network_genesis_hash: "genesis".to_owned(),
            account_address: "addr".to_owned(),
            contract_address: ccd_wallet_connect::ContractAddress {
                index: 1,
                subindex: 0,
            },
            receive_name: "my.set".to_owned(),
            amount_micro_ccd: "0".to_owned(),
            max_contract_execution_energy: 1000,
            parameter_hex: "2a".to_owned(),
            schema: Some(serde_json::json!({ "base64": schema_base64 })),
            validate: false,
        };
        assert_eq!(contract_update::display_parameter(&update), "42");

        let mut no_schema = update;
        no_schema.schema = None;
        assert_eq!(contract_update::display_parameter(&no_schema), "0x2a");
    }
}
