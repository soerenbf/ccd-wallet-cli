use anyhow::{Context, Result, bail};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256, Sha512};
use zeroize::Zeroizing;

use concordium_rust_sdk::base::{
    base::CredentialRegistrationID,
    curve_arithmetic::{Curve, Field, PrimeField},
    dodis_yampolskiy_prf,
    id::{
        constants::{ArCurve, BaseField, IpPairing},
        pedersen_commitment::Randomness as CommitmentRandomness,
        types::{AttributeTag, GlobalContext},
    },
    ps_sig::SigRetrievalRandomness,
};
use ed25519_dalek::SigningKey;

const HARDENED_OFFSET: u32 = 1 << 31;
const ED25519_CURVE: &[u8] = b"ed25519 seed";

pub type Fr = BaseField;
pub type CredId = <ArCurve as Curve>::Scalar;
pub type PrfKey = dodis_yampolskiy_prf::SecretKey<ArCurve>;

/// A zeroizing extended Ed25519 SLIP-10 node.
///
/// The node contains the private key and chain code needed to derive hardened child nodes. It is
/// used for transient Ledger account-creation commitment-randomness exports.
pub struct ExtendedSlip10Node {
    private_key: Zeroizing<[u8; 32]>,
    chain_code: Zeroizing<[u8; 32]>,
}

impl ExtendedSlip10Node {
    /// Construct an extended node from `private_key || chain_code` bytes.
    ///
    /// # Arguments
    ///
    /// * `bytes` - Zeroizing 64-byte extended-node encoding.
    ///
    /// # Returns
    ///
    /// A node that clears its private key and chain code when dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// use ccd_wallet_core::wallet::ExtendedSlip10Node;
    /// use zeroize::Zeroizing;
    ///
    /// let node = ExtendedSlip10Node::from_bytes(Zeroizing::new([0; 64]));
    /// let _child = node.derive_hardened_child(0).unwrap();
    /// ```
    pub fn from_bytes(bytes: Zeroizing<[u8; 64]>) -> Self {
        let mut private_key = Zeroizing::new([0; 32]);
        private_key.copy_from_slice(&bytes[..32]);
        let mut chain_code = Zeroizing::new([0; 32]);
        chain_code.copy_from_slice(&bytes[32..]);
        Self {
            private_key,
            chain_code,
        }
    }

    /// Derive a hardened child node.
    ///
    /// # Arguments
    ///
    /// * `index` - Unhardened child index, which must be smaller than `2^31`.
    ///
    /// # Returns
    ///
    /// A zeroizing node containing the derived child private key and chain code.
    ///
    /// # Errors
    ///
    /// Returns an error if `index` cannot be represented as a hardened SLIP-10 index.
    pub fn derive_hardened_child(&self, index: u32) -> Result<Self> {
        let index = harden(index)?;
        let mut data = Zeroizing::new([0u8; 37]);
        data[1..33].copy_from_slice(&self.private_key[..]);
        data[33..].copy_from_slice(&index.to_be_bytes());

        let mut mac = Hmac::<Sha512>::new_from_slice(&self.chain_code[..])
            .expect("HMAC accepts keys of any length");
        mac.update(&data[..]);
        // `GenericArray` returned by `hmac` does not implement `Zeroize`; copy it immediately
        // into the zeroizing child representation.
        let digest = mac.finalize().into_bytes();
        let mut child = Zeroizing::new([0; 64]);
        child.copy_from_slice(&digest);

        let mut private_key = Zeroizing::new([0; 32]);
        private_key.copy_from_slice(&child[..32]);
        let mut chain_code = Zeroizing::new([0; 32]);
        chain_code.copy_from_slice(&child[32..]);
        Ok(Self {
            private_key,
            chain_code,
        })
    }

    /// Borrow this node's zeroizing private key.
    fn private_key(&self) -> &Zeroizing<[u8; 32]> {
        &self.private_key
    }

    /// Derive commitment randomness for a hardened attribute tag.
    pub fn derive_attribute_commitment_randomness(
        &self,
        attribute_tag: AttributeTag,
    ) -> Result<CommitmentRandomness<ArCurve>> {
        let child = self.derive_hardened_child(u32::from(attribute_tag.0))?;
        Ok(CommitmentRandomness::new(keygen_bls(
            &**child.private_key(),
        )?))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Net {
    Mainnet,
    Testnet,
}

impl Net {
    pub fn net_code(self) -> u32 {
        match self {
            Net::Mainnet => 919,
            Net::Testnet => 1,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConcordiumHdWallet {
    pub seed: [u8; 64],
    pub net: Net,
}

impl ConcordiumHdWallet {
    pub fn from_seed_phrase(phrase: &str, net: Net) -> Result<Self> {
        let words_count = phrase.split_whitespace().count();
        if ![12, 15, 18, 21, 24].contains(&words_count) {
            bail!("invalid mnemonic length: expected 12, 15, 18, 21, or 24 words");
        }

        let mnemonic = bip39::Mnemonic::parse(phrase).context("invalid seed phrase")?;
        let seed = mnemonic.to_seed("");
        Ok(Self { seed, net })
    }

    /// Derive the zeroizing account signing key for a credential.
    pub fn get_account_signing_key(
        &self,
        identity_provider_index: u32,
        identity_index: u32,
        credential_counter: u32,
    ) -> Result<Zeroizing<[u8; 32]>> {
        self.derive_identity_key(
            identity_provider_index,
            identity_index,
            0,
            [credential_counter],
        )
    }

    pub fn get_account_public_key(
        &self,
        identity_provider_index: u32,
        identity_index: u32,
        credential_counter: u32,
    ) -> Result<[u8; 32]> {
        let signing_key = self.get_account_signing_key(
            identity_provider_index,
            identity_index,
            credential_counter,
        )?;
        Ok(SigningKey::from_bytes(&signing_key)
            .verifying_key()
            .to_bytes())
    }

    pub fn get_id_cred_sec(
        &self,
        identity_provider_index: u32,
        identity_index: u32,
    ) -> Result<CredId> {
        let key_seed = self.derive_identity_key_leaf(identity_provider_index, identity_index, 2)?;
        keygen_bls(&*key_seed)
    }

    pub fn get_prf_key(&self, identity_provider_index: u32, identity_index: u32) -> Result<PrfKey> {
        let key_seed = self.derive_identity_key_leaf(identity_provider_index, identity_index, 3)?;
        Ok(PrfKey::new(keygen_bls(&*key_seed)?))
    }

    pub fn get_blinding_randomness(
        &self,
        identity_provider_index: u32,
        identity_index: u32,
    ) -> Result<SigRetrievalRandomness<IpPairing>> {
        let key_seed = self.derive_identity_key_leaf(identity_provider_index, identity_index, 4)?;
        Ok(SigRetrievalRandomness::new(keygen_bls(&*key_seed)?))
    }

    pub fn get_credential_registration_id(
        &self,
        identity_provider_index: u32,
        identity_index: u32,
        credential_counter: u8,
        global_context: &GlobalContext<ArCurve>,
    ) -> Result<CredentialRegistrationID> {
        let prf_key = self.get_prf_key(identity_provider_index, identity_index)?;
        let cred_id_exponent = prf_key
            .prf_exponent(credential_counter)
            .context("failed to derive credential registration id exponent")?;
        Ok(CredentialRegistrationID::from_exponent(
            global_context,
            cred_id_exponent,
        ))
    }

    pub fn get_attribute_commitment_randomness(
        &self,
        identity_provider_index: u32,
        identity_index: u32,
        credential_counter: u32,
        attribute_tag: AttributeTag,
    ) -> Result<CommitmentRandomness<ArCurve>> {
        let key_seed = self.derive_identity_key(
            identity_provider_index,
            identity_index,
            5,
            [credential_counter, u32::from(attribute_tag.0)],
        )?;
        Ok(CommitmentRandomness::new(keygen_bls(&*key_seed)?))
    }

    fn derive_identity_key_leaf(
        &self,
        identity_provider_index: u32,
        identity_index: u32,
        leaf: u32,
    ) -> Result<Zeroizing<[u8; 32]>> {
        self.derive_identity_key(identity_provider_index, identity_index, leaf, [])
    }

    fn derive_identity_key(
        &self,
        identity_provider_index: u32,
        identity_index: u32,
        leaf: u32,
        suffix: impl IntoIterator<Item = u32>,
    ) -> Result<Zeroizing<[u8; 32]>> {
        let mut path = vec![
            harden(44)?,
            harden(self.net.net_code())?,
            harden(identity_provider_index)?,
            harden(identity_index)?,
            harden(leaf)?,
        ];
        for index in suffix {
            path.push(harden(index)?);
        }
        Ok(slip10_derive(&self.seed, &path))
    }
}

pub fn harden(index: u32) -> Result<u32> {
    if index >= HARDENED_OFFSET {
        bail!("derivation path index {index} is too large to harden");
    }
    Ok(index + HARDENED_OFFSET)
}

/// Derive a zeroizing private key from a hardened SLIP-10 path.
pub fn slip10_derive(seed: &[u8; 64], path: &[u32]) -> Zeroizing<[u8; 32]> {
    let node = slip10_derive_extended(seed, path);
    Zeroizing::new(**node.private_key())
}

fn slip10_derive_extended(seed: &[u8; 64], path: &[u32]) -> ExtendedSlip10Node {
    let mut mac =
        Hmac::<Sha512>::new_from_slice(ED25519_CURVE).expect("HMAC accepts keys of any length");
    mac.update(seed);
    // `GenericArray` returned by `hmac` does not implement `Zeroize`; copy it immediately
    // into the zeroizing extended-node representation.
    let digest = mac.finalize().into_bytes();
    let mut master = Zeroizing::new([0; 64]);
    master.copy_from_slice(&digest);
    let node = ExtendedSlip10Node::from_bytes(master);

    path.iter().fold(node, |node, index| {
        assert!(
            index & HARDENED_OFFSET != 0,
            "SLIP-0010 path must be hardened"
        );
        node.derive_hardened_child(index - HARDENED_OFFSET)
            .expect("validated hardened SLIP-10 index")
    })
}

pub fn keygen_bls(key_seed: &[u8]) -> Result<Fr> {
    let mut ikm = Zeroizing::new(key_seed.to_vec());
    ikm.push(0);

    let key_info = b"";
    let l = 48u8;
    let mut l_bytes = key_info.to_vec();
    l_bytes.push(0);
    l_bytes.push(l);

    let mut sk = Fr::zero();
    let shift = Fr::from_repr(&[0, 0, 0, 72_057_594_037_927_936])
        .context("failed to construct BLS keygen shift constant")?;
    let mut salt = Sha256::digest(b"BLS-SIG-KEYGEN-SALT-");

    while sk.is_zero() {
        let (_, hkdf) = hkdf::Hkdf::<Sha256>::extract(Some(&salt), &ikm);
        let mut okm = Zeroizing::new(vec![0u8; l as usize]);
        hkdf.expand(&l_bytes, &mut okm)
            .map_err(|_| anyhow::anyhow!("failed to expand BLS key material"))?;
        okm.reverse();

        let mut y1_vec = Zeroizing::new([0u8; 32]);
        let mut y2_vec = Zeroizing::new([0u8; 32]);
        y1_vec[..31].copy_from_slice(&okm[..31]);
        y2_vec[..17].copy_from_slice(&okm[31..]);

        // The SDK takes scalar encodings by value, so these are the unavoidable final
        // third-party value constructions; their source buffers remain zeroizing.
        let y1 = ArCurve::scalar_from_bytes(*y1_vec);
        let mut y2 = ArCurve::scalar_from_bytes(*y2_vec);
        y2.mul_assign(&shift);
        sk = y1;
        sk.add_assign(&y2);
        salt = Sha256::digest(salt);
    }

    Ok(sk)
}

#[cfg(test)]
mod tests {
    use super::*;
    use concordium_rust_sdk::base::common::base16_encode_string;

    const TEST_SEED_1: &str = "efa5e27326f8fa0902e647b52449bf335b7b605adc387015ec903f41d95080eb71361cbc7fb78721dcd4f3926a337340aa1406df83332c44c1cdcfe100603860";

    fn wallet(net: Net) -> ConcordiumHdWallet {
        let decoded = hex::decode(TEST_SEED_1).unwrap();
        let seed: [u8; 64] = decoded.try_into().unwrap();
        ConcordiumHdWallet { seed, net }
    }

    #[test]
    fn extended_node_attribute_children_match_full_seed_paths() {
        let wallet = wallet(Net::Mainnet);
        let root_path = [
            harden(44).unwrap(),
            harden(Net::Mainnet.net_code()).unwrap(),
            harden(5).unwrap(),
            harden(0).unwrap(),
            harden(5).unwrap(),
            harden(4).unwrap(),
        ];
        let root = slip10_derive_extended(&wallet.seed, &root_path);

        for tag in [0u8, 1, 47] {
            let child = root.derive_hardened_child(u32::from(tag)).unwrap();
            let derived = CommitmentRandomness::new(keygen_bls(&**child.private_key()).unwrap());
            let full_path = wallet
                .get_attribute_commitment_randomness(5, 0, 4, AttributeTag(tag))
                .unwrap();
            assert_eq!(derived, full_path, "attribute tag {tag}");
        }
    }

    #[test]
    fn derives_mainnet_account_key_material_from_key_derivation_vectors() {
        assert_eq!(
            hex::encode(
                wallet(Net::Mainnet)
                    .get_account_signing_key(0, 55, 7)
                    .unwrap()
            ),
            "e4d1693c86eb9438feb9cbc3d561fbd9299e3a8b3a676eb2483b135f8dbf6eb1"
        );
        assert_eq!(
            hex::encode(
                wallet(Net::Mainnet)
                    .get_account_public_key(1, 341, 9)
                    .unwrap()
            ),
            "d54aab7218fc683cbd4d822f7c2b4e7406c41ae08913012fab0fa992fa008e98"
        );
        assert_eq!(
            base16_encode_string(
                &wallet(Net::Mainnet)
                    .get_attribute_commitment_randomness(
                        5,
                        0,
                        4,
                        concordium_rust_sdk::id::types::AttributeTag(0),
                    )
                    .unwrap()
            ),
            "6ef6ba6490fa37cd517d2b89a12b77edf756f89df5e6f5597440630cd4580b8f"
        );
    }

    #[test]
    fn derives_testnet_account_key_material_from_key_derivation_vectors() {
        assert_eq!(
            hex::encode(
                wallet(Net::Testnet)
                    .get_account_signing_key(0, 55, 7)
                    .unwrap()
            ),
            "aff97882c6df085e91ae2695a32d39dccb8f4b8d68d2f0db9637c3a95f845e3c"
        );
        assert_eq!(
            hex::encode(
                wallet(Net::Testnet)
                    .get_account_public_key(1, 341, 9)
                    .unwrap()
            ),
            "ef6fd561ca0291a57cdfee896245db9803a86da74c9a6c1bf0252b18f8033003"
        );
        assert_eq!(
            base16_encode_string(
                &wallet(Net::Testnet)
                    .get_attribute_commitment_randomness(
                        5,
                        0,
                        4,
                        concordium_rust_sdk::id::types::AttributeTag(0),
                    )
                    .unwrap()
            ),
            "409fa90314ec8fb4a2ae812fd77fe58bfac81765cad3990478ff7a73ba6d88ae"
        );
    }

    #[test]
    fn derives_mainnet_identity_key_material_from_vectors() {
        assert_eq!(
            base16_encode_string(&wallet(Net::Mainnet).get_id_cred_sec(2, 115).unwrap()),
            "33b9d19b2496f59ed853eb93b9d374482d2e03dd0a12e7807929d6ee54781bb1"
        );
        assert_eq!(
            base16_encode_string(&wallet(Net::Mainnet).get_prf_key(3, 35).unwrap()),
            "4409e2e4acffeae641456b5f7406ecf3e1e8bd3472e2df67a9f1e8574f211bc5"
        );
        assert_eq!(
            base16_encode_string(
                &wallet(Net::Mainnet)
                    .get_blinding_randomness(4, 5713)
                    .unwrap()
            ),
            "1e3633af2b1dbe5600becfea0324bae1f4fa29f90bdf419f6fba1ff520cb3167"
        );
    }

    #[test]
    fn derives_testnet_identity_key_material_from_vectors() {
        assert_eq!(
            base16_encode_string(&wallet(Net::Testnet).get_id_cred_sec(2, 115).unwrap()),
            "33c9c538e362c5ac836afc08210f4b5d881ba65a0a45b7e353586dad0a0f56df"
        );
        assert_eq!(
            base16_encode_string(&wallet(Net::Testnet).get_prf_key(3, 35).unwrap()),
            "41d794d0b06a7a31fb79bb76c44e6b87c63e78f9afe8a772fc64d20f3d9e8e82"
        );
        assert_eq!(
            base16_encode_string(
                &wallet(Net::Testnet)
                    .get_blinding_randomness(4, 5713)
                    .unwrap()
            ),
            "079eb7fe4a2e89007f411ede031543bd7f687d50341a5596e015c9f2f4c1f39b"
        );
    }
}
