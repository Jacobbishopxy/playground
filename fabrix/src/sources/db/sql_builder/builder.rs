//! Sql builder

use polars::prelude::{DataType, Field};
use sea_query::Value as SValue;

use crate::{value, DataFrame, FabrixError, FabrixResult, Series, Value};

#[derive(Debug, Clone)]
pub enum SqlBuilder {
    Mysql,
    Postgres,
    Sqlite,
}

impl std::fmt::Display for SqlBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mysql => write!(f, "mysql"),
            Self::Postgres => write!(f, "postgres"),
            Self::Sqlite => write!(f, "sqlite"),
        }
    }
}

impl From<&str> for SqlBuilder {
    fn from(v: &str) -> Self {
        match &v.to_lowercase()[..] {
            "mysql" | "m" => SqlBuilder::Mysql,
            "postgres" | "p" => SqlBuilder::Postgres,
            _ => SqlBuilder::Sqlite,
        }
    }
}

pub enum IndexType {
    Int,
    BigInt,
    Uuid,
}

impl From<&str> for IndexType {
    fn from(v: &str) -> Self {
        match &v.to_lowercase()[..] {
            "uuid" | "u" => IndexType::Uuid,
            "bigint" | "b" => IndexType::BigInt,
            _ => IndexType::Int,
        }
    }
}

pub struct IndexOption<'a> {
    pub name: &'a str,
    pub index_type: IndexType,
}

impl<'a> IndexOption<'a> {
    pub fn new<T>(name: &'a str, index_type: T) -> Self
    where
        T: Into<IndexType>,
    {
        let index_type: IndexType = index_type.into();
        IndexOption { name, index_type }
    }

    pub fn try_from_series(series: &'a Series) -> FabrixResult<Self> {
        let dtype = series.dtype();
        let index_type = match dtype {
            DataType::UInt8 => Ok(IndexType::Int),
            DataType::UInt16 => Ok(IndexType::Int),
            DataType::UInt32 => Ok(IndexType::Int),
            DataType::UInt64 => Ok(IndexType::BigInt),
            DataType::Int8 => Ok(IndexType::Int),
            DataType::Int16 => Ok(IndexType::Int),
            DataType::Int32 => Ok(IndexType::Int),
            DataType::Int64 => Ok(IndexType::BigInt),
            DataType::Utf8 => Ok(IndexType::Uuid), // TODO: saving uuid as String?
            _ => Err(FabrixError::new_common_error(format!(
                "{:?} cannot convert to index type",
                dtype
            ))),
        }?;

        Ok(IndexOption {
            name: series.name(),
            index_type,
        })
    }
}

pub enum SaveStrategy {
    Replace,
    Append,
    Upsert,
    Fail,
}

/// table field: column name, column type & is nullable
pub struct TableField {
    pub(crate) field: Field,
    pub(crate) nullable: bool,
}

impl TableField {
    pub fn new(field: Field, nullable: bool) -> Self {
        TableField { field, nullable }
    }

    pub fn field(&self) -> &Field {
        &self.field
    }

    pub fn name(&self) -> &String {
        &self.field.name()
    }

    pub fn data_type(&self) -> &DataType {
        &self.field.data_type()
    }

    pub fn nullable(&self) -> bool {
        self.nullable
    }
}

impl From<Field> for TableField {
    fn from(f: Field) -> Self {
        TableField::new(f, true)
    }
}

/// Type conversion: from polars DataType to SeqQuery Value
fn from_data_type_to_null_svalue(dtype: &DataType) -> SValue {
    match dtype {
        DataType::Boolean => SValue::Bool(None),
        DataType::UInt8 => SValue::TinyUnsigned(None),
        DataType::UInt16 => SValue::SmallUnsigned(None),
        DataType::UInt32 => SValue::Unsigned(None),
        DataType::UInt64 => SValue::BigUnsigned(None),
        DataType::Int8 => SValue::TinyInt(None),
        DataType::Int16 => SValue::SmallInt(None),
        DataType::Int32 => SValue::Int(None),
        DataType::Int64 => SValue::BigInt(None),
        DataType::Float32 => SValue::Float(None),
        DataType::Float64 => SValue::Double(None),
        DataType::Utf8 => SValue::String(None),
        DataType::Date32 => todo!(),
        DataType::Date64 => todo!(),
        DataType::Time64(_) => todo!(),
        DataType::List(_) => todo!(),
        DataType::Duration(_) => todo!(),
        DataType::Null => todo!(),
        DataType::Categorical => todo!(),
    }
}

/// Type conversion: from Value to `sea-query` Value
pub(crate) fn try_from_value_to_svalue(
    value: Value,
    dtype: &DataType,
    nullable: bool,
) -> FabrixResult<SValue> {
    match value {
        Value::Bool(v) => Ok(SValue::Bool(Some(v))),
        Value::U8(v) => Ok(SValue::TinyUnsigned(Some(v))),
        Value::U16(v) => Ok(SValue::SmallUnsigned(Some(v))),
        Value::U32(v) => Ok(SValue::Unsigned(Some(v))),
        Value::U64(v) => Ok(SValue::BigUnsigned(Some(v))),
        Value::I8(v) => Ok(SValue::TinyInt(Some(v))),
        Value::I16(v) => Ok(SValue::SmallInt(Some(v))),
        Value::I32(v) => Ok(SValue::Int(Some(v))),
        Value::I64(v) => Ok(SValue::BigInt(Some(v))),
        Value::F32(v) => Ok(SValue::Float(Some(v))),
        Value::F64(v) => Ok(SValue::Double(Some(v))),
        Value::String(v) => Ok(SValue::String(Some(Box::new(v)))),
        Value::Date(_) => todo!(),
        Value::Time(_) => todo!(),
        Value::DateTime(_) => todo!(),
        Value::Null => {
            if nullable {
                Ok(from_data_type_to_null_svalue(dtype))
            } else {
                Err(FabrixError::new_parse_error(value, dtype))
            }
        }
    }
}

/// from `SeaQuery` Value to Value
macro_rules! sv_2_v {
    ($option_value:expr, $nullable:ident) => {
        if $nullable {
            Ok($crate::value!($option_value))
        } else {
            match $option_value {
                Some(v) => Ok($crate::value!(v)),
                None => Err($crate::FabrixError::new_common_error("unsupported type")),
            }
        }
    };
}

/// Type conversion: from `SeaQuery` Value to Value
pub(crate) fn _from_svalue_to_value(svalue: SValue, nullable: bool) -> FabrixResult<Value> {
    match svalue {
        SValue::Bool(ov) => sv_2_v!(ov, nullable),
        SValue::TinyInt(ov) => sv_2_v!(ov, nullable),
        SValue::SmallInt(ov) => sv_2_v!(ov, nullable),
        SValue::Int(ov) => sv_2_v!(ov, nullable),
        SValue::BigInt(ov) => sv_2_v!(ov, nullable),
        SValue::TinyUnsigned(ov) => sv_2_v!(ov, nullable),
        SValue::SmallUnsigned(ov) => sv_2_v!(ov, nullable),
        SValue::Unsigned(ov) => sv_2_v!(ov, nullable),
        SValue::BigUnsigned(ov) => sv_2_v!(ov, nullable),
        SValue::Float(ov) => sv_2_v!(ov, nullable),
        SValue::Double(ov) => sv_2_v!(ov, nullable),
        SValue::String(ov) => match ov {
            Some(v) => Ok(value!(*v)),
            None => Ok(value!(None::<String>)),
        },
        SValue::Date(_) => todo!(),
        SValue::Time(_) => todo!(),
        SValue::DateTime(_) => todo!(),
        SValue::Uuid(ov) => match ov {
            Some(v) => Ok(value!(v.to_string())),
            None => Ok(value!(None::<String>)),
        },
        _ => Err(FabrixError::new_common_error("unsupported type")),
    }
}

// DDL Query
pub trait DdlQuery {
    fn check_table(&self, table_name: &str) -> String;

    fn check_table_schema(&self, table_name: &str) -> String;

    // fn list_tables(&self) -> String;
}

// DDL Mutation
pub trait DdlMutation {
    fn create_table(
        &self,
        table_name: &str,
        columns: &Vec<TableField>,
        index_option: Option<&IndexOption>,
    ) -> String;

    fn delete_table(&self, table_name: &str) -> String;

    // fn alter_table(&self) -> Vec<String>;

    // fn drop_table(&self, table_name: &str) -> String;

    // fn rename_table(&self, from: &str, to: &str) -> String;

    // fn truncate_table(&self, table_name: &str) -> String;

    // fn create_index(&self) -> String;

    // fn drop_index(&self) -> String;

    // fn create_foreign_key(&self) -> String;

    // fn drop_foreign_key(&self) -> String;
}

// DML Query
pub trait DmlQuery {
    fn select_exist_ids(&self, table_name: &str, index: &Series) -> FabrixResult<String>;

    // fn select(&self) -> String;
}

// DML Mutation
pub trait DmlMutation {
    fn insert(&self, table_name: &str, df: DataFrame) -> FabrixResult<String>;

    fn update(
        &self,
        table_name: &str,
        df: DataFrame,
        index_option: &IndexOption,
    ) -> FabrixResult<Vec<String>>;

    fn save(
        &self,
        table_name: &str,
        df: DataFrame,
        save_strategy: &SaveStrategy,
    ) -> FabrixResult<Vec<String>>;
}
