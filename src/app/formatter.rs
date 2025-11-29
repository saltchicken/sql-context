use crate::app::models::TableData;
use std::fmt::Write;


pub struct OutputGenerator;

impl OutputGenerator {
    pub fn generate_markdown(
        db_name: &str,
        tables: &[TableData],
    ) -> Result<String, std::fmt::Error> {
        let mut output = String::new();

        writeln!(output, "Database Schema for: {}\n", db_name)?;

        for table in tables {
            writeln!(output, "## Table: {}", table.name)?;

            writeln!(output, "| Column | Type | Nullable |")?;
            writeln!(output, "|---|---|---|")?;
            for col in &table.columns {
                writeln!(
                    output,
                    "| {} | {} | {} |",
                    col.column_name, col.data_type, col.is_nullable
                )?;
            }

            if !table.primary_keys.is_empty() {
                writeln!(
                    output,
                    "\n**Primary Key:** {}",
                    table.primary_keys.join(", ")
                )?;
            }

            if !table.foreign_keys.is_empty() {
                writeln!(output, "\n**Foreign Keys:**")?;
                for fk in &table.foreign_keys {
                    writeln!(
                        output,
                        "- `{}.{}` -> `{}.{}`",
                        table.name, fk.column_name, fk.foreign_table_name, fk.foreign_column_name
                    )?;
                }
            }

            if !table.sample_rows.is_empty() {
                writeln!(output, "\n**Sample Data (Top 5 rows):**")?;
                for row in &table.sample_rows {
                    writeln!(output, "- `{}`", row)?;
                }
            }

            writeln!(output, "\n---\n")?;
        }

        Ok(output)
    }
}