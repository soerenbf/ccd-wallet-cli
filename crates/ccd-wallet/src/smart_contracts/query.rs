//! Shared read-only smart contract query helpers.

use super::shared;
use anyhow::{Context, Result};
use concordium_rust_sdk::{
    common::types::{AccountAddress, Amount},
    smart_contracts::common::{ContractAddress, ModuleReference, OwnedParameter, OwnedReceiveName},
    types::{
        Energy,
        smart_contracts::{ContractContext, InstanceInfo, InvokeContractResult, WasmModule},
    },
    v2::{self, BlockIdentifier},
};

/// Prepared read-only contract invocation context.
pub(crate) struct PreparedContractInvoke {
    /// Invocation context sent to the node.
    pub(crate) context: ContractContext,
}

/// Prepare a read-only contract invocation.
pub(crate) fn prepare_contract_invoke(
    contract: ContractAddress,
    receive_name: OwnedReceiveName,
    amount: Amount,
    parameter: OwnedParameter,
    energy: Option<Energy>,
    invoker: Option<AccountAddress>,
) -> PreparedContractInvoke {
    let mut context = ContractContext::new(contract, receive_name);
    context.amount = amount;
    context.parameter = parameter;
    context.energy = energy;
    context.invoker = invoker.map(Into::into);
    PreparedContractInvoke { context }
}

/// Invoke a contract without submitting a transaction.
pub(crate) async fn invoke_contract(
    client: &mut v2::Client,
    block: BlockIdentifier,
    prepared: &PreparedContractInvoke,
) -> Result<InvokeContractResult> {
    Ok(client
        .invoke_instance(block, &prepared.context)
        .await
        .context("failed to invoke contract instance")?
        .response)
}

/// Fetch contract instance information.
pub(crate) async fn get_instance_info(
    client: &mut v2::Client,
    block: BlockIdentifier,
    contract: ContractAddress,
) -> Result<InstanceInfo> {
    Ok(client
        .get_instance_info(contract, block)
        .await
        .context("failed to fetch contract instance info")?
        .response)
}

/// Fetch a module source by module reference.
pub(crate) async fn get_module_source(
    client: &mut v2::Client,
    block: BlockIdentifier,
    module_ref: &ModuleReference,
) -> Result<WasmModule> {
    Ok(client
        .get_module_source(module_ref, block)
        .await
        .context("failed to fetch module source")?
        .response)
}

/// Resolve a module reference from a module reference or contract address.
pub(crate) async fn resolve_module_ref(
    client: &mut v2::Client,
    block: BlockIdentifier,
    module_ref: Option<ModuleReference>,
    contract: Option<ContractAddress>,
) -> Result<ModuleReference> {
    match (module_ref, contract) {
        (Some(module_ref), None) => Ok(module_ref),
        (None, Some(contract)) => Ok(get_instance_info(client, block, contract)
            .await?
            .source_module()),
        _ => anyhow::bail!("provide exactly one of module reference or contract address"),
    }
}

/// Render an invocation result as JSON.
pub(crate) fn invoke_result_json(result: &InvokeContractResult) -> serde_json::Value {
    match result {
        InvokeContractResult::Success {
            return_value,
            events,
            used_energy,
        } => serde_json::json!({
            "status": "success",
            "usedEnergy": used_energy.energy,
            "returnValue": return_value.as_ref().map(|value| hex::encode(&value.value)),
            "eventCount": events.len(),
        }),
        InvokeContractResult::Failure {
            return_value,
            reason,
            used_energy,
        } => serde_json::json!({
            "status": "failure",
            "usedEnergy": used_energy.energy,
            "returnValue": return_value.as_ref().map(|value| hex::encode(&value.value)),
            "reason": format!("{reason:?}"),
        }),
    }
}

/// Render instance information as JSON.
pub(crate) fn instance_info_json(
    contract: ContractAddress,
    info: &InstanceInfo,
) -> serde_json::Value {
    serde_json::json!({
        "address": { "index": contract.index, "subindex": contract.subindex },
        "name": info.name().to_string(),
        "owner": owner_string(info),
        "amountMicroCcd": info.amount().micro_ccd(),
        "sourceModule": info.source_module().to_string(),
        "entrypoints": info.entrypoints().iter().map(ToString::to_string).collect::<Vec<_>>(),
        "version": match info {
            InstanceInfo::V0 { .. } => "v0",
            InstanceInfo::V1 { .. } => "v1",
        },
    })
}

/// Render instance information for humans.
pub(crate) fn render_instance_info(contract: ContractAddress, info: &InstanceInfo) -> String {
    let mut output = format!(
        "Contract <{}, {}>\nName: {}\nOwner: {}\nBalance: {} microCCD\nSource module: {}\nVersion: {}\nEntrypoints:",
        contract.index,
        contract.subindex,
        info.name(),
        owner_string(info),
        info.amount().micro_ccd(),
        info.source_module(),
        match info {
            InstanceInfo::V0 { .. } => "v0",
            InstanceInfo::V1 { .. } => "v1",
        }
    );
    for entrypoint in info.entrypoints() {
        output.push_str(&format!("\n  {entrypoint}"));
    }
    output
}

/// Render an invocation result for humans.
pub(crate) fn render_invoke_result(result: &InvokeContractResult) -> String {
    match result {
        InvokeContractResult::Success {
            return_value,
            events,
            used_energy,
        } => format!(
            "Invocation succeeded\nUsed energy: {}\nReturn value: {}\nEvents: {}",
            used_energy.energy,
            return_value
                .as_ref()
                .map(|value| format!("0x{}", hex::encode(&value.value)))
                .unwrap_or_else(|| "<none>".to_owned()),
            events.len()
        ),
        InvokeContractResult::Failure {
            return_value,
            reason,
            used_energy,
        } => format!(
            "Invocation failed\nUsed energy: {}\nReason: {:?}\nReturn value: {}",
            used_energy.energy,
            reason,
            return_value
                .as_ref()
                .map(|value| format!("0x{}", hex::encode(&value.value)))
                .unwrap_or_else(|| "<none>".to_owned())
        ),
    }
}

fn owner_string(info: &InstanceInfo) -> String {
    match info {
        InstanceInfo::V0 { owner, .. } | InstanceInfo::V1 { owner, .. } => owner.to_string(),
    }
}

/// Serialize a module source to bytes suitable for writing to disk.
pub(crate) fn module_bytes(module: &WasmModule) -> Vec<u8> {
    shared::serialize_module(module)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invoke_defaults_to_no_invoker() {
        let contract = ContractAddress {
            index: 1,
            subindex: 0,
        };
        let receive = OwnedReceiveName::new_unchecked("counter.view".to_owned());
        let prepared = prepare_contract_invoke(
            contract,
            receive,
            Amount::from_micro_ccd(0),
            OwnedParameter::empty(),
            None,
            None,
        );
        assert!(prepared.context.invoker.is_none());
        assert!(prepared.context.energy.is_none());
        assert!(prepared.context.parameter.as_ref().is_empty());
    }
}
