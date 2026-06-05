//! Signing command helpers for Concordium Ledger app transaction flows.

const MAX_SCHEDULE_PAIR_CHUNK_BYTES: usize = 15 * 16;

use crate::{
    apdu::{
        ApduCommand,
        constants::{
            Instruction, NONE, P1_AGGREGATION_KEY, P1_AMOUNT, P1_AR_IDENTITY, P1_ATTRIBUTE_TAG,
            P1_ATTRIBUTE_VALUE, P1_COMMISSION_FEE, P1_CREDENTIAL_DATES, P1_DATA, P1_FIRST_BATCH,
            P1_FIRST_CHUNK, P1_INITIAL_PACKET, P1_INITIAL_WITH_MEMO, P1_INITIAL_WITH_MEMO_SCHEDULE,
            P1_LENGTH_OF_PROOFS, P1_MEMO, P1_MEMO_SCHEDULE, P1_NAME, P1_NEW_OR_EXISTING, P1_PARAM,
            P1_PROOF, P1_PROOFS, P1_REMAINING_AMOUNT, P1_SCHEDULED_TRANSFER_PAIRS,
            P1_SIGNATURE_THRESHOLD, P1_SOURCE, P1_SUSPENDED, P1_URL, P1_URL_LENGTH,
            P1_VERIFICATION_KEY, P1_VERIFICATION_KEY_LENGTH, P2_CREDENTIAL_CREDENTIAL,
            P2_CREDENTIAL_CREDENTIAL_INDEX, P2_CREDENTIAL_ID, P2_CREDENTIAL_ID_COUNT,
            P2_CREDENTIAL_INITIAL, P2_LAST, P2_MORE, P2_THRESHOLD,
        },
    },
    error::{LedgerError, Result},
    serialization::{chunk_payload, chunk_payload_with_path, length_prefix_u16},
    transport::LedgerTransport,
    types::{
        ChunkedSigningRequest, ConfigureBakerSigningRequest, ContractSigningRequest,
        CredentialDeploymentContext, CredentialDeploymentSigningRequest, CredentialSigningPayload,
        DeployModuleSigningRequest, PublicInfoForIpSigningRequest, RawSignature,
        RegisterDataSigningRequest, ScheduledTransferSigningRequest,
        ScheduledTransferWithMemoSigningRequest, TransferToPublicSigningRequest,
        TransferWithMemoSigningRequest, UpdateCredentialsSigningRequest,
    },
};

/// Build generic chunked signing APDU commands for a command family.
///
/// # Arguments
///
/// * `instruction` - Ledger signing instruction to use.
/// * `request` - Request containing path and serialized transaction bytes.
///
/// # Errors
///
/// Returns an error if the request payload cannot be chunked.
pub fn build_chunked_signing_commands(
    instruction: Instruction,
    request: &ChunkedSigningRequest,
) -> Result<Vec<ApduCommand>> {
    let payloads = chunk_payload_with_path(&request.path, &request.transaction)?;
    let last_index = payloads.len().saturating_sub(1);
    payloads
        .into_iter()
        .enumerate()
        .map(|(index, payload)| {
            let p1_offset = u8::try_from(index).map_err(|_| {
                LedgerError::invalid_request(format!(
                    "signing command requires chunk index {index}, exceeding APDU P1 range"
                ))
            })?;
            let p2 = if index == last_index {
                P2_LAST
            } else {
                P2_MORE
            };
            Ok(ApduCommand::new(
                instruction.as_u8(),
                P1_FIRST_CHUNK.saturating_add(p1_offset),
                p2,
                payload,
            ))
        })
        .collect()
}

/// Execute a generic chunked signing command and return the raw signature.
///
/// # Arguments
///
/// * `transport` - APDU transport connected to a Ledger device.
/// * `instruction` - Ledger signing instruction to use.
/// * `request` - Request containing path and serialized transaction bytes.
///
/// # Errors
///
/// Returns an error if APDU exchange fails or the final response is not a signature.
pub fn sign_chunked<T: LedgerTransport>(
    transport: &mut T,
    instruction: Instruction,
    request: &ChunkedSigningRequest,
) -> Result<RawSignature> {
    let commands = build_chunked_signing_commands(instruction, request)?;
    exchange_sequence_for_signature(transport, &commands)
}

/// Build transfer-with-memo signing commands.
pub fn build_transfer_with_memo_commands(
    request: &TransferWithMemoSigningRequest,
) -> Vec<ApduCommand> {
    vec![
        ApduCommand::new(
            Instruction::SignTransferMemo.as_u8(),
            P1_INITIAL_WITH_MEMO,
            NONE,
            request.header_address_memo_length.clone(),
        ),
        ApduCommand::new(
            Instruction::SignTransferMemo.as_u8(),
            P1_MEMO,
            NONE,
            request.memo.clone(),
        ),
        ApduCommand::new(
            Instruction::SignTransferMemo.as_u8(),
            P1_AMOUNT,
            NONE,
            request.amount.clone(),
        ),
    ]
}

/// Execute transfer-with-memo signing and return the raw signature.
pub fn sign_transfer_with_memo<T: LedgerTransport>(
    transport: &mut T,
    request: &TransferWithMemoSigningRequest,
) -> Result<RawSignature> {
    exchange_sequence_for_signature(transport, &build_transfer_with_memo_commands(request))
}

/// Build transfer-with-schedule signing commands.
pub fn build_scheduled_transfer_commands(
    request: &ScheduledTransferSigningRequest,
) -> Vec<ApduCommand> {
    let mut commands = vec![ApduCommand::new(
        Instruction::SignTransferSchedule.as_u8(),
        P1_INITIAL_PACKET,
        NONE,
        request.header_address_schedule_length.clone(),
    )];
    commands.extend(
        chunk_schedule_pairs(&request.schedule)
            .into_iter()
            .map(|payload| {
                ApduCommand::new(
                    Instruction::SignTransferSchedule.as_u8(),
                    P1_SCHEDULED_TRANSFER_PAIRS,
                    NONE,
                    payload,
                )
            }),
    );
    commands
}

/// Execute transfer-with-schedule signing and return the raw signature.
pub fn sign_scheduled_transfer<T: LedgerTransport>(
    transport: &mut T,
    request: &ScheduledTransferSigningRequest,
) -> Result<RawSignature> {
    exchange_sequence_for_signature(transport, &build_scheduled_transfer_commands(request))
}

/// Build transfer-with-schedule-and-memo signing commands.
pub fn build_scheduled_transfer_with_memo_commands(
    request: &ScheduledTransferWithMemoSigningRequest,
) -> Vec<ApduCommand> {
    let mut commands = vec![
        ApduCommand::new(
            Instruction::SignTransferScheduleAndMemo.as_u8(),
            P1_INITIAL_WITH_MEMO_SCHEDULE,
            NONE,
            request.header_address_schedule_memo_length.clone(),
        ),
        ApduCommand::new(
            Instruction::SignTransferScheduleAndMemo.as_u8(),
            P1_MEMO_SCHEDULE,
            NONE,
            request.memo.clone(),
        ),
    ];
    commands.extend(
        chunk_schedule_pairs(&request.schedule)
            .into_iter()
            .map(|payload| {
                ApduCommand::new(
                    Instruction::SignTransferScheduleAndMemo.as_u8(),
                    P1_SCHEDULED_TRANSFER_PAIRS,
                    NONE,
                    payload,
                )
            }),
    );
    commands
}

/// Execute transfer-with-schedule-and-memo signing and return the raw signature.
pub fn sign_scheduled_transfer_with_memo<T: LedgerTransport>(
    transport: &mut T,
    request: &ScheduledTransferWithMemoSigningRequest,
) -> Result<RawSignature> {
    exchange_sequence_for_signature(
        transport,
        &build_scheduled_transfer_with_memo_commands(request),
    )
}

/// Build configure-baker signing commands.
pub fn build_configure_baker_commands(request: &ConfigureBakerSigningRequest) -> Vec<ApduCommand> {
    let ins = Instruction::SignConfigureBaker.as_u8();
    vec![
        ApduCommand::new(
            ins,
            P1_INITIAL_PACKET,
            NONE,
            request.header_kind_and_bitmap.clone(),
        ),
        ApduCommand::new(ins, P1_FIRST_BATCH, NONE, request.first_batch.clone()),
        ApduCommand::new(
            ins,
            P1_AGGREGATION_KEY,
            NONE,
            request.aggregation_keys.clone(),
        ),
        ApduCommand::new(ins, P1_URL_LENGTH, NONE, request.url_length.clone()),
        ApduCommand::new(ins, P1_URL, NONE, request.url.clone()),
        ApduCommand::new(ins, P1_COMMISSION_FEE, NONE, request.commission_fee.clone()),
        ApduCommand::new(ins, P1_SUSPENDED, NONE, request.suspended.clone()),
    ]
}

/// Execute configure-baker signing and return the raw signature.
pub fn sign_configure_baker<T: LedgerTransport>(
    transport: &mut T,
    request: &ConfigureBakerSigningRequest,
) -> Result<RawSignature> {
    exchange_sequence_for_signature(transport, &build_configure_baker_commands(request))
}

/// Build register-data signing commands.
pub fn build_register_data_commands(request: &RegisterDataSigningRequest) -> Vec<ApduCommand> {
    let mut commands = vec![ApduCommand::new(
        Instruction::SignRegisterData.as_u8(),
        P1_INITIAL_PACKET,
        NONE,
        request.header.clone(),
    )];
    commands.extend(chunk_payload(&request.data).into_iter().map(|payload| {
        ApduCommand::new(
            Instruction::SignRegisterData.as_u8(),
            P1_DATA,
            NONE,
            payload,
        )
    }));
    commands
}

/// Execute register-data signing and return the raw signature.
pub fn sign_register_data<T: LedgerTransport>(
    transport: &mut T,
    request: &RegisterDataSigningRequest,
) -> Result<RawSignature> {
    exchange_sequence_for_signature(transport, &build_register_data_commands(request))
}

/// Build transfer-to-public signing commands.
pub fn build_transfer_to_public_commands(
    request: &TransferToPublicSigningRequest,
) -> Vec<ApduCommand> {
    let mut commands = vec![
        ApduCommand::new(
            Instruction::SignTransferToPublic.as_u8(),
            P1_INITIAL_PACKET,
            NONE,
            request.header.clone(),
        ),
        ApduCommand::new(
            Instruction::SignTransferToPublic.as_u8(),
            P1_REMAINING_AMOUNT,
            NONE,
            request.amount_recipient_proofs_length.clone(),
        ),
    ];
    commands.extend(chunk_payload(&request.proofs).into_iter().map(|payload| {
        ApduCommand::new(
            Instruction::SignTransferToPublic.as_u8(),
            P1_PROOF,
            NONE,
            payload,
        )
    }));
    commands
}

/// Execute transfer-to-public signing and return the raw signature.
pub fn sign_transfer_to_public<T: LedgerTransport>(
    transport: &mut T,
    request: &TransferToPublicSigningRequest,
) -> Result<RawSignature> {
    exchange_sequence_for_signature(transport, &build_transfer_to_public_commands(request))
}

/// Build deploy-module staged signing commands.
pub fn build_deploy_module_commands(
    request: &DeployModuleSigningRequest,
) -> Result<Vec<ApduCommand>> {
    let mut header_chunks = chunk_payload_with_path(&request.path, &request.header_and_version)?;
    if header_chunks.len() != 1 {
        return Err(LedgerError::invalid_request(
            "deploy-module header/version stage must fit in one APDU command",
        ));
    }
    if request.source.is_empty() {
        return Err(LedgerError::invalid_request(
            "deploy-module source is empty",
        ));
    }
    let mut commands = vec![ApduCommand::new(
        Instruction::SignDeployModule.as_u8(),
        P1_INITIAL_PACKET,
        P2_LAST,
        header_chunks.remove(0),
    )];
    let source_chunks = chunk_payload(&request.source);
    let last_index = source_chunks.len().saturating_sub(1);
    for (index, payload) in source_chunks.into_iter().enumerate() {
        let p2 = if index == last_index {
            P2_LAST
        } else {
            P2_MORE
        };
        commands.push(ApduCommand::new(
            Instruction::SignDeployModule.as_u8(),
            P1_SOURCE,
            p2,
            payload,
        ));
    }
    Ok(commands)
}

/// Execute deploy-module signing and return the raw signature.
pub fn sign_deploy_module<T: LedgerTransport>(
    transport: &mut T,
    request: &DeployModuleSigningRequest,
) -> Result<RawSignature> {
    let commands = build_deploy_module_commands(request)?;
    exchange_sequence_for_signature(transport, &commands)
}

/// Build init-contract staged signing commands.
pub fn build_init_contract_commands(request: &ContractSigningRequest) -> Result<Vec<ApduCommand>> {
    build_contract_commands(Instruction::SignInitContract, request)
}

/// Execute init-contract signing and return the raw signature.
pub fn sign_init_contract<T: LedgerTransport>(
    transport: &mut T,
    request: &ContractSigningRequest,
) -> Result<RawSignature> {
    let commands = build_init_contract_commands(request)?;
    exchange_sequence_for_signature(transport, &commands)
}

/// Build update-contract staged signing commands.
pub fn build_update_contract_commands(
    request: &ContractSigningRequest,
) -> Result<Vec<ApduCommand>> {
    build_contract_commands(Instruction::SignUpdateContract, request)
}

/// Execute update-contract signing and return the raw signature.
pub fn sign_update_contract<T: LedgerTransport>(
    transport: &mut T,
    request: &ContractSigningRequest,
) -> Result<RawSignature> {
    let commands = build_update_contract_commands(request)?;
    exchange_sequence_for_signature(transport, &commands)
}

/// Build public-info-for-IP signing commands.
pub fn build_public_info_for_ip_commands(
    request: &PublicInfoForIpSigningRequest,
) -> Vec<ApduCommand> {
    let mut commands = vec![ApduCommand::new(
        Instruction::SignPublicInfoForIp.as_u8(),
        P1_INITIAL_PACKET,
        NONE,
        request.initial.clone(),
    )];
    commands.extend(request.keys.iter().cloned().map(|payload| {
        ApduCommand::new(
            Instruction::SignPublicInfoForIp.as_u8(),
            P1_VERIFICATION_KEY,
            NONE,
            payload,
        )
    }));
    commands.push(ApduCommand::new(
        Instruction::SignPublicInfoForIp.as_u8(),
        P1_SIGNATURE_THRESHOLD,
        NONE,
        request.threshold.clone(),
    ));
    commands
}

/// Execute public-info-for-IP signing and return the raw signature.
pub fn sign_public_info_for_ip<T: LedgerTransport>(
    transport: &mut T,
    request: &PublicInfoForIpSigningRequest,
) -> Result<RawSignature> {
    exchange_sequence_for_signature(transport, &build_public_info_for_ip_commands(request))
}

/// Build credential-deployment signing commands.
pub fn build_credential_deployment_commands(
    request: &CredentialDeploymentSigningRequest,
) -> Vec<ApduCommand> {
    let ins = Instruction::SignCredentialDeployment.as_u8();
    let mut commands = vec![ApduCommand::new(
        ins,
        P1_INITIAL_PACKET,
        NONE,
        request.path.clone(),
    )];
    push_credential_payload(&mut commands, ins, NONE, &request.credential);
    let mut final_payload = Vec::new();
    match &request.context {
        CredentialDeploymentContext::New { expiry } => {
            final_payload.push(0);
            final_payload.extend_from_slice(expiry);
        }
        CredentialDeploymentContext::Existing { account_address } => {
            final_payload.push(1);
            final_payload.extend_from_slice(account_address);
        }
    }
    commands.push(ApduCommand::new(
        ins,
        P1_NEW_OR_EXISTING,
        NONE,
        final_payload,
    ));
    commands
}

/// Execute credential-deployment signing and return the raw signature.
pub fn sign_credential_deployment<T: LedgerTransport>(
    transport: &mut T,
    request: &CredentialDeploymentSigningRequest,
) -> Result<RawSignature> {
    exchange_sequence_for_signature(transport, &build_credential_deployment_commands(request))
}

/// Build update-credentials signing commands.
pub fn build_update_credentials_commands(
    request: &UpdateCredentialsSigningRequest,
) -> Vec<ApduCommand> {
    let ins = Instruction::SignUpdateCredentials.as_u8();
    let mut commands = vec![ApduCommand::new(
        ins,
        NONE,
        P2_CREDENTIAL_INITIAL,
        request.header_kind_and_index_length.clone(),
    )];
    for entry in &request.new_credentials {
        commands.push(ApduCommand::new(
            ins,
            NONE,
            P2_CREDENTIAL_CREDENTIAL_INDEX,
            entry.credential_index.clone(),
        ));
        push_credential_payload(
            &mut commands,
            ins,
            P2_CREDENTIAL_CREDENTIAL,
            &entry.credential,
        );
    }
    commands.push(ApduCommand::new(
        ins,
        NONE,
        P2_CREDENTIAL_ID_COUNT,
        request.credential_id_count.clone(),
    ));
    commands.extend(
        request
            .credential_ids
            .iter()
            .cloned()
            .map(|payload| ApduCommand::new(ins, NONE, P2_CREDENTIAL_ID, payload)),
    );
    commands.push(ApduCommand::new(
        ins,
        NONE,
        P2_THRESHOLD,
        request.threshold.clone(),
    ));
    commands
}

/// Execute update-credentials signing and return the raw signature.
pub fn sign_update_credentials<T: LedgerTransport>(
    transport: &mut T,
    request: &UpdateCredentialsSigningRequest,
) -> Result<RawSignature> {
    exchange_sequence_for_signature(transport, &build_update_credentials_commands(request))
}

fn chunk_schedule_pairs(schedule: &[u8]) -> Vec<Vec<u8>> {
    schedule
        .chunks(MAX_SCHEDULE_PAIR_CHUNK_BYTES)
        .map(<[u8]>::to_vec)
        .collect()
}

fn build_contract_commands(
    instruction: Instruction,
    request: &ContractSigningRequest,
) -> Result<Vec<ApduCommand>> {
    let mut header_chunks = chunk_payload_with_path(&request.path, &request.header_and_data)?;
    if header_chunks.len() != 1 {
        return Err(LedgerError::invalid_request(
            "contract header/data stage must fit in one APDU command",
        ));
    }
    let mut commands = vec![ApduCommand::new(
        instruction.as_u8(),
        P1_INITIAL_PACKET,
        NONE,
        header_chunks.remove(0),
    )];
    for payload in chunk_payload(&length_prefix_u16(&request.name)?) {
        commands.push(ApduCommand::new(
            instruction.as_u8(),
            P1_NAME,
            NONE,
            payload,
        ));
    }
    for payload in chunk_payload(&length_prefix_u16(&request.parameter)?) {
        commands.push(ApduCommand::new(
            instruction.as_u8(),
            P1_PARAM,
            NONE,
            payload,
        ));
    }
    Ok(commands)
}

fn push_credential_payload(
    commands: &mut Vec<ApduCommand>,
    ins: u8,
    p2: u8,
    credential: &CredentialSigningPayload,
) {
    commands.push(ApduCommand::new(
        ins,
        P1_VERIFICATION_KEY_LENGTH,
        p2,
        credential.verification_key_count.clone(),
    ));
    commands.push(ApduCommand::new(
        ins,
        P1_VERIFICATION_KEY,
        p2,
        credential.key_index_scheme_public_key.clone(),
    ));
    commands.push(ApduCommand::new(
        ins,
        P1_SIGNATURE_THRESHOLD,
        p2,
        credential.threshold_credential_id_identity.clone(),
    ));
    commands.push(ApduCommand::new(
        ins,
        P1_AR_IDENTITY,
        p2,
        credential.ar_identity.clone(),
    ));
    commands.push(ApduCommand::new(
        ins,
        P1_CREDENTIAL_DATES,
        p2,
        credential.credential_dates.clone(),
    ));
    for attribute in &credential.attributes {
        let mut tag = attribute.tag.clone();
        tag.push(attribute.value.len() as u8);
        commands.push(ApduCommand::new(ins, P1_ATTRIBUTE_TAG, p2, tag));
        commands.push(ApduCommand::new(
            ins,
            P1_ATTRIBUTE_VALUE,
            p2,
            attribute.value.clone(),
        ));
    }
    commands.push(ApduCommand::new(
        ins,
        P1_LENGTH_OF_PROOFS,
        p2,
        credential.proof_length.clone(),
    ));
    commands.extend(
        chunk_payload(&credential.proofs)
            .into_iter()
            .map(|payload| ApduCommand::new(ins, P1_PROOFS, p2, payload)),
    );
}

fn exchange_sequence_for_signature<T: LedgerTransport>(
    transport: &mut T,
    commands: &[ApduCommand],
) -> Result<RawSignature> {
    let mut last = Vec::new();
    for command in commands {
        last = transport.exchange(command)?.data;
    }
    RawSignature::from_response(last)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CredentialAttribute, DerivationPath, MockTransport, UpdateCredentialEntry};

    fn ok_reply(data: impl Into<Vec<u8>>) -> Vec<u8> {
        let mut data = data.into();
        data.extend_from_slice(&[0x90, 0x00]);
        data
    }

    fn signature_reply() -> Vec<u8> {
        ok_reply(vec![9; 64])
    }

    fn credential_payload() -> CredentialSigningPayload {
        CredentialSigningPayload {
            verification_key_count: vec![1],
            key_index_scheme_public_key: vec![2],
            threshold_credential_id_identity: vec![3],
            ar_identity: vec![4],
            credential_dates: vec![5],
            attributes: vec![CredentialAttribute {
                tag: vec![6],
                value: vec![7, 8],
            }],
            proof_length: vec![0, 0, 0, 1],
            proofs: vec![9],
        }
    }

    #[test]
    fn chunked_signing_marks_only_last_chunk_as_last() {
        let request =
            ChunkedSigningRequest::new(DerivationPath::new([1]).unwrap(), vec![0xAA; 300]).unwrap();
        let commands = build_chunked_signing_commands(Instruction::SignTransfer, &request).unwrap();
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].p2, P2_MORE);
        assert_eq!(commands[1].p2, P2_LAST);
    }

    #[test]
    fn staged_transfer_with_memo_uses_expected_p1_sequence() {
        let request = TransferWithMemoSigningRequest {
            header_address_memo_length: vec![1],
            memo: vec![2],
            amount: vec![3],
        };
        let commands = build_transfer_with_memo_commands(&request);
        assert_eq!(
            commands.iter().map(|c| c.p1).collect::<Vec<_>>(),
            vec![P1_INITIAL_WITH_MEMO, P1_MEMO, P1_AMOUNT]
        );
    }

    #[test]
    fn scheduled_transfer_chunks_schedule_pairs() {
        let request = ScheduledTransferSigningRequest {
            header_address_schedule_length: vec![1],
            schedule: vec![2; 300],
        };
        let commands = build_scheduled_transfer_commands(&request);
        assert_eq!(commands.len(), 3);
        assert_eq!(commands[1].data.len(), 240);
        assert_eq!(commands[1].p1, P1_SCHEDULED_TRANSFER_PAIRS);
        assert_eq!(commands[2].p1, P1_SCHEDULED_TRANSFER_PAIRS);
    }

    #[test]
    fn configure_baker_uses_reference_stage_order() {
        let request = ConfigureBakerSigningRequest {
            header_kind_and_bitmap: vec![1],
            first_batch: vec![2],
            aggregation_keys: vec![3],
            url_length: vec![4],
            url: vec![5],
            commission_fee: vec![6],
            suspended: vec![7],
        };
        let commands = build_configure_baker_commands(&request);
        assert_eq!(
            commands.iter().map(|c| c.p1).collect::<Vec<_>>(),
            vec![0, 1, 2, 3, 4, 5, 6]
        );
    }

    #[test]
    fn contract_commands_stage_header_name_and_parameter() {
        let request = ContractSigningRequest {
            path: DerivationPath::new([1]).unwrap(),
            header_and_data: vec![2],
            name: b"init_contract".to_vec(),
            parameter: vec![3, 4],
        };
        let commands = build_init_contract_commands(&request).unwrap();
        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0].p1, P1_INITIAL_PACKET);
        assert_eq!(commands[1].p1, P1_NAME);
        assert_eq!(commands[2].p1, P1_PARAM);
        assert_eq!(&commands[1].data[..2], &(13u16.to_be_bytes()));
        assert_eq!(&commands[2].data[..2], &(2u16.to_be_bytes()));
    }

    #[test]
    fn credential_deployment_includes_final_new_or_existing_stage() {
        let request = CredentialDeploymentSigningRequest {
            path: vec![1],
            credential: credential_payload(),
            context: CredentialDeploymentContext::New { expiry: vec![0; 8] },
        };
        let commands = build_credential_deployment_commands(&request);
        assert_eq!(commands.last().unwrap().p1, P1_NEW_OR_EXISTING);
        assert_eq!(commands.last().unwrap().data[0], 0);
    }

    #[test]
    fn update_credentials_uses_p2_subprotocol() {
        let request = UpdateCredentialsSigningRequest {
            header_kind_and_index_length: vec![1],
            new_credentials: vec![UpdateCredentialEntry {
                credential_index: vec![0],
                credential: credential_payload(),
            }],
            credential_id_count: vec![1],
            credential_ids: vec![vec![2]],
            threshold: vec![1],
        };
        let commands = build_update_credentials_commands(&request);
        assert_eq!(commands[0].p2, P2_CREDENTIAL_INITIAL);
        assert!(
            commands
                .iter()
                .any(|command| command.p2 == P2_CREDENTIAL_CREDENTIAL)
        );
        assert_eq!(commands[commands.len() - 2].p2, P2_CREDENTIAL_ID);
        assert_eq!(commands.last().unwrap().p2, P2_THRESHOLD);
    }

    #[test]
    fn mock_transport_captures_signing_sequence() {
        let request =
            ChunkedSigningRequest::new(DerivationPath::new([1]).unwrap(), vec![2]).unwrap();
        let mut transport = MockTransport::new([signature_reply()]);
        let signature = sign_chunked(&mut transport, Instruction::SignTransfer, &request).unwrap();
        assert_eq!(signature.0, [9; 64]);
        assert_eq!(transport.commands().len(), 1);
        assert_eq!(
            transport.commands()[0].ins,
            Instruction::SignTransfer.as_u8()
        );
    }
}
