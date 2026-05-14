use crate::{
    cli::SeedSubcommand,
    store::{seeds, wallet_state},
};
use anyhow::{Context, Result, bail};
use bip39::{Language, Mnemonic};
use console::Term;
use rusqlite::Connection;
use std::{sync::mpsc, thread, time::Duration};

const SEED_REVEAL_TIMEOUT: Duration = Duration::from_secs(30);

/// Enter the terminal alternate screen buffer (saves normal screen, shows blank buffer).
const ENTER_ALT_SCREEN: &str = "\x1b[?1049h";
/// Leave the terminal alternate screen buffer (restores normal screen).
const LEAVE_ALT_SCREEN: &str = "\x1b[?1049l";

pub trait SeedPrompts {
    fn prompt_seed_phrase(&mut self) -> Result<String>;
    fn prompt_password(&mut self) -> Result<String>;
    fn prompt_password_confirmation(&mut self) -> Result<String>;
    fn prompt_unlock_password(&mut self, label: &str) -> Result<String>;
}

pub trait SeedPhraseRevealer {
    fn reveal(&mut self, label: &str, seed_phrase: &str) -> Result<()>;
}

pub struct TerminalSeedPrompts;

impl SeedPrompts for TerminalSeedPrompts {
    fn prompt_seed_phrase(&mut self) -> Result<String> {
        Ok(rpassword::prompt_password("Enter seed phrase: ")?)
    }

    fn prompt_password(&mut self) -> Result<String> {
        Ok(rpassword::prompt_password("Set password: ")?)
    }

    fn prompt_password_confirmation(&mut self) -> Result<String> {
        Ok(rpassword::prompt_password("Confirm password: ")?)
    }

    fn prompt_unlock_password(&mut self, label: &str) -> Result<String> {
        Ok(rpassword::prompt_password(format!(
            "Password for seed '{label}': "
        ))?)
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
        SeedSubcommand::Add(args) => add(conn, args.label, prompts).await,
        SeedSubcommand::Use(args) => use_seed(conn, args.label).await,
        SeedSubcommand::Show(args) => show(conn, args.label, prompts, revealer).await,
    }
}

async fn add(conn: &Connection, label: String, prompts: &mut impl SeedPrompts) -> Result<()> {
    validate_seed_label(&label)?;

    if seeds::find_by_label(conn, &label)?.is_some() {
        bail!("seed label '{label}' already exists");
    }

    let seed_phrase = normalize_seed_phrase(&prompts.prompt_seed_phrase()?);
    validate_seed_phrase(&seed_phrase)?;

    let password = prompts.prompt_password()?;
    let password_confirmation = prompts.prompt_password_confirmation()?;
    if password != password_confirmation {
        bail!("passwords do not match");
    }

    seeds::add(conn, &label, seed_phrase.as_bytes(), &password)?;

    println!("Seed '{label}' added successfully.");

    Ok(())
}

async fn use_seed(conn: &Connection, label: String) -> Result<()> {
    ensure_seed_exists(conn, &label)?;
    wallet_state::set(conn, wallet_state::ACTIVE_SEED_KEY, &label)?;

    println!("Active seed set to '{label}'.");

    Ok(())
}

async fn show(
    conn: &Connection,
    label: Option<String>,
    prompts: &mut impl SeedPrompts,
    revealer: &mut impl SeedPhraseRevealer,
) -> Result<()> {
    let label = resolve_seed_label(conn, label)?;
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

fn resolve_seed_label(conn: &Connection, label: Option<String>) -> Result<String> {
    match label {
        Some(label) => Ok(label),
        None => wallet_state::get(conn, wallet_state::ACTIVE_SEED_KEY)?.with_context(
            || "no active seed is set; provide a seed label or run `ccd-wallet seed use <LABEL>`",
        ),
    }
}

pub fn normalize_seed_phrase(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn validate_seed_phrase(normalized: &str) -> Result<()> {
    Mnemonic::parse_in_normalized(Language::English, normalized)
        .map(|_| ())
        .map_err(|err| anyhow::anyhow!("invalid seed phrase: {err}"))
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
    use crate::store::migrations;

    const VALID_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[derive(Default)]
    struct TestPrompts {
        seed_phrase: String,
        password: String,
        password_confirmation: String,
        unlock_password: String,
    }

    impl SeedPrompts for TestPrompts {
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
            seed_phrase: VALID_MNEMONIC.to_owned(),
            password: "one".to_owned(),
            password_confirmation: "two".to_owned(),
            unlock_password: String::new(),
        };
        let mut revealer = TestRevealer::default();

        let err = run_with_io(
            &conn,
            SeedSubcommand::Add(crate::cli::SeedAddArgs {
                label: "main_seed".to_owned(),
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
            seed_phrase: "not valid".to_owned(),
            password: "password".to_owned(),
            password_confirmation: "password".to_owned(),
            unlock_password: String::new(),
        };
        let mut revealer = TestRevealer::default();

        assert!(
            run_with_io(
                &conn,
                SeedSubcommand::Add(crate::cli::SeedAddArgs {
                    label: "main_seed".to_owned(),
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
    async fn seed_use_sets_active_seed() {
        let conn = conn();
        add_test_seed(&conn);

        use_seed(&conn, "main_seed".to_owned()).await.unwrap();

        assert_eq!(
            wallet_state::get(&conn, wallet_state::ACTIVE_SEED_KEY).unwrap(),
            Some("main_seed".to_owned())
        );
    }

    #[tokio::test]
    async fn seed_use_rejects_unknown_seed_without_writing_state() {
        let conn = conn();

        assert!(use_seed(&conn, "missing".to_owned()).await.is_err());
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

        show(&conn, None, &mut prompts, &mut revealer)
            .await
            .unwrap();

        assert_eq!(revealer.revealed.len(), 1);
        assert_eq!(revealer.revealed[0].1, VALID_MNEMONIC);
    }

    #[test]
    fn missing_active_seed_is_actionable() {
        let conn = conn();

        let err = resolve_seed_label(&conn, None).unwrap_err();
        assert!(err.to_string().contains("ccd-wallet seed use <LABEL>"));
    }

    #[tokio::test]
    async fn stale_active_seed_is_actionable_before_password_prompt() {
        let conn = conn();
        wallet_state::set(&conn, wallet_state::ACTIVE_SEED_KEY, "missing").unwrap();
        let mut prompts = TestPrompts::default();
        let mut revealer = TestRevealer::default();

        let err = show(&conn, None, &mut prompts, &mut revealer)
            .await
            .unwrap_err();

        assert!(err.to_string().contains("seed 'missing' is not configured"));
        assert!(revealer.revealed.is_empty());
    }

    #[test]
    fn reveal_inner_waits_for_timeout_and_prints_phrase() {
        let term = Term::stdout();
        reveal_seed_phrase_inner(&term, "main_seed", VALID_MNEMONIC, Duration::from_millis(1))
            .unwrap();
    }
}
