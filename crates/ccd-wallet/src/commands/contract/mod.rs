//! Smart contract command orchestration.

mod deploy_module;
mod download_module;
mod init;
mod invoke;
mod parameter_template;
mod show;
mod update;

use crate::cli::ContractSubcommand;
use anyhow::Result;
use rusqlite::Connection;

pub async fn run(conn: &Connection, command: ContractSubcommand) -> Result<()> {
    match command {
        ContractSubcommand::DeployModule(args) => deploy_module::deploy_module(conn, *args).await,
        ContractSubcommand::Init(args) => init::init(conn, *args).await,
        ContractSubcommand::Update(args) => update::update(conn, *args).await,
        ContractSubcommand::Invoke(args) => invoke::invoke(conn, *args).await,
        ContractSubcommand::Show(args) => show::show(conn, *args).await,
        ContractSubcommand::ParameterTemplate(args) => {
            parameter_template::parameter_template(conn, *args).await
        }
        ContractSubcommand::DownloadModule(args) => {
            download_module::download_module(conn, *args).await
        }
    }
}
