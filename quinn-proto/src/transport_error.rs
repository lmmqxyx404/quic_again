use std::fmt;

/// Transport-level errors occur when a peer violates the protocol specification
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Error {}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      /* self.code.fmt(f)?;
      if let Some(frame) = self.frame {
          write!(f, " in {frame}")?;
      }
      if !self.reason.is_empty() {
          write!(f, ": {}", self.reason)?;
      } */
      Ok(())
  }
}

impl std::error::Error for Error {}
