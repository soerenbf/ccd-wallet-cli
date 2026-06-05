//! Governance update signing commands for the Governance Ledger app.

use crate::{
    apdu::{
        ApduCommand, Instruction, MAX_APDU_PAYLOAD_SIZE, NONE, P1_BYTES, P1_INITIAL, P1_LENGTH,
    },
    error::{GovernanceLedgerError, Result},
    serialization::{chunk_payload, non_empty_chunks},
    transport::GovernanceLedgerTransport,
    types::{
        AddAnonymityRevokerRequest, AddIdentityProviderRequest, AuthorizationsUpdateRequest,
        CreatePltRequest, DescriptionFields, FixedUpdateRequest, HigherLevelKeyUpdateRequest,
        ProtocolUpdateRequest, RawSignature,
    },
};

/// Sign a fixed-shape governance update and return the raw signature.
///
/// # Arguments
///
/// * `transport` - APDU transport used for exchange.
/// * `instruction` - Governance Ledger instruction for the update family.
/// * `request` - Fixed-shape update request.
///
/// # Errors
///
/// Returns an error if the payload is too large, APDU exchange fails, or the response is malformed.
pub fn sign_fixed_update<T: GovernanceLedgerTransport>(
    transport: &mut T,
    instruction: Instruction,
    request: &FixedUpdateRequest,
) -> Result<RawSignature> {
    let command = build_fixed_update_command(instruction, request)?;
    let reply = transport.exchange(&command)?;
    RawSignature::from_response(reply.data)
}

/// Build a fixed-shape governance update command.
///
/// # Arguments
///
/// * `instruction` - Governance Ledger instruction.
/// * `request` - Fixed-shape update request.
///
/// # Errors
///
/// Returns an error if the APDU payload is larger than 255 bytes.
pub fn build_fixed_update_command(
    instruction: Instruction,
    request: &FixedUpdateRequest,
) -> Result<ApduCommand> {
    let mut data = request.prefix.to_ledger_bytes();
    data.extend_from_slice(&request.payload);
    ensure_apdu_payload_len(&data)?;
    Ok(ApduCommand::new(
        instruction.as_u8(),
        P1_INITIAL,
        NONE,
        data,
    ))
}

/// Sign a protocol update and return the raw signature.
pub fn sign_protocol_update<T: GovernanceLedgerTransport>(
    transport: &mut T,
    request: &ProtocolUpdateRequest,
) -> Result<RawSignature> {
    exchange_commands_for_signature(transport, build_protocol_update_commands(request)?)
}

/// Build APDU commands for a protocol update request.
///
/// # Arguments
///
/// * `request` - Protocol update request.
///
/// # Errors
///
/// Returns an error if a staged field cannot be represented in the protocol.
pub fn build_protocol_update_commands(request: &ProtocolUpdateRequest) -> Result<Vec<ApduCommand>> {
    let ins = Instruction::UpdateProtocol.as_u8();
    let mut commands = Vec::new();
    let mut initial = request.prefix.to_ledger_bytes();
    initial.extend_from_slice(&request.payload_length.to_be_bytes());
    ensure_apdu_payload_len(&initial)?;
    commands.push(ApduCommand::new(ins, P1_INITIAL, NONE, initial));
    push_u64_length_and_bytes(&mut commands, ins, &request.message)?;
    push_u64_length_and_bytes(&mut commands, ins, &request.specification_url)?;
    commands.push(ApduCommand::new(
        ins,
        0x03,
        NONE,
        request.specification_hash.to_vec(),
    ));
    for chunk in non_empty_chunks("auxiliary data", &request.auxiliary_data)? {
        commands.push(ApduCommand::new(ins, 0x04, NONE, chunk));
    }
    Ok(commands)
}

/// Sign an add-anonymity-revoker update and return the raw signature.
pub fn sign_add_anonymity_revoker<T: GovernanceLedgerTransport>(
    transport: &mut T,
    request: &AddAnonymityRevokerRequest,
) -> Result<RawSignature> {
    exchange_commands_for_signature(transport, build_add_anonymity_revoker_commands(request)?)
}

/// Build APDU commands for an add-anonymity-revoker update.
pub fn build_add_anonymity_revoker_commands(
    request: &AddAnonymityRevokerRequest,
) -> Result<Vec<ApduCommand>> {
    let ins = Instruction::AddAnonymityRevoker.as_u8();
    let mut commands = Vec::new();
    let mut initial = request.prefix.to_ledger_bytes();
    initial.extend_from_slice(&request.payload_length.to_be_bytes());
    initial.extend_from_slice(&request.ar_info_length.to_be_bytes());
    initial.extend_from_slice(&request.ar_identity.to_be_bytes());
    ensure_apdu_payload_len(&initial)?;
    commands.push(ApduCommand::new(ins, P1_INITIAL, NONE, initial));
    push_description_fields(&mut commands, ins, &request.description)?;
    ensure_exact_len("anonymity revoker public key", &request.public_key, 96)?;
    commands.push(ApduCommand::new(
        ins,
        0x03,
        NONE,
        request.public_key.clone(),
    ));
    Ok(commands)
}

/// Sign an add-identity-provider update and return the raw signature.
pub fn sign_add_identity_provider<T: GovernanceLedgerTransport>(
    transport: &mut T,
    request: &AddIdentityProviderRequest,
) -> Result<RawSignature> {
    exchange_commands_for_signature(transport, build_add_identity_provider_commands(request)?)
}

/// Build APDU commands for an add-identity-provider update.
pub fn build_add_identity_provider_commands(
    request: &AddIdentityProviderRequest,
) -> Result<Vec<ApduCommand>> {
    let ins = Instruction::AddIdentityProvider.as_u8();
    let mut commands = Vec::new();
    let mut initial = request.prefix.to_ledger_bytes();
    initial.extend_from_slice(&request.payload_length.to_be_bytes());
    initial.extend_from_slice(&request.ip_info_length.to_be_bytes());
    initial.extend_from_slice(&request.ip_identity.to_be_bytes());
    ensure_apdu_payload_len(&initial)?;
    commands.push(ApduCommand::new(ins, P1_INITIAL, NONE, initial));
    push_description_fields(&mut commands, ins, &request.description)?;
    ensure_apdu_payload_len(&request.verify_key)?;
    commands.push(ApduCommand::new(
        ins,
        0x03,
        NONE,
        request.verify_key.clone(),
    ));
    commands.push(ApduCommand::new(
        ins,
        0x04,
        NONE,
        request.cdi_verify_key.to_vec(),
    ));
    Ok(commands)
}

/// Sign a create-PLT update and return the raw signature.
pub fn sign_create_plt<T: GovernanceLedgerTransport>(
    transport: &mut T,
    request: &CreatePltRequest,
) -> Result<RawSignature> {
    exchange_commands_for_signature(transport, build_create_plt_commands(request)?)
}

/// Build APDU commands for a create-PLT update.
pub fn build_create_plt_commands(request: &CreatePltRequest) -> Result<Vec<ApduCommand>> {
    let ins = Instruction::CreatePlt.as_u8();
    if request.token_id.is_empty() || request.token_id.len() > 128 {
        return Err(GovernanceLedgerError::invalid_request(
            "token id length must be between 1 and 128 bytes",
        ));
    }
    if request.initialization_parameters.is_empty() {
        return Err(GovernanceLedgerError::invalid_request(
            "initialization parameters cannot be empty",
        ));
    }
    let mut commands = Vec::new();
    let initial = request.prefix.to_ledger_bytes();
    ensure_apdu_payload_len(&initial)?;
    commands.push(ApduCommand::new(ins, P1_INITIAL, NONE, initial));

    let params_len = u32::try_from(request.initialization_parameters.len()).map_err(|_| {
        GovernanceLedgerError::invalid_request("initialization parameters length exceeds u32::MAX")
    })?;
    let mut payload = Vec::with_capacity(1 + request.token_id.len() + 32 + 1 + 4);
    payload.push(request.token_id.len() as u8);
    payload.extend_from_slice(&request.token_id);
    payload.extend_from_slice(&request.token_module);
    payload.push(request.decimals);
    payload.extend_from_slice(&params_len.to_be_bytes());
    ensure_apdu_payload_len(&payload)?;
    commands.push(ApduCommand::new(ins, 0x01, NONE, payload));
    for chunk in chunk_payload(&request.initialization_parameters) {
        commands.push(ApduCommand::new(ins, 0x02, NONE, chunk));
    }
    Ok(commands)
}

/// Sign a higher-level governance key update and return the raw signature.
pub fn sign_higher_level_keys<T: GovernanceLedgerTransport>(
    transport: &mut T,
    instruction: Instruction,
    request: &HigherLevelKeyUpdateRequest,
) -> Result<RawSignature> {
    exchange_commands_for_signature(
        transport,
        build_higher_level_key_update_commands(instruction, request)?,
    )
}

/// Build APDU commands for higher-level governance key updates.
pub fn build_higher_level_key_update_commands(
    instruction: Instruction,
    request: &HigherLevelKeyUpdateRequest,
) -> Result<Vec<ApduCommand>> {
    let ins = instruction.as_u8();
    let key_count = u16::try_from(request.keys.len())
        .map_err(|_| GovernanceLedgerError::invalid_request("too many governance keys"))?;
    let mut commands = Vec::new();
    let mut initial = request.prefix.to_ledger_bytes();
    initial.push(
        request
            .key_update_type
            .to_device_byte(request.prefix.update_type)?,
    );
    initial.extend_from_slice(&key_count.to_be_bytes());
    ensure_apdu_payload_len(&initial)?;
    commands.push(ApduCommand::new(ins, P1_INITIAL, NONE, initial));
    for key in &request.keys {
        let mut data = Vec::with_capacity(33);
        data.push(key.scheme_id);
        data.extend_from_slice(&key.key);
        commands.push(ApduCommand::new(ins, 0x01, NONE, data));
    }
    commands.push(ApduCommand::new(
        ins,
        0x02,
        NONE,
        request.threshold.to_be_bytes().to_vec(),
    ));
    Ok(commands)
}

/// Sign a level-2 authorizations update and return the raw signature.
pub fn sign_authorizations<T: GovernanceLedgerTransport>(
    transport: &mut T,
    instruction: Instruction,
    request: &AuthorizationsUpdateRequest,
) -> Result<RawSignature> {
    exchange_commands_for_signature(
        transport,
        build_authorizations_update_commands(instruction, request)?,
    )
}

/// Build APDU commands for a level-2 authorizations update.
pub fn build_authorizations_update_commands(
    instruction: Instruction,
    request: &AuthorizationsUpdateRequest,
) -> Result<Vec<ApduCommand>> {
    let ins = instruction.as_u8();
    let p2 = request.version.p2();
    let key_count = u16::try_from(request.keys.len())
        .map_err(|_| GovernanceLedgerError::invalid_request("too many governance keys"))?;
    let mut commands = Vec::new();
    let mut initial = request.prefix.to_ledger_bytes();
    initial.push(request.key_update_type.to_device_byte(request.version));
    initial.extend_from_slice(&key_count.to_be_bytes());
    ensure_apdu_payload_len(&initial)?;
    commands.push(ApduCommand::new(ins, P1_INITIAL, p2, initial));
    for key in &request.keys {
        let mut data = Vec::with_capacity(33);
        data.push(key.scheme_id);
        data.extend_from_slice(&key.key);
        commands.push(ApduCommand::new(ins, 0x01, p2, data));
    }
    for structure in &request.access_structures {
        let structure_len = u16::try_from(structure.key_indices.len()).map_err(|_| {
            GovernanceLedgerError::invalid_request("too many access-structure key indices")
        })?;
        commands.push(ApduCommand::new(
            ins,
            0x02,
            p2,
            structure_len.to_be_bytes().to_vec(),
        ));
        let mut indices = Vec::with_capacity(structure.key_indices.len() * 2);
        for index in &structure.key_indices {
            indices.extend_from_slice(&index.to_be_bytes());
        }
        ensure_apdu_payload_len(&indices)?;
        commands.push(ApduCommand::new(ins, 0x03, p2, indices));
        commands.push(ApduCommand::new(
            ins,
            0x04,
            p2,
            structure.threshold.to_be_bytes().to_vec(),
        ));
    }
    Ok(commands)
}

fn exchange_commands_for_signature<T: GovernanceLedgerTransport>(
    transport: &mut T,
    commands: Vec<ApduCommand>,
) -> Result<RawSignature> {
    let mut last = None;
    for command in commands {
        last = Some(transport.exchange(&command)?.data);
    }
    RawSignature::from_response(last.unwrap_or_default())
}

fn push_u64_length_and_bytes(commands: &mut Vec<ApduCommand>, ins: u8, bytes: &[u8]) -> Result<()> {
    let len = u64::try_from(bytes.len())
        .map_err(|_| GovernanceLedgerError::invalid_request("field length exceeds u64::MAX"))?;
    commands.push(ApduCommand::new(
        ins,
        P1_LENGTH,
        NONE,
        len.to_be_bytes().to_vec(),
    ));
    for chunk in non_empty_chunks("staged bytes", bytes)? {
        commands.push(ApduCommand::new(ins, P1_BYTES, NONE, chunk));
    }
    Ok(())
}

fn push_description_fields(
    commands: &mut Vec<ApduCommand>,
    ins: u8,
    fields: &DescriptionFields,
) -> Result<()> {
    push_u32_length_and_bytes(commands, ins, &fields.name)?;
    push_u32_length_and_bytes(commands, ins, &fields.url)?;
    push_u32_length_and_bytes(commands, ins, &fields.description)
}

fn push_u32_length_and_bytes(commands: &mut Vec<ApduCommand>, ins: u8, bytes: &[u8]) -> Result<()> {
    let len = u32::try_from(bytes.len())
        .map_err(|_| GovernanceLedgerError::invalid_request("field length exceeds u32::MAX"))?;
    commands.push(ApduCommand::new(
        ins,
        P1_LENGTH,
        NONE,
        len.to_be_bytes().to_vec(),
    ));
    for chunk in non_empty_chunks("description field", bytes)? {
        commands.push(ApduCommand::new(ins, P1_BYTES, NONE, chunk));
    }
    Ok(())
}

fn ensure_apdu_payload_len(bytes: &[u8]) -> Result<()> {
    if bytes.len() > MAX_APDU_PAYLOAD_SIZE {
        return Err(GovernanceLedgerError::invalid_request(format!(
            "APDU payload is {} bytes; maximum is {MAX_APDU_PAYLOAD_SIZE}",
            bytes.len()
        )));
    }
    Ok(())
}

fn ensure_exact_len(field_name: &str, bytes: &[u8], expected: usize) -> Result<()> {
    if bytes.len() != expected {
        return Err(GovernanceLedgerError::invalid_request(format!(
            "{field_name} must be {expected} bytes, got {} bytes",
            bytes.len()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DerivationPath, GovernanceUpdatePrefix, MockTransport, UpdateHeaderBytes};

    fn prefix(update_type: u8) -> GovernanceUpdatePrefix {
        GovernanceUpdatePrefix {
            path: DerivationPath::new([1]).unwrap(),
            header: UpdateHeaderBytes::new([0; 28]),
            update_type,
        }
    }

    fn signature_reply() -> Vec<u8> {
        let mut reply = vec![7; 64];
        reply.extend_from_slice(&[0x90, 0x00]);
        reply
    }

    #[test]
    fn fixed_update_command_contains_prefix_and_payload() {
        let request = FixedUpdateRequest {
            prefix: prefix(3),
            payload: vec![1, 2],
        };
        let command =
            build_fixed_update_command(Instruction::UpdateExchangeRate, &request).unwrap();
        assert_eq!(command.ins, 0x06);
        assert_eq!(command.p1, 0x00);
        assert_eq!(command.data.last().copied(), Some(2));
    }

    #[test]
    fn create_plt_chunks_initialization_parameters() {
        let request = CreatePltRequest {
            prefix: prefix(24),
            token_id: b"TRY".to_vec(),
            token_module: [1; 32],
            decimals: 6,
            initialization_parameters: vec![9; 256],
        };
        let commands = build_create_plt_commands(&request).unwrap();
        assert_eq!(commands[0].ins, Instruction::CreatePlt.as_u8());
        assert_eq!(commands.len(), 4);
        assert_eq!(commands[2].data.len(), 255);
        assert_eq!(commands[3].data.len(), 1);
    }

    #[test]
    fn fixed_update_round_trip_with_mock_transport() {
        let mut transport = MockTransport::new([signature_reply()]);
        let request = FixedUpdateRequest {
            prefix: prefix(3),
            payload: vec![0; 16],
        };
        let signature =
            sign_fixed_update(&mut transport, Instruction::UpdateExchangeRate, &request).unwrap();
        assert_eq!(signature.0, [7; 64]);
        assert_eq!(transport.commands()[0].ins, 0x06);
    }

    #[test]
    fn authorizations_uses_version_p2() {
        let request = AuthorizationsUpdateRequest {
            prefix: prefix(10),
            key_update_type: crate::AuthorizationsKeyUpdateType::RootKeys,
            version: crate::AuthorizationsVersion::V2,
            keys: vec![],
            access_structures: vec![crate::AccessStructureUpdate {
                key_indices: vec![1, 2],
                threshold: 1,
            }],
        };
        let commands =
            build_authorizations_update_commands(Instruction::UpdateLevel2KeysRoot, &request)
                .unwrap();
        assert!(commands.iter().all(|command| command.p2 == 2));
    }
}
