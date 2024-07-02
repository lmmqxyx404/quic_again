use crate::RESET_TOKEN_SIZE;

/// Stateless reset token
///
/// Used for an endpoint to securely communicate that it has lost state for a connection.
#[allow(clippy::derived_hash_with_manual_eq)] // Custom PartialEq impl matches derived semantics
#[derive(Debug, Copy, Clone, Hash)]
pub(crate) struct ResetToken([u8; RESET_TOKEN_SIZE]);

impl PartialEq for ResetToken {
  fn eq(&self, other: &Self) -> bool {
      crate::constant_time::eq(&self.0, &other.0)
  }
}

impl Eq for ResetToken {}
