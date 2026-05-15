pub mod callback;
pub mod client;

use anyhow::{Result, bail};
use ccd_wallet_core::wallet::ConcordiumHdWallet;
use concordium_rust_sdk::base::{
    common::{VERSION_0, Versioned},
    id::{
        account_holder::generate_pio_v1,
        constants::{ArCurve, IpPairing},
        pedersen_commitment::Value as PedersenValue,
        secret_sharing::Threshold,
        types::{
            AccCredentialInfo, ArIdentity, ArInfo, CredentialHolderInfo, GlobalContext,
            IdCredentials, IdObjectUseData, IpContext, IpInfo, PreIdentityObjectV1,
        },
    },
};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize)]
struct IdentityObjectRequestV1 {
    #[serde(rename = "idObjectRequest")]
    id_object_request: Versioned<PreIdentityObjectV1<IpPairing, ArCurve>>,
}

pub fn build_request(
    wallet: &ConcordiumHdWallet,
    ip_info: &IpInfo<IpPairing>,
    ar_infos: &BTreeMap<ArIdentity, ArInfo<ArCurve>>,
    global_context: &GlobalContext<ArCurve>,
    identity_index: u32,
) -> Result<String> {
    let id_cred_sec = wallet.get_id_cred_sec(ip_info.ip_identity.0, identity_index)?;
    let prf_key = wallet.get_prf_key(ip_info.ip_identity.0, identity_index)?;
    let blinding_randomness =
        wallet.get_blinding_randomness(ip_info.ip_identity.0, identity_index)?;

    if ar_infos.is_empty() {
        bail!("identity request requires at least one anonymity revoker");
    }

    let threshold = Threshold::try_new(ar_infos.len() as u8)?;
    let context = IpContext {
        ip_info,
        ars_infos: ar_infos,
        global_context,
    };
    let id_use_data = IdObjectUseData {
        aci: AccCredentialInfo {
            cred_holder_info: CredentialHolderInfo {
                id_cred: IdCredentials {
                    id_cred_sec: PedersenValue::new(id_cred_sec),
                },
            },
            prf_key,
        },
        randomness: blinding_randomness,
    };

    let (pio, _) = generate_pio_v1(&context, threshold, &id_use_data)
        .ok_or_else(|| anyhow::anyhow!("generating the pre-identity object failed"))?;

    Ok(serde_json::to_string(&IdentityObjectRequestV1 {
        id_object_request: Versioned::new(VERSION_0, pio),
    })?)
}
