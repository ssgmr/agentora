//! 操作签名与验证

use ed25519_dalek::{Signature, SigningKey, VerifyingKey, Verifier};
use crate::types::PeerId;
use serde::{Deserialize, Serialize};
use hex::ToHex;

/// 签名器
pub struct Signer {
    signing_key: SigningKey,
    peer_id: PeerId,
}

impl Signer {
    /// 从随机种子生成
    pub fn new() -> Self {
        let mut csprng = rand::thread_rng();
        let signing_key = SigningKey::generate(&mut csprng);
        let peer_id = PeerId::new(signing_key.verifying_key().to_bytes().encode_hex::<String>());

        Self { signing_key, peer_id }
    }

    /// 签名数据
    pub fn sign(&self, data: &[u8]) -> SignedData {
        use ed25519_dalek::Signer;
        let signature = self.signing_key.sign(data);
        SignedData {
            data: data.to_vec(),
            signature: signature.to_bytes().to_vec(),
            peer_id: self.peer_id.clone(),
        }
    }

    /// 获取PeerId
    pub fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }
}

impl Default for Signer {
    fn default() -> Self {
        Self::new()
    }
}

/// 已签名数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedData {
    pub data: Vec<u8>,
    pub signature: Vec<u8>,
    pub peer_id: PeerId,
}

impl SignedData {
    /// 验证签名
    pub fn verify(&self) -> Result<(), SignatureError> {
        // 从PeerId重建公钥
        let public_key_bytes = hex::decode(&self.peer_id.0)
            .map_err(|_| SignatureError::InvalidPublicKey)?;
        let verifying_key = VerifyingKey::from_bytes(
            &public_key_bytes.try_into().map_err(|_| SignatureError::InvalidPublicKey)?
        ).map_err(|_| SignatureError::InvalidPublicKey)?;

        let signature = Signature::from_bytes(
            &self.signature.clone().try_into().map_err(|_| SignatureError::InvalidSignature)?
        );

        verifying_key.verify(&self.data, &signature)
            .map_err(|_| SignatureError::VerificationFailed)?;

        Ok(())
    }
}

/// 签名错误
#[derive(Debug, Clone)]
pub enum SignatureError {
    InvalidPublicKey,
    InvalidSignature,
    VerificationFailed,
}