use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Deserialize, Serialize)]
pub enum Error {
    #[error("need pass to be un masked")]
    NeedUnMaskPass,
    #[error("already a node here")]
    NodeExist,
    #[error("not a node here")]
    NodeNotExist,
    #[error("node id not valid")]
    IdInvalid,
    #[error("node pass not match")]
    PassNotMatch,
    #[error("node is too old to delete")]
    DeleteLimitOverdue,
    #[error("can not link to peer")]
    NetworkError,
}

pub type Result<T> = std::result::Result<T, Error>;
