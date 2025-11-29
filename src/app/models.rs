use sqlx::FromRow;


#[derive(FromRow, Debug, Clone)]
pub struct ColumnInfo {
    pub column_name: String,
    pub data_type: String,
    pub is_nullable: String,
    pub udt_name: String,
}

#[derive(FromRow, Debug, Clone)]
pub struct ForeignKeyInfo {
    pub column_name: String,
    pub foreign_table_name: String,
    pub foreign_column_name: String,
}


// This allows separating the "Scanning" phase from the "Formatting" phase.
#[derive(Debug, Clone)]
pub struct TableData {
    pub name: String,
    pub columns: Vec<ColumnInfo>,
    pub primary_keys: Vec<String>,
    pub foreign_keys: Vec<ForeignKeyInfo>,
    pub sample_rows: Vec<String>,
}