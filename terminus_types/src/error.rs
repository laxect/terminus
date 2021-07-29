use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("need pass to be un masked")]
    NeedUnMaskPass,
}

pub type Result<T> = std::result::Result<T, Error>;
