use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use std::fmt::Write;

// Replace with your actual connection string
const DB_URL: &str = "postgresql://saltchicken:password@10.0.0.5:5432/facer_db";

#[tokio::main]
async fn main() -> Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(DB_URL)
        .await?;

    println!(
        "Database Schema for: {}\n",
        DB_URL.split('/').last().unwrap_or("Unknown")
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

        // --- 1. Columns ---
        let columns: Vec<(String, String, String)> = sqlx::query_as(
            r#"
            SELECT column_name, data_type, is_nullable 
            FROM information_schema.columns 
            WHERE table_name = $1 AND table_schema = 'public'
            ORDER BY ordinal_position
            "#,
        )
        .bind(&table_name)
        .fetch_all(&pool)
        .await?;

        writeln!(output, "| Column | Type | Nullable |")?;
        writeln!(output, "|---|---|---|")?;
        for (col_name, data_type, is_nullable) in columns {
            writeln!(output, "| {} | {} | {} |", col_name, data_type, is_nullable)?;
        }

        // --- 2. Primary Keys ---
        let pks: Vec<(String,)> = sqlx::query_as(
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
        .bind(&table_name)
        .fetch_all(&pool)
        .await?;

        if !pks.is_empty() {
            let pk_list: Vec<String> = pks.into_iter().map(|(name,)| name).collect();
            writeln!(output, "\n**Primary Key:** {}", pk_list.join(", "))?;
        }

        // --- 3. Foreign Keys ---
        let fks: Vec<(String, String, String)> = sqlx::query_as(
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
        .bind(&table_name)
        .fetch_all(&pool)
        .await?;

        if !fks.is_empty() {
            writeln!(output, "\n**Foreign Keys:**")?;
            for (col, f_table, f_col) in fks {
                writeln!(
                    output,
                    "- `{}.{}` -> `{}.{}`",
                    table_name, col, f_table, f_col
                )?;
            }
        }

        // --- 4. Sample Data ---
        // We use 'row_to_json' to force Postgres to serialize the dynamic row into a string.
        // We format the SQL query string directly because table names cannot be bound as parameters ($1).
        // Note: We wrap table_name in quotes \"" to handle capitalization or special characters.
        let data_query = format!(
            "SELECT row_to_json(t)::text FROM (SELECT * FROM \"{}\" LIMIT 5) t",
            table_name
        );

        let rows: Vec<(String,)> = sqlx::query_as(&data_query)
            .fetch_all(&pool)
            .await
            .unwrap_or_default(); // If query fails (e.g. permissions), just return empty list

        if !rows.is_empty() {
            writeln!(output, "\n**Sample Data (Top 5 rows):**")?;
            for (json_row,) in rows {
                writeln!(output, "- `{}`", json_row)?;
            }
        }

        writeln!(output, "\n---\n")?;
        print!("{}", output);
    }

    Ok(())
}
