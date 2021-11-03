//! Fabrix util
//!
//! utilities

use crate::{DataFrame, FabrixError, FabrixResult, Series, Value};

type RDF = Result<polars::prelude::DataFrame, polars::error::PolarsError>;

/// From a Result polars' DataFrame and index name, and it will be removed consequently.
pub fn new_df_from_rdf_with_index(df: RDF, index_name: &str) -> FabrixResult<DataFrame> {
    let df = df?;
    let idx = df.column(index_name)?.clone();
    let mut df = df;

    df.drop_in_place(index_name)?;

    Ok(DataFrame::new(df, Series::from_polars_series(idx)))
}

/// From a Result polars' DataFrame, auto generate index
pub fn new_df_from_rdf_default_index(df: RDF) -> FabrixResult<DataFrame> {
    let df = df?;
    let h = df.height() as u64;

    let index = Series::from_integer(&h)?;

    Ok(DataFrame::new(df, index))
}

/// From a Result polars' DataFrame and Series
pub fn new_df_from_rdf_and_series(df: RDF, series: Series) -> FabrixResult<DataFrame> {
    let df = df?;

    Ok(DataFrame::new(df, series))
}

/// Used for counting iteration and determining when to stop yielding
pub struct Stepper {
    pub(crate) len: usize,
    pub(crate) step: usize,
}

impl Stepper {
    pub fn new(len: usize) -> Self {
        Stepper { len, step: 0 }
    }

    pub fn exhausted(&self) -> bool {
        if self.len == self.step {
            true
        } else {
            false
        }
    }

    pub fn forward(&mut self) {
        self.step += 1;
    }
}

/// a general naming for a default FDataFrame index
pub const IDX: &'static str = "index";

/// out of boundary error
pub(crate) fn oob_err(length: usize, len: usize) -> FabrixError {
    FabrixError::new_common_error(format!("length {:?} out of len {:?} boundary", length, len))
}

/// index not found error
pub(crate) fn inf_err<'a>(index: &Value) -> FabrixError {
    FabrixError::new_common_error(format!("index {:?} not found", index))
}

/// content empty error
pub(crate) fn cis_err(name: &str) -> FabrixError {
    FabrixError::new_common_error(format!("{:?} is empty", name))
}
