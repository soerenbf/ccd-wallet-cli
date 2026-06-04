//! Protocol-level token command orchestration.

mod admin_roles;
mod holder;
mod lists;
mod lock;
mod metadata;
mod pause;
mod shared;
mod show;

use crate::cli::{
    TokenAdminRolesSubcommand, TokenLockSubcommand, TokenMetadataSubcommand, TokenSubcommand,
};
use anyhow::Result;
use rusqlite::Connection;

/// Run a token command.
pub async fn run(conn: &Connection, command: TokenSubcommand) -> Result<()> {
    match command {
        TokenSubcommand::Show(args) => show::show(conn, *args).await,
        TokenSubcommand::Transfer(args) => holder::transfer(conn, *args).await,
        TokenSubcommand::Mint(args) => holder::mint(conn, *args).await,
        TokenSubcommand::Burn(args) => holder::burn(conn, *args).await,
        TokenSubcommand::AllowList(args) => match args.command {
            crate::cli::TokenListSubcommand::Add(args) => lists::allow_list_add(conn, *args).await,
            crate::cli::TokenListSubcommand::Remove(args) => {
                lists::allow_list_remove(conn, *args).await
            }
        },
        TokenSubcommand::DenyList(args) => match args.command {
            crate::cli::TokenListSubcommand::Add(args) => lists::deny_list_add(conn, *args).await,
            crate::cli::TokenListSubcommand::Remove(args) => {
                lists::deny_list_remove(conn, *args).await
            }
        },
        TokenSubcommand::Pause(args) => pause::pause(conn, *args).await,
        TokenSubcommand::Unpause(args) => pause::unpause(conn, *args).await,
        TokenSubcommand::AdminRoles(command) => match command.command {
            TokenAdminRolesSubcommand::Assign(args) => admin_roles::assign(conn, *args).await,
            TokenAdminRolesSubcommand::Revoke(args) => admin_roles::revoke(conn, *args).await,
        },
        TokenSubcommand::Metadata(command) => match command.command {
            TokenMetadataSubcommand::Update(args) => metadata::update(conn, *args).await,
        },
        TokenSubcommand::Lock(command) => match command.command {
            TokenLockSubcommand::Create(args) => lock::create(conn, *args).await,
            TokenLockSubcommand::Fund(args) => lock::fund(conn, *args).await,
            TokenLockSubcommand::Send(args) => lock::send(conn, *args).await,
            TokenLockSubcommand::Return(args) => lock::return_funds(conn, *args).await,
            TokenLockSubcommand::Cancel(args) => lock::cancel(conn, *args).await,
            TokenLockSubcommand::Show(args) => lock::show(conn, *args).await,
        },
    }
}
