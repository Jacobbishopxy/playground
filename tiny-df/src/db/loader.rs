//! tiny-df sql engine
//!
//! Similar to Python's pandas dataframe: `pd.Dataframe.to_sql`, `pd.Dataframe.read_sql` and etc.

use async_trait::async_trait;
use sqlx::mysql::MySqlRow;
use sqlx::postgres::PgRow;
use sqlx::sqlite::SqliteRow;
use sqlx::{MySqlPool, PgPool, Row, SqlitePool};

use super::types::*;
use crate::db::{ConnInfo, TdDbError, TdDbResult};
use crate::prelude::*;
use crate::se::{IndexOption, SaveOption, Sql};

/// Loader's engine
/// Engine is a trait that describes functionalities interacting with database
///
/// provided methods:
/// 1. get_table_schema
/// 1. raw_fetch
/// 1. fetch TODO: selection, condition & pagination
/// 1. create_table
/// 1. insert
/// 1. update TODO: id column must be specified
/// 1. upsert TODO: id column must be specified
/// 1. save TODO: transaction for upsert saving strategy
/// 1. ...
#[async_trait]
pub trait Engine<DF, COL> {
    async fn get_table_schema(&self, table: &str) -> TdDbResult<Vec<COL>>;

    /// fetch all data by a query string, and turn result into a `Dataframe` (strict mode)
    async fn raw_fetch(&self, query: &str) -> TdDbResult<Option<DF>>;

    // async fn fetch(&self,) -> TdDbResult<Option<DF>>;

    /// create a table by a dataframe's columns
    async fn create_table(
        &self,
        table_name: &str,
        columns: Vec<DataframeColumn>,
        index_option: Option<&IndexOption>,
    ) -> TdDbResult<u64>;

    /// insert a `Dataframe` to an existing table
    async fn insert(
        &self,
        table_name: &str,
        dataframe: Dataframe,
        index_option: Option<&IndexOption>,
    ) -> TdDbResult<u64>;

    async fn update(
        &self,
        table_name: &str,
        dataframe: Dataframe,
        index_option: &IndexOption,
    ) -> TdDbResult<u64>;

    // async fn upsert(&self, dataframe: Dataframe) -> TdDbResult<()>;

    /// the most useful and common writing method to a database (transaction is used)
    async fn save(
        &self,
        table_name: &str,
        dataframe: Dataframe,
        save_option: &SaveOption,
    ) -> TdDbResult<u64>;
}

#[async_trait]
impl Engine<Dataframe, DataframeColumn> for MySqlPool {
    async fn get_table_schema(&self, table: &str) -> TdDbResult<Vec<DataframeColumn>> {
        // query string for Mysql
        let query = Sql::Mysql.check_table_schema(table);

        let schema = sqlx::query(&query)
            .map(|row: MySqlRow| -> DataframeColumn {
                // get column name & type
                let name: String = row.get(0);
                let data_type: String = row.get(1);
                let data_type = SqlColumnType::new(&data_type, "m").to_datatype();

                DataframeColumn::new(name, data_type)
            })
            .fetch_all(self)
            .await?;

        Ok(schema)
    }

    async fn raw_fetch(&self, query: &str) -> TdDbResult<Option<Dataframe>> {
        let mut columns = vec![];
        let mut should_update_col = true;

        // `Vec<RowVec>`
        let mut d2: D2 = sqlx::query(query)
            .try_map(|row: MySqlRow| {
                if should_update_col {
                    columns = row_cols_name_mysql(&row);
                    should_update_col = false;
                }
                row_to_d1_mysql(row)
            })
            .fetch_all(self)
            .await?;

        d2.insert(0, columns);

        Ok(Some(Dataframe::from_vec(d2, "h")))
    }

    async fn create_table(
        &self,
        table_name: &str,
        columns: Vec<DataframeColumn>,
        index_option: Option<&IndexOption>,
    ) -> TdDbResult<u64> {
        // query string for Mysql
        let query = Sql::Mysql.create_table(table_name, &columns, index_option);

        let res = sqlx::query(&query).execute(self).await?.rows_affected();

        Ok(res)
    }

    async fn insert(
        &self,
        table_name: &str,
        dataframe: Dataframe,
        index_option: Option<&IndexOption>,
    ) -> TdDbResult<u64> {
        // query string for Mysql
        let query = Sql::Mysql.insert(table_name, dataframe, index_option);

        let res = sqlx::query(&query).execute(self).await?.rows_affected();

        Ok(res)
    }

    async fn update(
        &self,
        table_name: &str,
        dataframe: Dataframe,
        index_option: &IndexOption,
    ) -> TdDbResult<u64> {
        // query strings for Mysql
        let queries = Sql::Mysql.update(table_name, dataframe, index_option);

        let mut transaction = self.begin().await?;
        let mut affected_rows = 0u64;

        for que in queries.iter() {
            affected_rows += sqlx::query(que)
                .execute(&mut transaction)
                .await?
                .rows_affected();
        }

        transaction.commit().await?;

        Ok(affected_rows)
    }

    async fn save(
        &self,
        table_name: &str,
        dataframe: Dataframe,
        save_option: &SaveOption,
    ) -> TdDbResult<u64> {
        todo!()
    }
}

#[async_trait]
impl Engine<Dataframe, DataframeColumn> for PgPool {
    async fn get_table_schema(&self, table: &str) -> TdDbResult<Vec<DataframeColumn>> {
        // query string for Postgres
        let query = Sql::Postgres.check_table_schema(table);

        let schema = sqlx::query(&query)
            .map(|row: PgRow| -> DataframeColumn {
                // get column name & type
                let name: String = row.get(0);
                let data_type: String = row.get(1);
                let data_type = SqlColumnType::new(&data_type, "p").to_datatype();

                DataframeColumn::new(name, data_type)
            })
            .fetch_all(self)
            .await?;

        Ok(schema)
    }

    async fn raw_fetch(&self, query: &str) -> TdDbResult<Option<Dataframe>> {
        let mut columns = vec![];
        let mut should_update_col = true;

        let mut d2 = sqlx::query(query)
            .try_map(|row: PgRow| {
                if should_update_col {
                    columns = row_cols_name_pg(&row);
                    should_update_col = false;
                }
                row_to_d1_pg(row)
            })
            .fetch_all(self)
            .await?;

        d2.insert(0, columns);

        Ok(Some(Dataframe::from_vec(d2, "h")))
    }

    async fn create_table(
        &self,
        table_name: &str,
        columns: Vec<DataframeColumn>,
        index_option: Option<&IndexOption>,
    ) -> TdDbResult<u64> {
        // query string for Postgres
        let query = Sql::Postgres.create_table(table_name, &columns, index_option);

        let res = sqlx::query(&query).execute(self).await?.rows_affected();

        Ok(res)
    }

    async fn insert(
        &self,
        table_name: &str,
        dataframe: Dataframe,
        index_option: Option<&IndexOption>,
    ) -> TdDbResult<u64> {
        // query string for Postgres
        let query = Sql::Postgres.insert(table_name, dataframe, index_option);

        let res = sqlx::query(&query).execute(self).await?.rows_affected();

        Ok(res)
    }

    async fn update(
        &self,
        table_name: &str,
        dataframe: Dataframe,
        index_option: &IndexOption,
    ) -> TdDbResult<u64> {
        // query strings for Postgres
        let queries = Sql::Postgres.update(table_name, dataframe, index_option);

        let mut transaction = self.begin().await?;
        let mut affected_rows = 0u64;

        for que in queries.iter() {
            affected_rows += sqlx::query(que)
                .execute(&mut transaction)
                .await?
                .rows_affected();
        }

        transaction.commit().await?;

        Ok(affected_rows)
    }

    async fn save(
        &self,
        table_name: &str,
        dataframe: Dataframe,
        save_option: &SaveOption,
    ) -> TdDbResult<u64> {
        todo!()
    }
}

#[async_trait]
impl Engine<Dataframe, DataframeColumn> for SqlitePool {
    async fn get_table_schema(&self, table: &str) -> TdDbResult<Vec<DataframeColumn>> {
        // get query string for sqlite
        let query = Sql::Sqlite.check_table_schema(table);

        let schema = sqlx::query(&query)
            .map(|row: SqliteRow| -> DataframeColumn {
                // get column name & type
                let name: String = row.get(0);
                let data_type: String = row.get(1);
                let data_type = SqlColumnType::new(&data_type, "s").to_datatype();

                DataframeColumn::new(name, data_type)
            })
            .fetch_all(self)
            .await?;

        Ok(schema)
    }

    async fn raw_fetch(&self, query: &str) -> TdDbResult<Option<Dataframe>> {
        let mut columns = vec![];
        let mut should_update_col = true;

        let mut d2 = sqlx::query(query)
            .try_map(|row: SqliteRow| {
                if should_update_col {
                    columns = row_cols_name_sqlite(&row);
                    should_update_col = false;
                }
                row_to_d1_sqlite(row)
            })
            .fetch_all(self)
            .await?;

        d2.insert(0, columns);

        Ok(Some(Dataframe::from_vec(d2, "h")))
    }

    async fn create_table(
        &self,
        table_name: &str,
        columns: Vec<DataframeColumn>,
        index_option: Option<&IndexOption>,
    ) -> TdDbResult<u64> {
        // query string for Sqlite
        let query = Sql::Sqlite.create_table(table_name, &columns, index_option);

        let res = sqlx::query(&query).execute(self).await?.rows_affected();

        Ok(res)
    }

    async fn insert(
        &self,
        table_name: &str,
        dataframe: Dataframe,
        index_option: Option<&IndexOption>,
    ) -> TdDbResult<u64> {
        // query string for sqlite
        let query = Sql::Sqlite.insert(table_name, dataframe, index_option);

        let res = sqlx::query(&query).execute(self).await?.rows_affected();

        Ok(res)
    }

    async fn update(
        &self,
        table_name: &str,
        dataframe: Dataframe,
        index_option: &IndexOption,
    ) -> TdDbResult<u64> {
        // query strings for Sqlite
        let queries = Sql::Sqlite.update(table_name, dataframe, index_option);

        let mut transaction = self.begin().await?;
        let mut affected_rows = 0u64;

        for que in queries.iter() {
            affected_rows += sqlx::query(que)
                .execute(&mut transaction)
                .await?
                .rows_affected();
        }

        transaction.commit().await?;

        Ok(affected_rows)
    }

    async fn save(
        &self,
        table_name: &str,
        dataframe: Dataframe,
        save_option: &SaveOption,
    ) -> TdDbResult<u64> {
        todo!()
    }
}

pub struct Loader {
    driver: Sql,
    conn: String,
    pool: Option<Box<dyn Engine<Dataframe, DataframeColumn>>>,
}

const DB_COMMON_ERROR: TdDbError = TdDbError::Common("Loader pool not set");

// TODO: transaction functionality
impl Loader {
    /// create a loader from `ConnInfo`
    pub fn new(conn_info: ConnInfo) -> Self {
        Loader {
            driver: conn_info.driver.clone(),
            conn: conn_info.to_string(),
            pool: None,
        }
    }

    /// create a loader from `&str`
    pub fn from_str(conn_str: &str) -> Self {
        let mut s = conn_str.split(":");
        let driver = match s.next() {
            Some(v) => v.into(),
            None => Sql::Sqlite,
        };
        Loader {
            driver,
            conn: conn_str.to_string(),
            pool: None,
        }
    }

    /// manual establish connection pool
    pub async fn connect(&mut self) -> TdDbResult<()> {
        match self.driver {
            Sql::Mysql => match MySqlPool::connect(&self.conn).await {
                Ok(op) => {
                    self.pool = Some(Box::new(op));
                    Ok(())
                }
                Err(e) => Err(e.into()),
            },
            Sql::Postgres => match PgPool::connect(&self.conn).await {
                Ok(op) => {
                    self.pool = Some(Box::new(op));
                    Ok(())
                }
                Err(e) => Err(e.into()),
            },
            Sql::Sqlite => match SqlitePool::connect(&self.conn).await {
                Ok(op) => {
                    self.pool = Some(Box::new(op));
                    Ok(())
                }
                Err(e) => Err(e.into()),
            },
        }
    }

    /// get a table's schema
    pub async fn get_table_schema(&self, table: &str) -> TdDbResult<Vec<DataframeColumn>> {
        match &self.pool {
            Some(p) => Ok(p.get_table_schema(table).await?),
            None => Err(DB_COMMON_ERROR),
        }
    }

    /// fetch all data
    pub async fn raw_fetch(&self, query: &str) -> TdDbResult<Option<Dataframe>> {
        match &self.pool {
            Some(p) => Ok(p.raw_fetch(query).await?),
            None => Err(DB_COMMON_ERROR),
        }
    }

    /// create a table by a dataframe column
    pub async fn create_table<'a>(
        &self,
        table_name: &str,
        columns: Vec<DataframeColumn>,
        index_option: Option<&IndexOption<'a>>,
    ) -> TdDbResult<u64> {
        match &self.pool {
            Some(p) => Ok(p.create_table(table_name, columns, index_option).await?),
            None => Err(DB_COMMON_ERROR),
        }
    }

    /// insert a dataframe to an existing table
    pub async fn insert<'a>(
        &self,
        table_name: &str,
        dataframe: Dataframe,
        index_option: Option<&IndexOption<'a>>,
    ) -> TdDbResult<u64> {
        match &self.pool {
            Some(p) => Ok(p.insert(table_name, dataframe, index_option).await?),
            None => Err(DB_COMMON_ERROR),
        }
    }
}

#[cfg(test)]
mod test_loader {

    use super::*;
    use crate::df;
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

    const CONN1: &'static str = "mysql://root:secret@localhost:3306/dev";
    const CONN2: &'static str = "postgres://root:secret@localhost:5432/dev";
    const CONN3: &'static str = "sqlite:cache/dev.sqlite";

    #[test]
    fn test_new() {
        let loader1 = Loader::from_str(CONN1);
        println!("{:?}", loader1.conn);

        let conn_info = ConnInfo::new(Sql::Mysql, "root", "secret", "localhost", 3306, "dev");
        let loader2 = Loader::new(conn_info);
        println!("{:?}", loader2.conn);

        assert_eq!(loader1.conn, loader2.conn);
    }

    // ####################################################################################################
    // test connection & raw_fetch
    // ####################################################################################################

    #[tokio::test]
    async fn test_connection_mysql() {
        let mut loader = Loader::from_str(CONN1);
        loader.connect().await.unwrap();

        let df = loader.raw_fetch("select * from dev limit 1").await.unwrap();

        println!("{:#?}", df);
    }

    #[tokio::test]
    async fn test_connection_pg() {
        let mut loader = Loader::from_str(CONN2);
        loader.connect().await.unwrap();

        let df = loader.raw_fetch("select * from dev limit 1").await.unwrap();

        println!("{:#?}", df);
    }

    #[tokio::test]
    async fn test_connection_sqlite() {
        let mut loader = Loader::from_str(CONN3);
        loader.connect().await.unwrap();

        let df = loader.raw_fetch("select * from dev limit 1").await.unwrap();

        println!("{:#?}", df);
    }

    // ####################################################################################################
    // get table schema
    // ####################################################################################################

    #[tokio::test]
    async fn test_get_table_schema_mysql() {
        let mut loader = Loader::from_str(CONN1);
        loader.connect().await.unwrap();

        let scm = loader.get_table_schema("dev").await.unwrap();

        println!("{:#?}", scm);
    }

    #[tokio::test]
    async fn test_get_table_schema_pg() {
        let mut loader = Loader::from_str(CONN2);
        loader.connect().await.unwrap();

        let scm = loader.get_table_schema("dev").await.unwrap();

        println!("{:#?}", scm);
    }

    #[tokio::test]
    async fn test_get_table_schema_sqlite() {
        let mut loader = Loader::from_str(CONN3);
        loader.connect().await.unwrap();

        let scm = loader.get_table_schema("dev").await.unwrap();

        println!("{:#?}", scm);
    }

    // ####################################################################################################
    // create table
    // ####################################################################################################

    #[tokio::test]
    async fn test_create_table_mysql() {
        let mut loader = Loader::from_str(CONN1);
        loader.connect().await.unwrap();

        let cols = vec![
            DataframeColumn::new("id", DataType::Id),
            DataframeColumn::new("name", DataType::String),
            DataframeColumn::new("vol", DataType::Float),
            DataframeColumn::new("created_at", DataType::DateTime),
        ];

        let foo = loader.create_table("dev", cols, None).await.unwrap();

        println!("{:?}", foo);
    }

    #[tokio::test]
    async fn test_create_table_pg() {
        let mut loader = Loader::from_str(CONN2);
        loader.connect().await.unwrap();

        let cols = vec![
            DataframeColumn::new("id", DataType::Id),
            DataframeColumn::new("name", DataType::String),
            DataframeColumn::new("vol", DataType::Float),
            DataframeColumn::new("created_at", DataType::DateTime),
        ];

        let foo = loader.create_table("dev", cols, None).await.unwrap();

        println!("{:?}", foo);
    }

    #[tokio::test]
    async fn test_create_table_sqlite() {
        let mut loader = Loader::from_str(CONN3);
        loader.connect().await.unwrap();

        let cols = vec![
            DataframeColumn::new("id", DataType::Id),
            DataframeColumn::new("name", DataType::String),
            DataframeColumn::new("vol", DataType::Float),
            DataframeColumn::new("created_at", DataType::DateTime),
        ];

        let foo = loader.create_table("dev", cols, None).await.unwrap();

        println!("{:?}", foo);
    }

    // ####################################################################################################
    // insert a dataframe into a table
    // ####################################################################################################

    #[tokio::test]
    async fn test_insert_mysql() {
        let mut loader = Loader::from_str(CONN1);
        loader.connect().await.unwrap();

        let df = df![
            "h";
            "id" => [
                DataframeData::Id(0),
                DataframeData::Id(1),
                DataframeData::Id(2),
                DataframeData::Id(3),
            ],
            "name" => [
                "Jacob",
                "Sam",
                "MZ",
                "Jw"
            ],
            "vol" => [
                10,
                12,
                11,
                10
            ],
            "created_at" => [
                NaiveDateTime::new(
                    NaiveDate::from_ymd(2010,1,1),
                    NaiveTime::from_hms(1, 10, 0),
                ),
                NaiveDateTime::new(
                    NaiveDate::from_ymd(2011,1,1),
                    NaiveTime::from_hms(1, 10, 0),
                ),
                NaiveDateTime::new(
                    NaiveDate::from_ymd(2014,1,1),
                    NaiveTime::from_hms(1, 10, 0),
                ),
                NaiveDateTime::new(
                    NaiveDate::from_ymd(2015,1,1),
                    NaiveTime::from_hms(1, 10, 0),
                ),
            ]
        ];

        let foo = loader.insert("dev", df, None).await.unwrap();
        println!("{:?}", foo);
    }

    #[tokio::test]
    async fn test_insert_pg() {
        let mut loader = Loader::from_str(CONN2);
        loader.connect().await.unwrap();

        let df = df![
            "h";
            "id" => [
                DataframeData::Id(0),
                DataframeData::Id(1),
                DataframeData::Id(2),
                DataframeData::Id(3),
            ],
            "name" => [
                "Jacob",
                "Sam",
                "MZ",
                "Jw"
            ],
            "vol" => [
                10,
                12,
                11,
                10
            ],
            "created_at" => [
                NaiveDateTime::new(
                    NaiveDate::from_ymd(2010,1,1),
                    NaiveTime::from_hms(1, 10, 0),
                ),
                NaiveDateTime::new(
                    NaiveDate::from_ymd(2011,1,1),
                    NaiveTime::from_hms(1, 10, 0),
                ),
                NaiveDateTime::new(
                    NaiveDate::from_ymd(2014,1,1),
                    NaiveTime::from_hms(1, 10, 0),
                ),
                NaiveDateTime::new(
                    NaiveDate::from_ymd(2015,1,1),
                    NaiveTime::from_hms(1, 10, 0),
                ),
            ]
        ];

        let foo = loader.insert("dev", df, None).await.unwrap();
        println!("{:?}", foo);
    }

    #[tokio::test]
    async fn test_insert_sqlite() {
        let mut loader = Loader::from_str(CONN3);
        loader.connect().await.unwrap();

        let df = df![
            "h";
            "id" => [
                DataframeData::Id(0),
                DataframeData::Id(1),
                DataframeData::Id(2),
                DataframeData::Id(3),
            ],
            "name" => [
                "Jacob",
                "Sam",
                "MZ",
                "Jw"
            ],
            "vol" => [
                10,
                12,
                11,
                10
            ],
            "created_at" => [
                NaiveDateTime::new(
                    NaiveDate::from_ymd(2010,1,1),
                    NaiveTime::from_hms(1, 10, 0),
                ),
                NaiveDateTime::new(
                    NaiveDate::from_ymd(2011,1,1),
                    NaiveTime::from_hms(1, 10, 0),
                ),
                NaiveDateTime::new(
                    NaiveDate::from_ymd(2014,1,1),
                    NaiveTime::from_hms(1, 10, 0),
                ),
                NaiveDateTime::new(
                    NaiveDate::from_ymd(2015,1,1),
                    NaiveTime::from_hms(1, 10, 0),
                ),
            ]
        ];

        let foo = loader.insert("dev", df, None).await.unwrap();
        println!("{:?}", foo);
    }
}
