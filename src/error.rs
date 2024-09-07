use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
  #[error("Git operation failed: {0}")]
  Git(String),

  #[error(transparent)]
  ParsingError(#[from] std::string::FromUtf8Error),
}
