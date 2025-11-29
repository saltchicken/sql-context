use anyhow::{Context, Result};
use clap::Parser;
use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::{FromRow, Row};
use std::env;
use std::fmt::Write;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Optional database connection string. If not provided, looks for DB_URL env var.
    #[arg(short, long)]
    db_url: Option<String>,
}

#[derive(FromRow)]
struct ColumnInfo {
    column_name: String,
    data_type: String,
    is_nullable: String,
    udt_name: String,
}

#[derive(FromRow)]
struct ForeignKeyInfo {
    column_name: String,
    foreign_table_name: String,
    foreign_column_name: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file if present
    dotenvy::dotenv().ok();

    let args = Args::parse();

    let db_url = args
        .db_url
        .or_else(|| env::var("DB_URL").ok())
        .context("DB_URL must be set via --db-url or in .env/environment variables")?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .context("Failed to connect to database")?;

    println!(
        "Database Schema for: {}\n",
        db_url.split('/').last().unwrap_or("Unknown")
    );

    // Get all table names
    let tables: Vec<(String,)> = sqlx::query_as(
        "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'",
    )
    .fetch_all(&pool)
    .await?;

    for (table_name,) in tables {
        let mut output = String::new();
        writeln!(output, "## Table: {}", table_name)?;

        let columns = get_columns(&pool, &table_name).await?;

        writeln!(output, "| Column | Type | Nullable |")?;
        writeln!(output, "|---|---|---|")?;

        for col in &columns {
            writeln!(
                output,
                "| {} | {} | {} |",
                col.column_name, col.data_type, col.is_nullable
            )?;
        }

        let pks = get_primary_keys(&pool, &table_name).await?;
        if !pks.is_empty() {
            writeln!(output, "\n**Primary Key:** {}", pks.join(", "))?;
        }

        let fks = get_foreign_keys(&pool, &table_name).await?;
        if !fks.is_empty() {
            writeln!(output, "\n**Foreign Keys:**")?;
            for fk in fks {
                writeln!(
                    output,
                    "- `{}.{}` -> `{}.{}`",
                    table_name, fk.column_name, fk.foreign_table_name, fk.foreign_column_name
                )?;
            }
        }

        // --- Sample Data ---
        let samples = get_sample_data(&pool, &table_name, &columns).await?;

        if !samples.is_empty() {
            writeln!(output, "\n**Sample Data (Top 5 rows):**")?;
            for row in samples {
                writeln!(output, "- `{}`", row)?;
            }
        }

        writeln!(output, "\n---\n")?;
        print!("{}", output);
    }

    Ok(())
}

async fn get_columns(pool: &sqlx::PgPool, table_name: &str) -> Result<Vec<ColumnInfo>> {
    sqlx::query_as::<_, ColumnInfo>(
        r#"
        SELECT column_name, data_type, is_nullable, udt_name
        FROM information_schema.columns 
        WHERE table_name = $1 AND table_schema = 'public'
        ORDER BY ordinal_position
        "#,
    )
    .bind(table_name)
    .fetch_all(pool)
    .await
    .map_err(|e| e.into())
}

async fn get_primary_keys(pool: &sqlx::PgPool, table_name: &str) -> Result<Vec<String>> {
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
    .fetch_all(pool)
    .await?;

    Ok(result.into_iter().map(|(name,)| name).collect())
}

async fn get_foreign_keys(pool: &sqlx::PgPool, table_name: &str) -> Result<Vec<ForeignKeyInfo>> {
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
    .fetch_all(pool)
    .await
    .map_err(|e| e.into())
}

async fn get_sample_data(
    pool: &sqlx::PgPool,
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
        .fetch_all(pool)
        .await
        .unwrap_or_default();

    Ok(rows)
}

