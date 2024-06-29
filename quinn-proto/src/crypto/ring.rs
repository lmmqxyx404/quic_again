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
