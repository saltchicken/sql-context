use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use std::fmt::Write; // Allows us to write to a String buffer

// Replace with your actual connection string
const DB_URL: &str = "postgresql://saltchicken:password@10.0.0.5:5432/stock";

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Create a connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(DB_URL)
        .await?;

    println!(
        "Database Schema for: {}\n",
        DB_URL.split('/').last().unwrap_or("Unknown")
    );

    // 2. Get all table names from the public schema
    let tables: Vec<(String,)> = sqlx::query_as(
        "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'",
    )
    .fetch_all(&pool)
    .await?;

    for (table_name,) in tables {
        let mut output = String::new();
        writeln!(output, "## Table: {}", table_name)?;

        // 3. Get Columns
        // We fetch column name, data type, and is_nullable
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

        // 4. Get Primary Keys
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

        // 5. Get Foreign Keys (Crucial for context)
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

        writeln!(output, "\n---\n")?;

        // Print the block for this table
        print!("{}", output);
    }

    Ok(())
}
