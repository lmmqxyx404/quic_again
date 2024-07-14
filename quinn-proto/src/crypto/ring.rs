use ring::{aead, hkdf, hmac};

use crate::crypto::{self, CryptoError};

impl crypto::HmacKey for hmac::Key {
    fn sign(&self, data: &[u8], out: &mut [u8]) {
        out.copy_from_slice(hmac::sign(self, data).as_ref());
    }

    fn signature_len(&self) -> usize {
        32
    }

    fn verify(&self, data: &[u8], signature: &[u8]) -> Result<(), CryptoError> {
        Ok(hmac::verify(self, data, signature)?)
    }
}

impl From<ring::error::Unspecified> for CryptoError {
    fn from(_: ring::error::Unspecified) -> Self {
        Self
    }
}

impl crypto::HandshakeTokenKey for hkdf::Prk {
    fn aead_from_hkdf(&self, random_bytes: &[u8]) -> Box<dyn crypto::AeadKey> {
        let mut key_buffer = [0u8; 32];
        let info = [random_bytes];
        let okm = self.expand(&info, hkdf::HKDF_SHA256).unwrap();

        okm.fill(&mut key_buffer).unwrap();

        let key = aead::UnboundKey::new(&aead::AES_256_GCM, &key_buffer).unwrap();
        todo!()
        // Box::new(aead::LessSafeKey::new(key))
    }
}
