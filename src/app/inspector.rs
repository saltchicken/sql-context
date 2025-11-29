use crate::app::models::{ColumnInfo, ForeignKeyInfo, TableData};
use anyhow::Result;
use sqlx::Row;
use sqlx::postgres::PgRow;

// It handles all database interaction.

pub struct Inspector<'a> {
    pool: &'a sqlx::PgPool,
    collect_samples: bool,
    ignore_tables: Vec<String>,
}

impl<'a> Inspector<'a> {

    pub fn new(pool: &'a sqlx::PgPool, collect_samples: bool, ignore_tables: Vec<String>) -> Self {
        Self {
            pool,
            collect_samples,
            ignore_tables,
        }
    }

    pub async fn scan(&self) -> Result<Vec<TableData>> {
        let tables: Vec<(String,)> = sqlx::query_as(
            "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'",
        )
        .fetch_all(self.pool)
        .await?;

        let mut results = Vec::new();

        for (table_name,) in tables {

            if self.ignore_tables.contains(&table_name) {
                continue;
            }

            let columns = self.get_columns(&table_name).await?;
            let primary_keys = self.get_primary_keys(&table_name).await?;
            let foreign_keys = self.get_foreign_keys(&table_name).await?;

            // Check flag before fetching samples
            let sample_rows = if self.collect_samples {
                self.get_sample_data(&table_name, &columns).await?
            } else {
                Vec::new()
            };

            results.push(TableData {
                name: table_name,
                columns,
                primary_keys,
                foreign_keys,
                sample_rows,
            });
        }

        Ok(results)
    }

    async fn get_columns(&self, table_name: &str) -> Result<Vec<ColumnInfo>> {
        sqlx::query_as::<_, ColumnInfo>(
            r#"
            SELECT column_name, data_type, is_nullable, udt_name
            FROM information_schema.columns 
            WHERE table_name = $1 AND table_schema = 'public'
            ORDER BY ordinal_position
            "#,
        )
        .bind(table_name)
        .fetch_all(self.pool)
        .await
        .map_err(|e| e.into())
    }

    async fn get_primary_keys(&self, table_name: &str) -> Result<Vec<String>> {
        let result: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT kcu.column_name
            FROM information_schema.table_constraints tc
            JOIN information_schema.key_column_usage kcu
              ON tc.constraint_name = kcu.constraint_name
              AND tc.table_schema = kcu.table_schema
            WHERE tc.constraint_type = 'PRIMARY KEY' 
              AND tc.table_name = $1
            "#,
        )
        .bind(table_name)
        .fetch_all(self.pool)
        .await?;
        Ok(result.into_iter().map(|(name,)| name).collect())
    }

    async fn get_foreign_keys(&self, table_name: &str) -> Result<Vec<ForeignKeyInfo>> {
        sqlx::query_as::<_, ForeignKeyInfo>(
            r#"
            SELECT
                kcu.column_name,
                ccu.table_name AS foreign_table_name,
                ccu.column_name AS foreign_column_name
            FROM information_schema.key_column_usage AS kcu
            JOIN information_schema.referential_constraints AS rc
                ON kcu.constraint_name = rc.constraint_name
            JOIN information_schema.constraint_column_usage AS ccu
                ON rc.unique_constraint_name = ccu.constraint_name
            WHERE kcu.table_name = $1
            "#,
        )
        .bind(table_name)
        .fetch_all(self.pool)
        .await
        .map_err(|e| e.into())
    }

    async fn get_sample_data(
        &self,
        table_name: &str,
        columns: &[ColumnInfo],
    ) -> Result<Vec<String>> {
        let mut select_parts = Vec::new();
        for col in columns {
            if col.data_type == "bytea" {
                select_parts.push(format!("'[bytea]'::text AS \"{}\"", col.column_name));
            } else if col.udt_name == "vector" {
                select_parts.push(format!("'[vector]'::text AS \"{}\"", col.column_name));
            } else {
                select_parts.push(format!("\"{}\"", col.column_name));
            }
        }

        if select_parts.is_empty() {
            return Ok(vec![]);
        }

        let select_list = select_parts.join(", ");
        let data_query = format!(
            "SELECT row_to_json(t)::text FROM (SELECT {} FROM \"{}\" LIMIT 5) t",
            select_list, table_name
        );

        let rows = sqlx::query(&data_query)
            .map(|row: PgRow| row.get::<String, _>(0))
            .fetch_all(self.pool)
            .await
            .unwrap_or_default();
        Ok(rows)
    }
}