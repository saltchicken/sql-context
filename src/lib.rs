pub mod app;

// Re-export useful types for library users
pub use app::config::AppConfig;
pub use app::formatter::OutputGenerator;
pub use app::generate_report;
pub use app::inspector::Inspector;
pub use app::models::{ColumnInfo, ForeignKeyInfo, TableData};
