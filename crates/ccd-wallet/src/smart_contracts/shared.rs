//! Shared smart contract parsing, schema, and query helpers.

use anyhow::{Context, Result, bail};
use concordium_rust_sdk::{
    common::{
        Serial,
        types::{AccountAddress, Amount},
    },
    smart_contracts::common::{
        ContractAddress, ModuleReference, OwnedParameter, OwnedReceiveName,
        schema::VersionedModuleSchema,
    },
    types::smart_contracts::{WasmModule, WasmVersion},
    v2::{self, BlockIdentifier},
};
use concordium_smart_contract_engine::utils;
use std::{path::Path, str::FromStr};

/// Parse a CCD decimal amount into exact microCCD chain units.
///
/// # Arguments
///
/// * `value` - Optional user-facing decimal CCD amount. `None` defaults to zero CCD.
///
/// # Errors
///
/// Returns an error if the amount is empty, negative, contains invalid characters, has more than
/// six fractional digits, or overflows `u64` microCCD units.
///
/// # Examples
///
/// ```ignore
/// let amount = parse_decimal_ccd_amount(Some("1.25"))?;
/// assert_eq!(amount.micro_ccd(), 1_250_000);
/// # anyhow::Ok(())
/// ```
pub(crate) fn parse_decimal_ccd_amount(value: Option<&str>) -> Result<Amount> {
    let Some(raw) = value else {
        return Ok(Amount::from_micro_ccd(0));
    };
    let raw = raw.trim();
    if raw.is_empty() {
        bail!("amount must not be empty");
    }
    if raw.starts_with('-') {
        bail!("amount must not be negative");
    }
    let mut parts = raw.split('.');
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next();
    if parts.next().is_some() {
        bail!("amount must be a decimal CCD value such as 0, 1, or 1.25");
    }
    if whole.is_empty() && fraction.is_none() {
        bail!("amount must be a decimal CCD value such as 0, 1, or 1.25");
    }
    if !whole.chars().all(|ch| ch.is_ascii_digit()) || whole.is_empty() {
        bail!("amount whole CCD part must contain digits only");
    }
    let whole_micro = whole
        .parse::<u64>()?
        .checked_mul(1_000_000)
        .context("amount is too large")?;
    let fraction_micro = match fraction {
        Some(value) if value.len() > 6 => bail!("amount cannot have more than six decimal places"),
        Some(value) if !value.chars().all(|ch| ch.is_ascii_digit()) => {
            bail!("amount fractional CCD part must contain digits only")
        }
        Some(value) => {
            let padded = format!("{value:0<6}");
            padded.parse::<u64>()?
        }
        None => 0,
    };
    Ok(Amount::from_micro_ccd(
        whole_micro
            .checked_add(fraction_micro)
            .context("amount is too large")?,
    ))
}

/// Parse a smart contract instance address.
///
/// # Arguments
///
/// * `value` - Address in `index`, `index,subindex`, or `<index,subindex>` form.
///
/// # Errors
///
/// Returns an error when either component is not an unsigned integer.
pub(crate) fn parse_contract_address(value: &str) -> Result<ContractAddress> {
    let trimmed = value.trim().trim_start_matches('<').trim_end_matches('>');
    let mut parts = trimmed.split(',').map(str::trim);
    let index = parts
        .next()
        .context("contract address index is required")?
        .parse::<u64>()
        .with_context(|| format!("invalid contract address index in '{value}'"))?;
    let subindex = match parts.next() {
        Some(raw) if !raw.is_empty() => raw
            .parse::<u64>()
            .with_context(|| format!("invalid contract address subindex in '{value}'"))?,
        Some(_) | None => 0,
    };
    if parts.next().is_some() {
        bail!("contract address must be formatted as index,subindex");
    }
    Ok(ContractAddress { index, subindex })
}

/// Parse a module reference from its hex string form.
pub(crate) fn parse_module_reference(value: &str) -> Result<ModuleReference> {
    ModuleReference::from_str(value).with_context(|| format!("invalid module reference '{value}'"))
}

/// Parse a receive name and split it into contract and function components.
pub(crate) fn parse_receive_name(value: &str) -> Result<(OwnedReceiveName, String, String)> {
    let receive_name = OwnedReceiveName::new(value.to_owned())
        .with_context(|| format!("invalid receive name '{value}'"))?;
    let (contract_name, function_name) = value
        .split_once('.')
        .with_context(|| "receive name must be fully qualified as '<contract>.<function>'")?;
    Ok((
        receive_name,
        contract_name.to_owned(),
        function_name.to_owned(),
    ))
}

/// Parse an optional block selector, defaulting to last finalized.
pub(crate) fn parse_block_identifier(value: Option<&str>) -> Result<BlockIdentifier> {
    match value {
        None => Ok(BlockIdentifier::LastFinal),
        Some("last-final") => Ok(BlockIdentifier::LastFinal),
        Some(value) => value
            .parse()
            .with_context(|| format!("invalid block selector '{value}'")),
    }
}

/// Resolve raw or JSON parameter input into serialized contract parameter bytes.
pub(crate) async fn resolve_parameter(
    client: &mut v2::Client,
    block: BlockIdentifier,
    parameter_hex: Option<&str>,
    parameter_json: Option<&str>,
    parameter_json_file: Option<&Path>,
    schema_source: SchemaSource,
    schema_type: SchemaParameter,
) -> Result<OwnedParameter> {
    match (parameter_hex, parameter_json, parameter_json_file) {
        (Some(hex), None, None) => parse_parameter_hex(hex),
        (None, Some(json), None) => {
            let value = serde_json::from_str(json).context("parameter JSON is not valid JSON")?;
            encode_parameter_json(client, block, schema_source, schema_type, &value).await
        }
        (None, None, Some(path)) => {
            let contents = std::fs::read_to_string(path).with_context(|| {
                format!("failed to read parameter JSON file {}", path.display())
            })?;
            let value = serde_json::from_str(&contents).with_context(|| {
                format!("parameter JSON file {} is not valid JSON", path.display())
            })?;
            encode_parameter_json(client, block, schema_source, schema_type, &value).await
        }
        (None, None, None) => Ok(OwnedParameter::empty()),
        _ => bail!(
            "provide at most one of --parameter-hex, --parameter-json, or --parameter-json-file"
        ),
    }
}

/// Parse serialized parameter bytes from hex.
pub(crate) fn parse_parameter_hex(value: &str) -> Result<OwnedParameter> {
    let bytes = if value.is_empty() {
        Vec::new()
    } else {
        hex::decode(value).context("parameter hex must be lower- or upper-case hex")?
    };
    Ok(OwnedParameter::new_unchecked(bytes))
}

/// Source used to locate an embedded module schema.
#[derive(Clone, Copy)]
pub(crate) enum SchemaSource {
    /// Resolve schema from a module reference.
    Module(ModuleReference),
    /// Resolve schema from an instance's source module.
    Contract(ContractAddress),
}

/// Parameter schema kind to resolve.
pub(crate) enum SchemaParameter {
    /// Resolve an init parameter schema by contract name.
    Init { contract_name: String },
    /// Resolve a receive parameter schema by contract and function name.
    Receive {
        contract_name: String,
        function_name: String,
    },
}

/// Fetch and decode an embedded module schema from a module reference or instance.
pub(crate) async fn fetch_embedded_schema(
    client: &mut v2::Client,
    block: BlockIdentifier,
    source: SchemaSource,
) -> Result<(ModuleReference, VersionedModuleSchema)> {
    let module_ref = match source {
        SchemaSource::Module(module_ref) => module_ref,
        SchemaSource::Contract(address) => client
            .get_instance_info(address, block)
            .await
            .context("failed to fetch contract instance info")?
            .response
            .source_module(),
    };
    let module = client
        .get_module_source(&module_ref, block)
        .await
        .context("failed to fetch module source")?
        .response;
    let schema = embedded_schema(&module).with_context(|| {
        format!("module {module_ref} does not contain a compatible embedded schema")
    })?;
    Ok((module_ref, schema))
}

/// Render a JSON parameter template from an embedded schema.
pub(crate) async fn parameter_template(
    client: &mut v2::Client,
    block: BlockIdentifier,
    source: SchemaSource,
    schema_type: SchemaParameter,
) -> Result<serde_json::Value> {
    let (_module_ref, schema) = fetch_embedded_schema(client, block, source).await?;
    let ty = resolve_schema_type(&schema, schema_type)?;
    Ok(ty.to_json_template())
}

async fn encode_parameter_json(
    client: &mut v2::Client,
    block: BlockIdentifier,
    source: SchemaSource,
    schema_type: SchemaParameter,
    value: &serde_json::Value,
) -> Result<OwnedParameter> {
    let (_module_ref, schema) = fetch_embedded_schema(client, block, source).await?;
    let ty = resolve_schema_type(&schema, schema_type)?;
    let bytes = ty
        .serial_value(value)
        .map_err(|err| anyhow::anyhow!(err.to_string()))
        .context("failed to serialize parameter JSON with embedded schema")?;
    Ok(OwnedParameter::new_unchecked(bytes))
}

fn resolve_schema_type(
    schema: &VersionedModuleSchema,
    schema_type: SchemaParameter,
) -> Result<concordium_rust_sdk::smart_contracts::common::schema::Type> {
    match schema_type {
        SchemaParameter::Init { contract_name } => schema
            .get_init_param_schema(contract_name.trim_start_matches("init_"))
            .with_context(|| format!("no init parameter schema found for {contract_name}")),
        SchemaParameter::Receive {
            contract_name,
            function_name,
        } => schema
            .get_receive_param_schema(&contract_name, &function_name)
            .with_context(|| {
                format!("no receive parameter schema found for {contract_name}.{function_name}")
            }),
    }
}

fn embedded_schema(module: &WasmModule) -> Result<VersionedModuleSchema> {
    match module.version {
        WasmVersion::V0 => utils::get_embedded_schema_v0(module.source.as_ref()),
        WasmVersion::V1 => utils::get_embedded_schema_v1(module.source.as_ref()),
    }
    .map_err(|err| anyhow::anyhow!(err.to_string()))
}

/// Serialize a module source to its on-chain binary representation.
pub(crate) fn serialize_module(module: &WasmModule) -> Vec<u8> {
    let mut bytes = Vec::new();
    module.serial(&mut bytes);
    bytes
}

/// Parse an account address.
pub(crate) fn parse_account_address(value: &str) -> Result<AccountAddress> {
    AccountAddress::from_str(value).with_context(|| format!("invalid account address '{value}'"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_amount_accepts_fractional_ccd() {
        assert_eq!(
            parse_decimal_ccd_amount(Some("1.25")).unwrap().micro_ccd(),
            1_250_000
        );
        assert_eq!(
            parse_decimal_ccd_amount(Some("0.000001"))
                .unwrap()
                .micro_ccd(),
            1
        );
    }

    #[test]
    fn decimal_amount_rejects_too_many_digits() {
        assert!(parse_decimal_ccd_amount(Some("0.0000001")).is_err());
    }

    #[test]
    fn contract_address_defaults_subindex() {
        let address = parse_contract_address("42").unwrap();
        assert_eq!(address.index, 42);
        assert_eq!(address.subindex, 0);
    }

    #[test]
    fn contract_address_accepts_angle_brackets() {
        let address = parse_contract_address("<42,7>").unwrap();
        assert_eq!(address.index, 42);
        assert_eq!(address.subindex, 7);
    }
}
