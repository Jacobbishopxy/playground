//! fabrix error type
//!
//! errors

use std::fmt::Display;

use polars::prelude::DataType;
use thiserror::Error;

pub type FabrixResult<T> = Result<T, FabrixError>;

type DataFrameDTypes<'a> = (&'a DataType, Vec<DataType>);

#[derive(Debug)]
pub enum CommonError {
    Str(&'static str),
    String(String),
}

impl std::fmt::Display for CommonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommonError::Str(v) => write!(f, "{:?}", v),
            CommonError::String(v) => write!(f, "{:?}", v),
        }
    }
}

impl From<&'static str> for CommonError {
    fn from(v: &'static str) -> Self {
        CommonError::Str(v)
    }
}

impl From<String> for CommonError {
    fn from(v: String) -> Self {
        CommonError::String(v)
    }
}

#[derive(Error, Debug)]
pub enum FabrixError {
    #[error("common error {0}")]
    Common(CommonError),

    #[error("parse {0} into {1} error ")]
    Parse(String, String),

    #[error(transparent)]
    Polars(#[from] polars::error::PolarsError),

    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    SeqQuery(#[from] sea_query::error::Error),

    #[error("unknown error")]
    Unknown,
}

impl FabrixError {
    pub fn new_common_error<T>(msg: T) -> FabrixError
    where
        T: Into<CommonError>,
    {
        FabrixError::Common(msg.into())
    }

    pub fn new_parse_error<T1, T2>(type1: T1, type2: T2) -> FabrixError
    where
        T1: Display,
        T2: Display,
    {
        FabrixError::Parse(type1.to_string(), type2.to_string())
    }

    pub fn new_parse_info_error<T>(r#type: T, info: &str) -> FabrixError
    where
        T: Display,
    {
        FabrixError::Parse(r#type.to_string(), info.to_string())
    }

    pub fn new_df_dtypes_mismatch_error<'a>(
        d1: DataFrameDTypes<'a>,
        d2: DataFrameDTypes<'a>,
    ) -> FabrixError {
        FabrixError::new_common_error(format!(
            "dataframe dtypes mismatch, d1: {:#?}, d2: {:#?}",
            d1, d2
        ))
    }

    pub fn new_empty_error() -> FabrixError {
        FabrixError::new_common_error("empty content")
    }
}
