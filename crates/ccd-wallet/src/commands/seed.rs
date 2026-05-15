use crate::{
    cli::SeedSubcommand,
    commands::ui::{SelectItem, select_or_single},
};
use anyhow::{Context, Result, bail};
use bip39::{Language, Mnemonic};
use ccd_wallet_core::store::{accounts, identities, seeds, wallet_state};
use cliclack::{input, password};
use console::Term;
use rusqlite::Connection;
use std::{sync::mpsc, thread, time::Duration};

const SEED_REVEAL_TIMEOUT: Duration = Duration::from_secs(30);

/// Enter the terminal alternate screen buffer (saves normal screen, shows blank buffer).
const ENTER_ALT_SCREEN: &str = "\x1b[?1049h";
/// Leave the terminal alternate screen buffer (restores normal screen).
const LEAVE_ALT_SCREEN: &str = "\x1b[?1049l";

pub trait SeedPrompts {
    fn prompt_seed_label(&mut self, prompt: &str) -> Result<String>;
    fn prompt_seed_label_with_placeholder(
        &mut self,
        prompt: &str,
        placeholder: &str,
    ) -> Result<String>;
    fn select_seed_label(
        &mut self,
        prompt: &str,
        items: &[SelectItem<String>],
        active: Option<&str>,
    ) -> Result<String>;
    fn prompt_seed_phrase(&mut self) -> Result<String>;
    fn prompt_password(&mut self) -> Result<String>;
    fn prompt_password_confirmation(&mut self) -> Result<String>;
    fn prompt_unlock_password(&mut self, label: &str) -> Result<String>;
    fn prompt_remove_confirmation(&mut self, label: &str) -> Result<String>;
}

pub trait SeedPhraseRevealer {
    fn reveal(&mut self, label: &str, seed_phrase: &str) -> Result<()>;
}

pub struct TerminalSeedPrompts;

impl SeedPrompts for TerminalSeedPrompts {
    fn prompt_seed_label(&mut self, prompt: &str) -> Result<String> {
        Ok(input(prompt)
            .validate(|value: &String| {
                if value.is_empty() {
                    Err("Seed label is required.")
                } else {
                    Ok(())
                }
            })
            .interact()?)
    }

    fn prompt_seed_label_with_placeholder(
        &mut self,
        prompt: &str,
        placeholder: &str,
    ) -> Result<String> {
        Ok(input(prompt)
            .placeholder(placeholder)
            .validate(|value: &String| {
                if value.is_empty() {
                    Err("Seed label is required.")
                } else {
                    Ok(())
                }
            })
            .interact()?)
    }

    fn select_seed_label(
        &mut self,
        prompt: &str,
        items: &[SelectItem<String>],
        active: Option<&str>,
    ) -> Result<String> {
        let initial = active.map(str::to_owned);
        select_or_single(prompt, items, initial.as_ref())
    }

    fn prompt_seed_phrase(&mut self) -> Result<String> {
        Ok(password("Enter seed phrase:").mask('▪').interact()?)
    }

    fn prompt_password(&mut self) -> Result<String> {
        Ok(password("Set password:").mask('▪').interact()?)
    }

    fn prompt_password_confirmation(&mut self) -> Result<String> {
        Ok(password("Confirm password:").mask('▪').interact()?)
    }

    fn prompt_unlock_password(&mut self, label: &str) -> Result<String> {
        Ok(password(format!("Password for seed '{label}':"))
            .mask('▪')
            .interact()?)
    }

    fn prompt_remove_confirmation(&mut self, label: &str) -> Result<String> {
        cliclack::log::warning(format!(
            "This will remove seed '{label}' and all seed-owned data."
        ))?;
        Ok(input(format!("Type '{label}' to confirm:"))
            .validate(|value: &String| {
                if value.is_empty() {
                    Err("Confirmation is required.")
                } else {
                    Ok(())
                }
            })
            .interact()?)
    }
}

pub struct TerminalSeedPhraseRevealer;

impl SeedPhraseRevealer for TerminalSeedPhraseRevealer {
    fn reveal(&mut self, label: &str, seed_phrase: &str) -> Result<()> {
        reveal_seed_phrase_until_key_or_timeout(label, seed_phrase, SEED_REVEAL_TIMEOUT)
    }
}

pub async fn run(conn: &Connection, command: SeedSubcommand) -> Result<()> {
    let mut prompts = TerminalSeedPrompts;
    let mut revealer = TerminalSeedPhraseRevealer;
    run_with_io(conn, command, &mut prompts, &mut revealer).await
}

async fn run_with_io(
    conn: &Connection,
    command: SeedSubcommand,
    prompts: &mut impl SeedPrompts,
    revealer: &mut impl SeedPhraseRevealer,
) -> Result<()> {
    match command {
        SeedSubcommand::Add(args) => {
            add(
                conn,
                args.label,
                args.random,
                args.non_interactive,
                prompts,
                revealer,
            )
            .await
        }
        SeedSubcommand::List => list_seeds(conn).await,
        SeedSubcommand::Rename(args) => {
            rename_seed(
                conn,
                args.old_label,
                args.new_label,
                args.non_interactive,
                prompts,
            )
            .await
        }
        SeedSubcommand::Use(args) => {
            use_seed(conn, args.label, args.non_interactive, prompts).await
        }
        SeedSubcommand::Show(args) => {
            show(conn, args.label, args.no_defaults, prompts, revealer).await
        }
        SeedSubcommand::Remove(args) => {
            remove_seed(conn, args.label, args.non_interactive, prompts).await
        }
    }
}

async fn add(
    conn: &Connection,
    label: Option<String>,
    random: bool,
    non_interactive: bool,
    prompts: &mut impl SeedPrompts,
    revealer: &mut impl SeedPhraseRevealer,
) -> Result<()> {
    let label = resolve_required_seed_label(
        label,
        non_interactive,
        prompts,
        "Seed label:",
        "seed label must be provided in --non-interactive mode",
    )?;
    validate_seed_label(&label)?;

    if seeds::find_by_label(conn, &label)?.is_some() {
        bail!("seed label '{label}' already exists");
    }

    let seed_phrase = if random {
        generate_seed_phrase()?
    } else {
        let seed_phrase = normalize_seed_phrase(&prompts.prompt_seed_phrase()?);
        validate_seed_phrase(&seed_phrase)?;
        seed_phrase
    };

    let password = prompts.prompt_password()?;
    let password_confirmation = prompts.prompt_password_confirmation()?;
    if password != password_confirmation {
        bail!("passwords do not match");
    }

    seeds::add(conn, &label, seed_phrase.as_bytes(), &password)?;

    if random {
        revealer.reveal(&label, &seed_phrase)?;
    }

    println!("Seed '{label}' added successfully.");

    Ok(())
}

async fn list_seeds(conn: &Connection) -> Result<()> {
    let active = wallet_state::get(conn, wallet_state::ACTIVE_SEED_KEY)?;
    let seeds = seeds::list(conn)?;
    let identities = identities::list(conn)?;
    let accounts = accounts::list(conn)?;
    for seed in seeds {
        let identity_count = identities
            .iter()
            .filter(|record| record.seed_id == seed.id)
            .count();
        let account_count = accounts
            .iter()
            .filter(|record| record.seed_id == seed.id)
            .count();
        println!(
            "{}",
            render_seed_list_text(
                &seed.label,
                active.as_deref() == Some(seed.label.as_str()),
                identity_count,
                account_count,
            )
        );
    }
    Ok(())
}

fn render_seed_list_text(
    label: &str,
    active: bool,
    identity_count: usize,
    account_count: usize,
) -> String {
    render_seed_text(label, active, identity_count, account_count, true)
}

fn render_seed_selector_text(label: &str, identity_count: usize, account_count: usize) -> String {
    render_seed_text(label, false, identity_count, account_count, false)
}

fn render_seed_text(
    label: &str,
    active: bool,
    identity_count: usize,
    account_count: usize,
    show_active: bool,
) -> String {
    let mut text = format!(
        "{label} — {} • {}",
        format_count(identity_count, "identity", "identities"),
        format_count(account_count, "account", "accounts"),
    );
    if show_active && active {
        text.push_str(" • active");
    }
    text
}

fn format_count(count: usize, singular: &str, plural: &str) -> String {
    let noun = if count == 1 { singular } else { plural };
    format!("{count} {noun}")
}

async fn rename_seed(
    conn: &Connection,
    old_label: Option<String>,
    new_label: Option<String>,
    non_interactive: bool,
    prompts: &mut impl SeedPrompts,
) -> Result<()> {
    let old_label = match old_label {
        Some(label) => label,
        None if non_interactive => bail!("seed label must be provided in --non-interactive mode"),
        None => select_seed_label(conn, prompts)?,
    };
    ensure_seed_exists(conn, &old_label)?;
    let new_label = match new_label {
        Some(label) => label,
        None if non_interactive => {
            bail!("new seed label must be provided in --non-interactive mode")
        }
        None => prompts.prompt_seed_label_with_placeholder("New seed label:", &old_label)?,
    };
    validate_seed_label(&new_label)?;
    seeds::rename(conn, &old_label, &new_label)?;
    if wallet_state::get(conn, wallet_state::ACTIVE_SEED_KEY)?.as_deref()
        == Some(old_label.as_str())
    {
        wallet_state::set(conn, wallet_state::ACTIVE_SEED_KEY, &new_label)?;
    }
    println!("Seed '{old_label}' renamed to '{new_label}'.");
    Ok(())
}

async fn use_seed(
    conn: &Connection,
    label: Option<String>,
    non_interactive: bool,
    prompts: &mut impl SeedPrompts,
) -> Result<()> {
    let label = match label {
        Some(label) => label,
        None if non_interactive => {
            bail!("seed label must be provided in --non-interactive mode")
        }
        None => select_seed_label(conn, prompts)?,
    };
    ensure_seed_exists(conn, &label)?;
    wallet_state::set(conn, wallet_state::ACTIVE_SEED_KEY, &label)?;

    println!("Active seed set to '{label}'.");

    Ok(())
}

async fn remove_seed(
    conn: &Connection,
    label: Option<String>,
    non_interactive: bool,
    prompts: &mut impl SeedPrompts,
) -> Result<()> {
    let label = resolve_required_seed_label(
        label,
        non_interactive,
        prompts,
        "Seed label:",
        "seed label must be provided in --non-interactive mode",
    )?;
    ensure_seed_exists(conn, &label)?;
    let confirmation = prompts.prompt_remove_confirmation(&label)?;
    if confirmation != label {
        bail!("seed removal aborted: confirmation did not match '{label}'");
    }

    seeds::remove(conn, &label)?;
    if wallet_state::get(conn, wallet_state::ACTIVE_SEED_KEY)?.as_deref() == Some(label.as_str()) {
        wallet_state::remove(conn, wallet_state::ACTIVE_SEED_KEY)?;
    }

    println!("Seed '{label}' removed successfully.");

    Ok(())
}

async fn show(
    conn: &Connection,
    label: Option<String>,
    no_defaults: bool,
    prompts: &mut impl SeedPrompts,
    revealer: &mut impl SeedPhraseRevealer,
) -> Result<()> {
    let label = resolve_seed_label(conn, label, no_defaults, prompts)?;
    ensure_seed_exists(conn, &label)?;

    let password = prompts.prompt_unlock_password(&label)?;
    let seed_phrase = seeds::unlock(conn, &label, &password)?;
    let seed_phrase =
        std::str::from_utf8(&seed_phrase).context("stored seed phrase is not UTF-8")?;

    revealer.reveal(&label, seed_phrase)
}

fn ensure_seed_exists(conn: &Connection, label: &str) -> Result<seeds::SeedRecord> {
    seeds::find_by_label(conn, label)?.with_context(|| format!("seed '{label}' is not configured"))
}

fn resolve_required_seed_label(
    label: Option<String>,
    non_interactive: bool,
    prompts: &mut impl SeedPrompts,
    prompt: &str,
    error: &str,
) -> Result<String> {
    match label {
        Some(label) => Ok(label),
        None if non_interactive => bail!("{error}"),
        None => prompts.prompt_seed_label(prompt),
    }
}

fn resolve_seed_label(
    conn: &Connection,
    label: Option<String>,
    no_defaults: bool,
    prompts: &mut impl SeedPrompts,
) -> Result<String> {
    match label {
        Some(label) => Ok(label),
        None if no_defaults => select_seed_label(conn, prompts),
        None => wallet_state::get(conn, wallet_state::ACTIVE_SEED_KEY)?.with_context(
            || "no active seed is set; provide a seed label or run `ccd-wallet seed use <LABEL>`",
        ),
    }
}

fn select_seed_label(conn: &Connection, prompts: &mut impl SeedPrompts) -> Result<String> {
    let seeds = seeds::list(conn)?;
    if seeds.is_empty() {
        bail!("no seeds are configured; run `ccd-wallet seed add <LABEL>` first")
    }
    let identities = identities::list(conn)?;
    let accounts = accounts::list(conn)?;
    let active = wallet_state::get(conn, wallet_state::ACTIVE_SEED_KEY)?;
    let items = seeds
        .iter()
        .map(|seed| {
            let identity_count = identities
                .iter()
                .filter(|record| record.seed_id == seed.id)
                .count();
            let account_count = accounts
                .iter()
                .filter(|record| record.seed_id == seed.id)
                .count();
            SelectItem {
                value: seed.label.clone(),
                label: render_seed_selector_text(&seed.label, identity_count, account_count),
                hint: String::new(),
            }
        })
        .collect::<Vec<_>>();
    if items.len() == 1 {
        return Ok(items[0].value.clone());
    }
    prompts.select_seed_label("Select seed", &items, active.as_deref())
}

pub fn normalize_seed_phrase(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn validate_seed_phrase(normalized: &str) -> Result<()> {
    Mnemonic::parse_in_normalized(Language::English, normalized)
        .map(|_| ())
        .map_err(|err| anyhow::anyhow!("invalid seed phrase: {err}"))
}

pub fn generate_seed_phrase() -> Result<String> {
    Mnemonic::generate_in(Language::English, 24)
        .map(|mnemonic| mnemonic.to_string())
        .map_err(|err| anyhow::anyhow!("failed to generate seed phrase: {err}"))
}

pub fn validate_seed_label(label: &str) -> Result<()> {
    if label.is_empty() {
        bail!("seed label must not be empty");
    }

    if !label
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        bail!("seed labels may contain only ASCII letters, digits, dash, and underscore");
    }

    Ok(())
}

fn reveal_seed_phrase_until_key_or_timeout(
    label: &str,
    seed_phrase: &str,
    timeout: Duration,
) -> Result<()> {
    let term = Term::stdout();
    print!("{ENTER_ALT_SCREEN}");
    term.clear_screen()?;
    term.move_cursor_to(0, 0)?;

    let result = reveal_seed_phrase_inner(&term, label, seed_phrase, timeout);

    term.clear_screen()?;
    term.move_cursor_to(0, 0)?;
    print!("{LEAVE_ALT_SCREEN}");

    result
}

fn reveal_seed_phrase_inner(
    term: &Term,
    label: &str,
    seed_phrase: &str,
    timeout: Duration,
) -> Result<()> {
    term.write_line(&format!("Seed phrase for '{label}':\n"))?;
    term.write_line(&format!("{seed_phrase}\n"))?;
    term.write_line(
        "Copy this now. Press any key to hide. It will hide automatically in 30 seconds.",
    )?;

    wait_for_key_or_timeout(timeout);
    Ok(())
}

fn wait_for_key_or_timeout(timeout: Duration) {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = Term::stdout().read_key();
        let _ = tx.send(());
    });

    let _ = rx.recv_timeout(timeout);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccd_wallet_core::store::migrations;

    const VALID_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[derive(Default)]
    struct TestPrompts {
        seed_label: String,
        selected_active_seed: Option<String>,
        select_seed_calls: usize,
        seed_phrase: String,
        password: String,
        password_confirmation: String,
        unlock_password: String,
        remove_confirmation: String,
    }

    impl SeedPrompts for TestPrompts {
        fn prompt_seed_label(&mut self, _prompt: &str) -> Result<String> {
            Ok(self.seed_label.clone())
        }

        fn prompt_seed_label_with_placeholder(
            &mut self,
            _prompt: &str,
            _placeholder: &str,
        ) -> Result<String> {
            Ok(self.seed_label.clone())
        }

        fn select_seed_label(
            &mut self,
            _prompt: &str,
            _items: &[SelectItem<String>],
            active: Option<&str>,
        ) -> Result<String> {
            self.select_seed_calls += 1;
            self.selected_active_seed = active.map(str::to_owned);
            Ok(self.seed_label.clone())
        }

        fn prompt_seed_phrase(&mut self) -> Result<String> {
            Ok(self.seed_phrase.clone())
        }

        fn prompt_password(&mut self) -> Result<String> {
            Ok(self.password.clone())
        }

        fn prompt_password_confirmation(&mut self) -> Result<String> {
            Ok(self.password_confirmation.clone())
        }

        fn prompt_unlock_password(&mut self, _label: &str) -> Result<String> {
            Ok(self.unlock_password.clone())
        }

        fn prompt_remove_confirmation(&mut self, _label: &str) -> Result<String> {
            Ok(self.remove_confirmation.clone())
        }
    }

    #[derive(Default)]
    struct TestRevealer {
        revealed: Vec<(String, String)>,
    }

    impl SeedPhraseRevealer for TestRevealer {
        fn reveal(&mut self, label: &str, seed_phrase: &str) -> Result<()> {
            self.revealed
                .push((label.to_owned(), seed_phrase.to_owned()));
            Ok(())
        }
    }

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn
    }

    fn add_test_seed(conn: &Connection) {
        seeds::add(conn, "main_seed", VALID_MNEMONIC.as_bytes(), "password").unwrap();
    }

    #[test]
    fn normalizes_seed_phrase_whitespace() {
        assert_eq!(
            normalize_seed_phrase("  abandon\tabandon\nabout  "),
            "abandon abandon about"
        );
    }

    #[test]
    fn validates_valid_mnemonic() {
        validate_seed_phrase(VALID_MNEMONIC).unwrap();
    }

    #[test]
    fn rejects_invalid_mnemonic() {
        assert!(validate_seed_phrase("not a valid seed phrase").is_err());
    }

    #[test]
    fn validates_seed_labels() {
        for label in ["main_seed", "cold-wallet", "Seed123"] {
            validate_seed_label(label).unwrap();
        }

        for label in ["", "main seed", "main.seed", "påse"] {
            assert!(validate_seed_label(label).is_err());
        }
    }

    #[tokio::test]
    async fn password_confirmation_mismatch_does_not_write_seed() {
        let conn = conn();
        let mut prompts = TestPrompts {
            seed_label: String::new(),
            seed_phrase: VALID_MNEMONIC.to_owned(),
            password: "one".to_owned(),
            password_confirmation: "two".to_owned(),
            unlock_password: String::new(),
            remove_confirmation: String::new(),
            ..Default::default()
        };
        let mut revealer = TestRevealer::default();

        let err = run_with_io(
            &conn,
            SeedSubcommand::Add(crate::cli::SeedAddArgs {
                label: Some("main_seed".to_owned()),
                random: false,
                non_interactive: false,
            }),
            &mut prompts,
            &mut revealer,
        )
        .await
        .unwrap_err();
        assert!(err.to_string().contains("passwords do not match"));
        assert!(seeds::list(&conn).unwrap().is_empty());
    }

    #[tokio::test]
    async fn invalid_seed_phrase_does_not_write_seed() {
        let conn = conn();
        let mut prompts = TestPrompts {
            seed_label: String::new(),
            seed_phrase: "not valid".to_owned(),
            password: "password".to_owned(),
            password_confirmation: "password".to_owned(),
            unlock_password: String::new(),
            remove_confirmation: String::new(),
            ..Default::default()
        };
        let mut revealer = TestRevealer::default();

        assert!(
            run_with_io(
                &conn,
                SeedSubcommand::Add(crate::cli::SeedAddArgs {
                    label: Some("main_seed".to_owned()),
                    random: false,
                    non_interactive: false,
                }),
                &mut prompts,
                &mut revealer,
            )
            .await
            .is_err()
        );
        assert!(seeds::list(&conn).unwrap().is_empty());
    }

    #[tokio::test]
    async fn seed_use_without_label_prompts_with_selector() {
        let conn = conn();
        add_test_seed(&conn);
        seeds::add(&conn, "other_seed", VALID_MNEMONIC.as_bytes(), "password").unwrap();
        wallet_state::set(&conn, wallet_state::ACTIVE_SEED_KEY, "main_seed").unwrap();

        let mut prompts = TestPrompts {
            seed_label: "main_seed".to_owned(),
            ..Default::default()
        };
        use_seed(&conn, None, false, &mut prompts).await.unwrap();

        assert_eq!(prompts.select_seed_calls, 1);
        assert_eq!(prompts.selected_active_seed, Some("main_seed".to_owned()));
        assert_eq!(
            wallet_state::get(&conn, wallet_state::ACTIVE_SEED_KEY).unwrap(),
            Some("main_seed".to_owned())
        );
    }

    #[tokio::test]
    async fn seed_use_sets_active_seed() {
        let conn = conn();
        add_test_seed(&conn);

        let mut prompts = TestPrompts::default();
        use_seed(&conn, Some("main_seed".to_owned()), false, &mut prompts)
            .await
            .unwrap();

        assert_eq!(
            wallet_state::get(&conn, wallet_state::ACTIVE_SEED_KEY).unwrap(),
            Some("main_seed".to_owned())
        );
    }

    #[tokio::test]
    async fn seed_use_rejects_unknown_seed_without_writing_state() {
        let conn = conn();

        let mut prompts = TestPrompts::default();
        assert!(
            use_seed(&conn, Some("missing".to_owned()), false, &mut prompts)
                .await
                .is_err()
        );
        assert_eq!(
            wallet_state::get(&conn, wallet_state::ACTIVE_SEED_KEY).unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn seed_show_reveals_phrase_with_correct_password() {
        let conn = conn();
        add_test_seed(&conn);
        let mut prompts = TestPrompts {
            unlock_password: "password".to_owned(),
            ..Default::default()
        };
        let mut revealer = TestRevealer::default();

        show(
            &conn,
            Some("main_seed".to_owned()),
            false,
            &mut prompts,
            &mut revealer,
        )
        .await
        .unwrap();

        assert_eq!(
            revealer.revealed,
            vec![("main_seed".to_owned(), VALID_MNEMONIC.to_owned())]
        );
    }

    #[tokio::test]
    async fn seed_show_wrong_password_does_not_reveal_phrase() {
        let conn = conn();
        add_test_seed(&conn);
        let mut prompts = TestPrompts {
            unlock_password: "wrong".to_owned(),
            ..Default::default()
        };
        let mut revealer = TestRevealer::default();

        assert!(
            show(
                &conn,
                Some("main_seed".to_owned()),
                false,
                &mut prompts,
                &mut revealer,
            )
            .await
            .is_err()
        );

        assert!(revealer.revealed.is_empty());
    }

    #[tokio::test]
    async fn seed_show_without_label_uses_active_seed() {
        let conn = conn();
        add_test_seed(&conn);
        wallet_state::set(&conn, wallet_state::ACTIVE_SEED_KEY, "main_seed").unwrap();
        let mut prompts = TestPrompts {
            unlock_password: "password".to_owned(),
            ..Default::default()
        };
        let mut revealer = TestRevealer::default();

        show(&conn, None, false, &mut prompts, &mut revealer)
            .await
            .unwrap();

        assert_eq!(revealer.revealed.len(), 1);
        assert_eq!(revealer.revealed[0].1, VALID_MNEMONIC);
    }

    #[tokio::test]
    async fn seed_show_no_defaults_skips_selection_when_only_one_seed_exists() {
        let conn = conn();
        add_test_seed(&conn);
        wallet_state::set(&conn, wallet_state::ACTIVE_SEED_KEY, "main_seed").unwrap();
        let mut prompts = TestPrompts {
            unlock_password: "password".to_owned(),
            ..Default::default()
        };
        let mut revealer = TestRevealer::default();

        show(&conn, None, true, &mut prompts, &mut revealer)
            .await
            .unwrap();

        assert_eq!(prompts.select_seed_calls, 0);
        assert_eq!(prompts.selected_active_seed, None);
        assert_eq!(revealer.revealed.len(), 1);
    }

    #[tokio::test]
    async fn seed_show_no_defaults_prompts_and_preselects_active_seed() {
        let conn = conn();
        add_test_seed(&conn);
        seeds::add(&conn, "other_seed", VALID_MNEMONIC.as_bytes(), "password").unwrap();
        wallet_state::set(&conn, wallet_state::ACTIVE_SEED_KEY, "main_seed").unwrap();
        let mut prompts = TestPrompts {
            seed_label: "main_seed".to_owned(),
            unlock_password: "password".to_owned(),
            ..Default::default()
        };
        let mut revealer = TestRevealer::default();

        show(&conn, None, true, &mut prompts, &mut revealer)
            .await
            .unwrap();

        assert_eq!(prompts.selected_active_seed, Some("main_seed".to_owned()));
        assert_eq!(revealer.revealed.len(), 1);
    }

    #[tokio::test]
    async fn seed_add_prompts_for_missing_label_in_interactive_mode() {
        let conn = conn();
        let mut prompts = TestPrompts {
            seed_label: "prompted_seed".to_owned(),
            seed_phrase: VALID_MNEMONIC.to_owned(),
            password: "password".to_owned(),
            password_confirmation: "password".to_owned(),
            ..Default::default()
        };
        let mut revealer = TestRevealer::default();

        run_with_io(
            &conn,
            SeedSubcommand::Add(crate::cli::SeedAddArgs {
                label: None,
                random: false,
                non_interactive: false,
            }),
            &mut prompts,
            &mut revealer,
        )
        .await
        .unwrap();

        assert!(
            seeds::find_by_label(&conn, "prompted_seed")
                .unwrap()
                .is_some()
        );
    }

    #[tokio::test]
    async fn seed_add_missing_label_errors_in_non_interactive_mode() {
        let conn = conn();
        let mut prompts = TestPrompts::default();
        let mut revealer = TestRevealer::default();

        let err = run_with_io(
            &conn,
            SeedSubcommand::Add(crate::cli::SeedAddArgs {
                label: None,
                random: false,
                non_interactive: true,
            }),
            &mut prompts,
            &mut revealer,
        )
        .await
        .unwrap_err();
        assert!(err.to_string().contains("--non-interactive"));
    }

    #[test]
    fn missing_active_seed_is_actionable() {
        let conn = conn();

        let mut prompts = TestPrompts::default();
        let err = resolve_seed_label(&conn, None, false, &mut prompts).unwrap_err();
        assert!(err.to_string().contains("ccd-wallet seed use <LABEL>"));
    }

    #[tokio::test]
    async fn stale_active_seed_is_actionable_before_password_prompt() {
        let conn = conn();
        wallet_state::set(&conn, wallet_state::ACTIVE_SEED_KEY, "missing").unwrap();
        let mut prompts = TestPrompts::default();
        let mut revealer = TestRevealer::default();

        let err = show(&conn, None, false, &mut prompts, &mut revealer)
            .await
            .unwrap_err();

        assert!(err.to_string().contains("seed 'missing' is not configured"));
        assert!(revealer.revealed.is_empty());
    }

    #[test]
    fn generated_seed_phrase_is_valid_mnemonic() {
        let phrase = generate_seed_phrase().unwrap();
        assert_eq!(phrase.split_whitespace().count(), 24);
        validate_seed_phrase(&phrase).unwrap();
    }

    #[tokio::test]
    async fn seed_add_random_generates_stores_and_reveals_phrase() {
        let conn = conn();
        let mut prompts = TestPrompts {
            password: "password".to_owned(),
            password_confirmation: "password".to_owned(),
            ..Default::default()
        };
        let mut revealer = TestRevealer::default();

        run_with_io(
            &conn,
            SeedSubcommand::Add(crate::cli::SeedAddArgs {
                label: Some("random_seed".to_owned()),
                random: true,
                non_interactive: false,
            }),
            &mut prompts,
            &mut revealer,
        )
        .await
        .unwrap();

        assert_eq!(revealer.revealed.len(), 1);
        let generated = &revealer.revealed[0].1;
        assert_eq!(generated.split_whitespace().count(), 24);
        validate_seed_phrase(generated).unwrap();

        let unlocked = seeds::unlock(&conn, "random_seed", "password").unwrap();
        assert_eq!(std::str::from_utf8(&unlocked).unwrap(), generated);
    }

    #[tokio::test]
    async fn seed_add_random_rejects_duplicate_before_revealing() {
        let conn = conn();
        add_test_seed(&conn);
        let mut prompts = TestPrompts {
            password: "password".to_owned(),
            password_confirmation: "password".to_owned(),
            ..Default::default()
        };
        let mut revealer = TestRevealer::default();

        assert!(
            run_with_io(
                &conn,
                SeedSubcommand::Add(crate::cli::SeedAddArgs {
                    label: Some("main_seed".to_owned()),
                    random: true,
                    non_interactive: false,
                }),
                &mut prompts,
                &mut revealer,
            )
            .await
            .is_err()
        );

        assert!(revealer.revealed.is_empty());
    }

    #[tokio::test]
    async fn seed_remove_deletes_seed_and_clears_active_seed() {
        let conn = conn();
        add_test_seed(&conn);
        wallet_state::set(&conn, wallet_state::ACTIVE_SEED_KEY, "main_seed").unwrap();
        let mut prompts = TestPrompts {
            remove_confirmation: "main_seed".to_owned(),
            ..Default::default()
        };

        remove_seed(&conn, Some("main_seed".to_owned()), false, &mut prompts)
            .await
            .unwrap();

        assert!(seeds::find_by_label(&conn, "main_seed").unwrap().is_none());
        assert_eq!(
            wallet_state::get(&conn, wallet_state::ACTIVE_SEED_KEY).unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn seed_remove_confirmation_mismatch_keeps_seed_and_vault() {
        let conn = conn();
        add_test_seed(&conn);
        let mut prompts = TestPrompts {
            remove_confirmation: "wrong".to_owned(),
            ..Default::default()
        };

        assert!(
            remove_seed(&conn, Some("main_seed".to_owned()), false, &mut prompts)
                .await
                .is_err()
        );

        assert!(seeds::find_by_label(&conn, "main_seed").unwrap().is_some());
        let vault_count: u32 = conn
            .query_row("SELECT COUNT(*) FROM seed_vaults", [], |row| row.get(0))
            .unwrap();
        assert_eq!(vault_count, 1);
    }

    #[tokio::test]
    async fn seed_remove_inactive_seed_leaves_active_seed_unchanged() {
        let conn = conn();
        add_test_seed(&conn);
        seeds::add(&conn, "old_seed", VALID_MNEMONIC.as_bytes(), "password").unwrap();
        wallet_state::set(&conn, wallet_state::ACTIVE_SEED_KEY, "main_seed").unwrap();
        let mut prompts = TestPrompts {
            remove_confirmation: "old_seed".to_owned(),
            ..Default::default()
        };

        remove_seed(&conn, Some("old_seed".to_owned()), false, &mut prompts)
            .await
            .unwrap();

        assert_eq!(
            wallet_state::get(&conn, wallet_state::ACTIVE_SEED_KEY).unwrap(),
            Some("main_seed".to_owned())
        );
    }

    #[tokio::test]
    async fn seed_rename_updates_active_seed() {
        let conn = conn();
        add_test_seed(&conn);
        wallet_state::set(&conn, wallet_state::ACTIVE_SEED_KEY, "main_seed").unwrap();
        let mut prompts = TestPrompts::default();

        rename_seed(
            &conn,
            Some("main_seed".to_owned()),
            Some("daily".to_owned()),
            false,
            &mut prompts,
        )
        .await
        .unwrap();

        assert_eq!(
            wallet_state::get(&conn, wallet_state::ACTIVE_SEED_KEY).unwrap(),
            Some("daily".to_owned())
        );
        assert!(seeds::find_by_label(&conn, "daily").unwrap().is_some());
    }

    #[test]
    fn render_seed_list_text_marks_active_seed() {
        assert_eq!(
            render_seed_list_text("main_seed", true, 2, 3),
            "main_seed — 2 identities • 3 accounts • active"
        );
        assert_eq!(
            render_seed_list_text("other_seed", false, 1, 0),
            "other_seed — 1 identity • 0 accounts"
        );
        assert_eq!(
            render_seed_selector_text("main_seed", 2, 3),
            "main_seed — 2 identities • 3 accounts"
        );
    }

    #[test]
    fn format_count_handles_singular_and_plural() {
        assert_eq!(format_count(1, "account", "accounts"), "1 account");
        assert_eq!(format_count(2, "account", "accounts"), "2 accounts");
    }

    #[test]
    fn reveal_inner_waits_for_timeout_and_prints_phrase() {
        let term = Term::stdout();
        reveal_seed_phrase_inner(&term, "main_seed", VALID_MNEMONIC, Duration::from_millis(1))
            .unwrap();
    }
}
