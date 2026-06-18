//! Shared command-input preparation and resolution primitives.
//!
//! Clap-facing command structs should keep parsing simple, usually with
//! `Option<T>` for omitted values. Command implementations can then convert
//! those options into the semantic wrappers in this module so execution code is
//! explicit about when omission means prompt, default, or genuine absence.

use std::{fmt, future::Future, str::FromStr};

use anyhow::{Context, Result, bail};
use ccd_wallet_core::config;
use chrono::{DateTime, Utc};
use clap::Args;
use concordium_rust_sdk::{
    common::types::{AccountAddress, Amount, Timestamp},
    smart_contracts::common::ContractAddress,
    v2,
};

use crate::smart_contracts::shared::parse_contract_address;

/// Whether a command may prompt and whether it may fill active defaults.
///
/// `InputMode` centralizes the behavior controlled by common flags such as
/// `--non-interactive` and `--no-defaults`. Non-interactive mode always disables
/// prompts and defaults so commands do not silently use active defaults in
/// machine-oriented flows.
///
/// # Examples
///
/// ```ignore
/// let mode = InputMode::from_flags(false, false);
/// assert!(mode.prompts_allowed());
/// assert!(mode.defaults_allowed());
///
/// let mode = InputMode::from_flags(true, false);
/// assert!(!mode.prompts_allowed());
/// assert!(!mode.defaults_allowed());
/// ```
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct InputMode {
    prompt_policy: PromptPolicy,
    default_policy: DefaultPolicy,
}

impl InputMode {
    /// Build an input mode from the shared command flags.
    ///
    /// # Arguments
    ///
    /// * `non_interactive` - Whether `--non-interactive` was supplied.
    /// * `no_defaults` - Whether `--no-defaults` was supplied.
    ///
    /// # Returns
    ///
    /// An input mode where non-interactive mode disables both prompting and
    /// default filling. Interactive mode allows prompts and allows defaults
    /// unless `no_defaults` is set.
    ///
    /// # Errors
    ///
    /// This function never errors.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mode = InputMode::from_flags(false, true);
    /// assert!(mode.prompts_allowed());
    /// assert!(!mode.defaults_allowed());
    /// ```
    pub(crate) const fn from_flags(non_interactive: bool, no_defaults: bool) -> Self {
        if non_interactive {
            Self::non_interactive()
        } else if no_defaults {
            Self {
                prompt_policy: PromptPolicy::Allow,
                default_policy: DefaultPolicy::Forbid,
            }
        } else {
            Self::interactive()
        }
    }

    /// Return the standard interactive mode with prompts and defaults enabled.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mode = InputMode::interactive();
    /// assert!(mode.prompts_allowed());
    /// assert!(mode.defaults_allowed());
    /// ```
    pub(crate) const fn interactive() -> Self {
        Self {
            prompt_policy: PromptPolicy::Allow,
            default_policy: DefaultPolicy::Allow,
        }
    }

    /// Return the standard non-interactive mode with prompts and defaults disabled.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mode = InputMode::non_interactive();
    /// assert!(!mode.prompts_allowed());
    /// assert!(!mode.defaults_allowed());
    /// ```
    pub(crate) const fn non_interactive() -> Self {
        Self {
            prompt_policy: PromptPolicy::Forbid,
            default_policy: DefaultPolicy::Forbid,
        }
    }

    /// Return whether missing promptable values may ask the user.
    pub(crate) const fn prompts_allowed(self) -> bool {
        matches!(self.prompt_policy, PromptPolicy::Allow)
    }

    /// Return whether missing defaultable values may use active defaults.
    pub(crate) const fn defaults_allowed(self) -> bool {
        matches!(self.default_policy, DefaultPolicy::Allow)
    }
}

/// Prompt behavior for missing promptable values.
/// Shared clap flags that control prompting and default filling.
///
/// Embed this with `#[command(flatten)]` in clap command structs that support
/// `--non-interactive` and `--no-defaults`.
#[derive(Debug, Clone, Args)]
pub(crate) struct InputModeArgs {
    /// Fail instead of prompting for missing values.
    #[arg(long = "non-interactive")]
    pub(crate) non_interactive: bool,

    /// Disable silent use of active defaults and force explicit selection.
    #[arg(long = "no-defaults")]
    pub(crate) no_defaults: bool,
}

impl From<&InputModeArgs> for InputMode {
    fn from(args: &InputModeArgs) -> Self {
        Self::from_flags(args.non_interactive, args.no_defaults)
    }
}

impl From<InputModeArgs> for InputMode {
    fn from(args: InputModeArgs) -> Self {
        Self::from(&args)
    }
}

/// Shared clap flags for commands that accept either a configured network or a node endpoint.
///
/// The public flag names and conflict behavior match the existing command
/// surface: `--network <NAME>` conflicts with `--node <ENDPOINT>`.
#[derive(Debug, Clone, Args)]
pub(crate) struct NetworkNodeArgs {
    /// Registered network name to resolve from the config store.
    #[arg(long = "network", conflicts_with = "node", value_name = "NAME")]
    pub(crate) network: Option<NetworkName>,

    /// Concordium node gRPC endpoint.
    #[arg(
        long = "node",
        env = config::NODE_ENDPOINT_ENV,
        conflicts_with = "network",
        value_name = "ENDPOINT"
    )]
    pub(crate) node: Option<v2::Endpoint>,
}

/// Shared clap flag for commands that select a configured network only.
#[derive(Debug, Clone, Args)]
pub(crate) struct NetworkOnlyArgs {
    /// Registered network name to resolve from the config store.
    #[arg(long = "network", value_name = "NAME")]
    pub(crate) network: Option<NetworkName>,
}

/// Shared clap flag for transaction-submitting commands that may skip waiting.
#[derive(Debug, Clone, Args)]
pub(crate) struct SubmissionWaitArgs {
    /// Return after submission instead of waiting for finalization.
    #[arg(long = "no-wait")]
    pub(crate) no_wait: bool,
}

impl From<&SubmissionWaitArgs> for FinalizationPolicy {
    fn from(args: &SubmissionWaitArgs) -> Self {
        Self::from_no_wait(args.no_wait)
    }
}

impl From<SubmissionWaitArgs> for FinalizationPolicy {
    fn from(args: SubmissionWaitArgs) -> Self {
        Self::from(&args)
    }
}

/// Local account label used for signing-account inputs.
///
/// Unlike account references, signing-account labels intentionally reject raw
/// Concordium account addresses because the wallet needs a local signer.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) struct AccountLabel(String);

impl AccountLabel {
    /// Return the underlying label as a string slice.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AccountLabel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for AccountLabel {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        if value.parse::<AccountAddress>().is_ok() {
            bail!("account label must be a local wallet label, not a raw account address");
        }
        validate_cli_label("account label", value)?;
        Ok(Self(value.to_owned()))
    }
}

/// Configured network name supplied on the command line.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) struct NetworkName(String);

impl NetworkName {
    /// Return the underlying network name as a string slice.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NetworkName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for NetworkName {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        validate_cli_label("network name", value)?;
        Ok(Self(value.to_owned()))
    }
}

/// Local key-source label supplied on the command line.
#[allow(dead_code)]
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) struct KeySourceLabel(String);

impl fmt::Display for KeySourceLabel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for KeySourceLabel {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        validate_cli_label("key-source label", value)?;
        Ok(Self(value.to_owned()))
    }
}

/// Account input that can refer to either a raw chain address or a local label.
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum AccountReference {
    /// A raw Concordium account address.
    Address(AccountAddress),
    /// A finalized local account label.
    Label(AccountLabel),
}

impl fmt::Display for AccountReference {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Address(address) => address.fmt(formatter),
            Self::Label(label) => label.fmt(formatter),
        }
    }
}

impl FromStr for AccountReference {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        if let Ok(address) = value.parse::<AccountAddress>() {
            return Ok(Self::Address(address));
        }
        Ok(Self::Label(value.parse()?))
    }
}

/// Contract address input parsed with the CLI's accepted address forms.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) struct ContractAddressInput(ContractAddress);

impl ContractAddressInput {
    /// Return the parsed contract address.
    pub(crate) const fn address(self) -> ContractAddress {
        self.0
    }
}

impl FromStr for ContractAddressInput {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        parse_contract_address(value).map(Self)
    }
}

/// Token amount input whose final raw amount depends on token decimals.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) struct TokenAmountInput(String);

impl TokenAmountInput {
    /// Return the unresolved decimal amount text.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TokenAmountInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for TokenAmountInput {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            bail!("token amount must not be empty");
        }
        if trimmed.starts_with('-') {
            bail!("token amount must not be negative");
        }
        Ok(Self(trimmed.to_owned()))
    }
}

/// A scheduled-transfer release entry supplied as `RFC3339=CCD`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct ReleaseScheduleEntryInput {
    timestamp_text: String,
    amount_text: String,
}

impl ReleaseScheduleEntryInput {
    /// Return the parsed release timestamp.
    pub(crate) fn timestamp(&self) -> Result<Timestamp> {
        let datetime = DateTime::parse_from_rfc3339(&self.timestamp_text).with_context(|| {
            format!(
                "invalid release timestamp '{}'; use an RFC3339 instant",
                self.timestamp_text
            )
        })?;
        let millis = datetime.with_timezone(&Utc).timestamp_millis();
        if millis < 0 {
            bail!("release timestamp must not be before the unix epoch");
        }
        Ok(Timestamp::from_timestamp_millis(millis as u64))
    }

    /// Return the parsed CCD amount.
    pub(crate) fn amount(&self) -> Result<Amount> {
        parse_decimal_ccd_amount_text(&self.amount_text)
    }
}

impl fmt::Display for ReleaseScheduleEntryInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}={}", self.timestamp_text, self.amount_text)
    }
}

impl FromStr for ReleaseScheduleEntryInput {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        let trimmed = value.trim();
        let Some((timestamp_text, amount_text)) = trimmed.split_once('=') else {
            bail!("release entry must use RFC3339=CCD format");
        };
        let timestamp_text = timestamp_text.trim();
        let amount_text = amount_text.trim();
        if timestamp_text.is_empty() || amount_text.is_empty() {
            bail!("release entry must use RFC3339=CCD format");
        }
        let parsed = Self {
            timestamp_text: timestamp_text.to_owned(),
            amount_text: amount_text.to_owned(),
        };
        let _ = parsed.timestamp()?;
        let _ = parsed.amount()?;
        Ok(parsed)
    }
}

fn validate_cli_label(kind: &str, label: &str) -> Result<()> {
    if label.is_empty() {
        bail!("{kind} must not be empty");
    }
    if !label
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        bail!("{kind} may contain only letters, digits, dash, or underscore");
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum PromptPolicy {
    /// Interactive prompts are allowed.
    Allow,
    /// Interactive prompts are forbidden.
    Forbid,
}

/// Default-filling behavior for missing defaultable values.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum DefaultPolicy {
    /// Active defaults may be used.
    Allow,
    /// Active defaults may not be used.
    Forbid,
}

/// Where a resolved command input value came from.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum ResolvedSource {
    /// The value was supplied explicitly by the caller.
    Explicit,
    /// The value was obtained through an interactive prompt or selector.
    Prompt,
    /// The value was filled from an active default or contextual inference.
    Default,
}

/// A command input value after prompt/default resolution.
///
/// The source metadata lets command handlers and tests assert that omitted
/// values followed the intended path without changing the value's domain type.
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct Resolved<T> {
    /// The resolved domain value.
    pub(crate) value: T,
    /// The resolution path that produced `value`.
    pub(crate) source: ResolvedSource,
}

impl<T> Resolved<T> {
    /// Construct a resolved value with source metadata.
    ///
    /// # Arguments
    ///
    /// * `value` - The resolved domain value.
    /// * `source` - The resolution path that produced the value.
    ///
    /// # Returns
    ///
    /// A `Resolved<T>` containing both fields.
    pub(crate) const fn new(value: T, source: ResolvedSource) -> Self {
        Self { value, source }
    }

    /// Return the resolved value and discard source metadata.
    pub(crate) fn into_value(self) -> T {
        self.value
    }
}

/// A required command input that may be prompted for when omitted.
///
/// Missing values are resolved only when the caller supplies a prompt provider.
/// Non-interactive modes return an actionable error instead of prompting.
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum Promptable<T> {
    /// The value was supplied explicitly.
    Provided(T),
    /// The value is missing and may need an interactive prompt.
    Missing { value_name: &'static str },
}

impl<T> Promptable<T> {
    /// Convert a clap-parsed option into a promptable input.
    ///
    /// # Arguments
    ///
    /// * `value` - The optional value parsed by clap.
    /// * `value_name` - User-facing value name used in non-interactive errors.
    ///
    /// # Returns
    ///
    /// `Provided` when `value` is `Some`, otherwise `Missing`.
    pub(crate) fn from_option(value: Option<T>, value_name: &'static str) -> Self {
        match value {
            Some(value) => Self::Provided(value),
            None => Self::Missing { value_name },
        }
    }

    /// Resolve the input with a synchronous prompt provider.
    ///
    /// # Arguments
    ///
    /// * `mode` - Shared input mode controlling whether prompts are allowed.
    /// * `prompt` - Provider called only when the value is missing and prompts
    ///   are allowed.
    ///
    /// # Errors
    ///
    /// Returns an error if the value is missing and prompts are disabled, or if
    /// the prompt provider fails.
    pub(crate) fn resolve_with(
        self,
        mode: InputMode,
        prompt: impl FnOnce() -> Result<T>,
    ) -> Result<Resolved<T>> {
        match self {
            Self::Provided(value) => Ok(Resolved::new(value, ResolvedSource::Explicit)),
            Self::Missing { value_name: _ } if mode.prompts_allowed() => {
                prompt().map(|value| Resolved::new(value, ResolvedSource::Prompt))
            }
            Self::Missing { value_name } => missing_value_error(value_name),
        }
    }

    /// Resolve the input with an asynchronous prompt provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the value is missing and prompts are disabled, or if
    /// the asynchronous prompt provider fails.
    pub(crate) async fn resolve_with_async<F, Fut>(
        self,
        mode: InputMode,
        prompt: F,
    ) -> Result<Resolved<T>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        match self {
            Self::Provided(value) => Ok(Resolved::new(value, ResolvedSource::Explicit)),
            Self::Missing { value_name: _ } if mode.prompts_allowed() => prompt()
                .await
                .map(|value| Resolved::new(value, ResolvedSource::Prompt)),
            Self::Missing { value_name } => missing_value_error(value_name),
        }
    }
}

/// A command input that may use an active default when omitted.
///
/// Defaults are consulted only when callers provide a default provider at
/// resolution time and the shared input mode allows defaults.
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum Defaultable<T> {
    /// The value was supplied explicitly.
    Provided(T),
    /// The value is missing and may need an active default or prompt fallback.
    Missing { value_name: &'static str },
}

impl<T> Defaultable<T> {
    /// Convert a clap-parsed option into a defaultable input.
    pub(crate) fn from_option(value: Option<T>, value_name: &'static str) -> Self {
        match value {
            Some(value) => Self::Provided(value),
            None => Self::Missing { value_name },
        }
    }

    /// Resolve with a synchronous default provider and no prompt fallback.
    ///
    /// # Errors
    ///
    /// Returns an error if no explicit value is present and defaults are
    /// disabled, the provider fails, or the provider returns no active default.
    #[allow(dead_code)]
    pub(crate) fn resolve_with_default(
        self,
        mode: InputMode,
        default: impl FnOnce() -> Result<Option<T>>,
    ) -> Result<Resolved<T>> {
        match self {
            Self::Provided(value) => Ok(Resolved::new(value, ResolvedSource::Explicit)),
            Self::Missing { value_name } if mode.defaults_allowed() => match default()? {
                Some(value) => Ok(Resolved::new(value, ResolvedSource::Default)),
                None => missing_value_error(value_name),
            },
            Self::Missing { value_name } => missing_value_error(value_name),
        }
    }

    /// Resolve with a synchronous default provider and prompt fallback.
    ///
    /// Defaults are tried first when enabled. If there is no active default, an
    /// interactive prompt may still supply the value when prompts are allowed.
    ///
    /// # Errors
    ///
    /// Returns an error if neither explicit, default, nor prompt resolution can
    /// produce a value, or if a provider fails.
    pub(crate) fn resolve_with_default_or_prompt(
        self,
        mode: InputMode,
        default: impl FnOnce() -> Result<Option<T>>,
        prompt: impl FnOnce() -> Result<T>,
    ) -> Result<Resolved<T>> {
        match self {
            Self::Provided(value) => Ok(Resolved::new(value, ResolvedSource::Explicit)),
            Self::Missing { value_name } => {
                if mode.defaults_allowed()
                    && let Some(value) = default()?
                {
                    return Ok(Resolved::new(value, ResolvedSource::Default));
                }
                if mode.prompts_allowed() {
                    return prompt().map(|value| Resolved::new(value, ResolvedSource::Prompt));
                }
                missing_value_error(value_name)
            }
        }
    }

    /// Resolve with an asynchronous default provider and no prompt fallback.
    ///
    /// # Errors
    ///
    /// Returns an error if no explicit value is present and defaults are
    /// disabled, the provider fails, or the provider returns no active default.
    #[allow(dead_code)]
    pub(crate) async fn resolve_with_default_async<F, Fut>(
        self,
        mode: InputMode,
        default: F,
    ) -> Result<Resolved<T>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Option<T>>>,
    {
        match self {
            Self::Provided(value) => Ok(Resolved::new(value, ResolvedSource::Explicit)),
            Self::Missing { value_name } if mode.defaults_allowed() => match default().await? {
                Some(value) => Ok(Resolved::new(value, ResolvedSource::Default)),
                None => missing_value_error(value_name),
            },
            Self::Missing { value_name } => missing_value_error(value_name),
        }
    }

    /// Resolve with asynchronous default and prompt providers.
    ///
    /// # Errors
    ///
    /// Returns an error if neither explicit, default, nor prompt resolution can
    /// produce a value, or if a provider fails.
    #[allow(dead_code)]
    pub(crate) async fn resolve_with_default_or_prompt_async<DF, Dfut, PF, Pfut>(
        self,
        mode: InputMode,
        default: DF,
        prompt: PF,
    ) -> Result<Resolved<T>>
    where
        DF: FnOnce() -> Dfut,
        Dfut: Future<Output = Result<Option<T>>>,
        PF: FnOnce() -> Pfut,
        Pfut: Future<Output = Result<T>>,
    {
        match self {
            Self::Provided(value) => Ok(Resolved::new(value, ResolvedSource::Explicit)),
            Self::Missing { value_name } => {
                if mode.defaults_allowed()
                    && let Some(value) = default().await?
                {
                    return Ok(Resolved::new(value, ResolvedSource::Default));
                }
                if mode.prompts_allowed() {
                    return prompt()
                        .await
                        .map(|value| Resolved::new(value, ResolvedSource::Prompt));
                }
                missing_value_error(value_name)
            }
        }
    }
}

/// Whether a transaction-submitting command should wait for finalization.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum FinalizationPolicy {
    /// Submit and wait until the node reports finalization.
    Wait,
    /// Submit and return after successful submission.
    SubmitOnly,
}

impl FinalizationPolicy {
    /// Build a finalization policy from the shared `--no-wait` flag.
    pub(crate) const fn from_no_wait(no_wait: bool) -> Self {
        if no_wait {
            Self::SubmitOnly
        } else {
            Self::Wait
        }
    }

    /// Return whether callers should wait for transaction finalization.
    pub(crate) const fn should_wait(self) -> bool {
        matches!(self, Self::Wait)
    }
}

/// Whether a command should perform optional pre-submission validation.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum ValidationPolicy {
    /// Perform validation.
    Validate,
    /// Skip validation.
    Skip,
}

impl ValidationPolicy {
    /// Build a validation policy from a `--no-validate` style flag.
    pub(crate) const fn from_no_validate(no_validate: bool) -> Self {
        if no_validate {
            Self::Skip
        } else {
            Self::Validate
        }
    }

    /// Return whether validation should run.
    pub(crate) const fn should_validate(self) -> bool {
        matches!(self, Self::Validate)
    }
}

fn parse_decimal_ccd_amount_text(value: &str) -> Result<Amount> {
    let value = value.trim();
    if value.is_empty() {
        bail!("amount must not be empty");
    }
    if value.starts_with('-') {
        bail!("amount must not be negative");
    }
    let mut parts = value.split('.');
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next();
    if parts.next().is_some() {
        bail!("amount must be a decimal CCD value such as 0, 1, or 1.25");
    }
    if whole.is_empty() || !whole.chars().all(|ch| ch.is_ascii_digit()) {
        bail!("amount whole CCD part must contain digits only");
    }
    let whole_micro = whole
        .parse::<u64>()?
        .checked_mul(1_000_000)
        .context("amount is too large")?;
    let fraction_micro = match fraction {
        None => 0,
        Some(fraction) => {
            if fraction.is_empty() {
                bail!("amount fractional CCD part must contain digits only");
            }
            if !fraction.chars().all(|ch| ch.is_ascii_digit()) {
                bail!("amount fractional CCD part must contain digits only");
            }
            if fraction.len() > 6 {
                bail!("amount must use at most 6 fractional digits");
            }
            let padded = format!("{fraction:0<6}");
            padded.parse::<u64>()?
        }
    };
    let micro_ccd = whole_micro
        .checked_add(fraction_micro)
        .context("amount is too large")?;
    Ok(Amount::from_micro_ccd(micro_ccd))
}

fn missing_value_error<T>(value_name: &'static str) -> Result<T> {
    bail!("missing required command-line value: {value_name}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Debug, Parser)]
    struct InputModeCommand {
        #[command(flatten)]
        input_mode: InputModeArgs,
    }

    #[derive(Debug, Parser)]
    struct NetworkNodeCommand {
        #[command(flatten)]
        network_node: NetworkNodeArgs,
    }

    #[derive(Debug, Parser)]
    struct NetworkOnlyCommand {
        #[command(flatten)]
        network: NetworkOnlyArgs,
    }

    #[derive(Debug, Parser)]
    struct SubmissionCommand {
        #[command(flatten)]
        submission: SubmissionWaitArgs,
    }

    #[test]
    fn promptable_uses_explicit_value_without_prompting() -> Result<()> {
        let resolved = Promptable::from_option(Some("alice"), "account")
            .resolve_with(InputMode::non_interactive(), || Ok("bob"))?;

        assert_eq!(resolved.value, "alice");
        assert_eq!(resolved.source, ResolvedSource::Explicit);
        Ok(())
    }

    #[test]
    fn promptable_prompts_in_interactive_mode() -> Result<()> {
        let resolved = Promptable::from_option(None, "account")
            .resolve_with(InputMode::interactive(), || Ok("alice"))?;

        assert_eq!(resolved.value, "alice");
        assert_eq!(resolved.source, ResolvedSource::Prompt);
        Ok(())
    }

    #[test]
    fn promptable_errors_in_non_interactive_mode() {
        let err = Promptable::<String>::from_option(None, "account")
            .resolve_with(InputMode::non_interactive(), || Ok(String::from("alice")))
            .unwrap_err();

        assert!(err.to_string().contains("account"));
    }

    #[test]
    fn defaultable_uses_default_when_allowed() -> Result<()> {
        let resolved = Defaultable::from_option(None, "network")
            .resolve_with_default(InputMode::interactive(), || Ok(Some("testnet")))?;

        assert_eq!(resolved.value, "testnet");
        assert_eq!(resolved.source, ResolvedSource::Default);
        Ok(())
    }

    #[test]
    fn no_defaults_preserves_prompt_fallback() -> Result<()> {
        let mode = InputMode::from_flags(false, true);
        let resolved = Defaultable::from_option(None, "network").resolve_with_default_or_prompt(
            mode,
            || Ok(Some("mainnet")),
            || Ok("testnet"),
        )?;

        assert_eq!(resolved.value, "testnet");
        assert_eq!(resolved.source, ResolvedSource::Prompt);
        Ok(())
    }

    #[test]
    fn non_interactive_disables_default_filling() {
        let err = Defaultable::from_option(None, "network")
            .resolve_with_default(InputMode::non_interactive(), || Ok(Some("testnet")))
            .unwrap_err();

        assert!(err.to_string().contains("network"));
    }

    #[tokio::test]
    async fn async_promptable_resolves_prompt() -> Result<()> {
        let resolved = Promptable::from_option(None, "account")
            .resolve_with_async(InputMode::interactive(), || async { Ok("alice") })
            .await?;

        assert_eq!(resolved.value, "alice");
        assert_eq!(resolved.source, ResolvedSource::Prompt);
        Ok(())
    }

    #[test]
    fn account_label_rejects_raw_address() {
        let err = "4UC8o4m8AgTxt5VBFMdLwMCwwJQVJwjesNzW7RPXkACynrULmd"
            .parse::<AccountLabel>()
            .unwrap_err();

        assert!(err.to_string().contains("not a raw account address"));
    }

    #[test]
    fn account_reference_preserves_address_or_label() -> Result<()> {
        let address =
            "4UC8o4m8AgTxt5VBFMdLwMCwwJQVJwjesNzW7RPXkACynrULmd".parse::<AccountReference>()?;
        let label = "alice-main".parse::<AccountReference>()?;

        assert!(matches!(address, AccountReference::Address(_)));
        assert!(matches!(label, AccountReference::Label(_)));
        Ok(())
    }

    #[test]
    fn label_newtypes_reject_invalid_characters() {
        assert!("testnet".parse::<NetworkName>().is_ok());
        assert!("ledger-main_1".parse::<KeySourceLabel>().is_ok());
        assert!("bad label".parse::<NetworkName>().is_err());
    }

    #[test]
    fn contract_address_input_accepts_cli_address_forms() -> Result<()> {
        let address = "42,0".parse::<ContractAddressInput>()?.address();

        assert_eq!(address.index, 42);
        assert_eq!(address.subindex, 0);
        Ok(())
    }

    #[test]
    fn token_amount_input_preserves_decimal_text_until_decimals_are_known() -> Result<()> {
        let amount = "1.2300".parse::<TokenAmountInput>()?;

        assert_eq!(amount.as_str(), "1.2300");
        assert!("".parse::<TokenAmountInput>().is_err());
        assert!("-1".parse::<TokenAmountInput>().is_err());
        Ok(())
    }

    #[test]
    fn release_schedule_entry_input_parses_rfc3339_amount_pairs() -> Result<()> {
        let entry = "2026-07-01T00:00:00Z=10.5".parse::<ReleaseScheduleEntryInput>()?;

        assert_eq!(entry.to_string(), "2026-07-01T00:00:00Z=10.5");
        assert_eq!(entry.amount()?.micro_ccd(), 10_500_000);
        assert!("tomorrow=10".parse::<ReleaseScheduleEntryInput>().is_err());
        assert!(
            "2026-07-01T00:00:00Z"
                .parse::<ReleaseScheduleEntryInput>()
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn shared_input_mode_args_parse_public_flags() {
        let args = InputModeCommand::parse_from(["test", "--non-interactive", "--no-defaults"]);
        let mode = InputMode::from(&args.input_mode);

        assert!(!mode.prompts_allowed());
        assert!(!mode.defaults_allowed());
    }

    #[test]
    fn shared_network_node_args_preserve_conflict() {
        let err = NetworkNodeCommand::try_parse_from([
            "test",
            "--network",
            "testnet",
            "--node",
            "https://node.example",
        ])
        .unwrap_err();

        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn shared_network_only_args_parse_network() {
        let args = NetworkOnlyCommand::parse_from(["test", "--network", "testnet"]);

        assert_eq!(
            args.network
                .network
                .as_ref()
                .map(|network| network.as_str()),
            Some("testnet")
        );
    }

    #[test]
    fn shared_submission_args_map_to_finalization_policy() {
        let args = SubmissionCommand::parse_from(["test", "--no-wait"]);

        assert_eq!(
            FinalizationPolicy::from(&args.submission),
            FinalizationPolicy::SubmitOnly
        );
    }

    #[test]
    fn policies_map_common_flags() {
        assert_eq!(
            FinalizationPolicy::from_no_wait(false),
            FinalizationPolicy::Wait
        );
        assert_eq!(
            FinalizationPolicy::from_no_wait(true),
            FinalizationPolicy::SubmitOnly
        );
        assert!(ValidationPolicy::from_no_validate(false).should_validate());
        assert!(!ValidationPolicy::from_no_validate(true).should_validate());
    }
}
