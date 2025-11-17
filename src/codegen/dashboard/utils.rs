/// Utility functions and types for dashboard code generation.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Database type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseType {
    PostgreSQL,
    MySQL,
    MariaDB,
}

impl DatabaseType {
    /// Get the database type as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            DatabaseType::PostgreSQL => "postgresql",
            DatabaseType::MySQL => "mysql",
            DatabaseType::MariaDB => "mariadb",
        }
    }

    /// Check if this is MySQL or MariaDB (similar syntax)
    pub fn is_mysql_like(&self) -> bool {
        matches!(self, DatabaseType::MySQL | DatabaseType::MariaDB)
    }

    /// Detect database type from DATABASE_URL
    ///
    /// # Examples
    /// ```no_run
    /// use nomnom::codegen::dashboard::DatabaseType;
    ///
    /// let db_type = DatabaseType::from_url("postgres://localhost/mydb");
    /// assert_eq!(db_type, DatabaseType::PostgreSQL);
    ///
    /// let db_type = DatabaseType::from_url("mysql://localhost/mydb");
    /// assert_eq!(db_type, DatabaseType::MySQL);
    /// ```
    pub fn from_url(url: &str) -> DatabaseType {
        if url.starts_with("postgres://") || url.starts_with("postgresql://") {
            DatabaseType::PostgreSQL
        } else if url.starts_with("mysql://") {
            DatabaseType::MySQL
        } else {
            // Default to PostgreSQL for backward compatibility
            DatabaseType::PostgreSQL
        }
    }
}

/// Dashboard configuration
#[derive(Debug, Clone)]
pub struct DashboardConfig {
    pub polling_interval_ms: u32,
    pub max_events_per_poll: u32,
    pub frontend_port: u16,
    pub backend_port: u16,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        DashboardConfig {
            polling_interval_ms: 500,
            max_events_per_poll: 100,
            frontend_port: 5173,
            backend_port: 8000,
        }
    }
}

/// Entity display configuration for the dashboard
#[derive(Debug, Clone)]
pub struct EntityDisplayConfig {
    pub name: String,
    pub table: String,
    pub color: String,
    pub icon: String,
    pub display_fields: Vec<String>,
    pub max_records: usize,
}

/// Generate a consistent color for an entity based on its name hash
pub fn entity_color(name: &str) -> String {
    let colors = [
        "#3b82f6", // blue
        "#10b981", // green
        "#f59e0b", // amber
        "#ef4444", // red
        "#8b5cf6", // violet
        "#ec4899", // pink
        "#14b8a6", // teal
        "#f97316", // orange
    ];

    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    let hash = hasher.finish();

    colors[(hash % colors.len() as u64) as usize].to_string()
}

/// Assign an icon to an entity based on name patterns
pub fn entity_icon(name: &str) -> &'static str {
    let name_lower = name.to_lowercase();

    // Pattern matching for common entity types
    // Check more specific patterns first to avoid false matches
    if name_lower.contains("line") {
        // Line items should be checked before "order" or "item"
        "📄"
    } else if name_lower.contains("order") {
        "📦"
    } else if name_lower.contains("customer") || name_lower.contains("user") {
        "👤"
    } else if name_lower.contains("product") || name_lower.contains("item") {
        "📦"
    } else if name_lower.contains("payment") || name_lower.contains("transaction") {
        "💳"
    } else if name_lower.contains("invoice") {
        "🧾"
    } else if name_lower.contains("shipment") || name_lower.contains("delivery") {
        "🚚"
    } else if name_lower.contains("address") || name_lower.contains("location") {
        "📍"
    } else if name_lower.contains("contact") {
        "📞"
    } else if name_lower.contains("message") || name_lower.contains("notification") {
        "✉️"
    } else {
        // Default icon
        "📊"
    }
}

/// Select the first N display fields from entity field overrides
pub fn select_display_fields(
    entity: &crate::codegen::EntityDef,
    max_fields: usize,
) -> Vec<String> {
    if let Some(ref persistence) = entity.persistence {
        // Get fields from field_overrides
        persistence.field_overrides
            .iter()
            .take(max_fields)
            .map(|f| f.name.clone())
            .collect()
    } else {
        // Fall back to entity fields
        entity.fields
            .iter()
            .filter(|f| !f.primary_key) // Skip auto-generated PKs
            .take(max_fields)
            .map(|f| f.name.clone())
            .collect()
    }
}

/// Convert PascalCase to snake_case
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_is_lower = false;

    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 && prev_is_lower {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap());
            prev_is_lower = false;
        } else {
            result.push(ch);
            prev_is_lower = true;
        }
    }

    result
}

/// Generate entity display configuration
pub fn generate_entity_display_config(
    entity: &crate::codegen::EntityDef,
    all_entities: &[crate::codegen::EntityDef],
) -> EntityDisplayConfig {
    let table = if let Some(db_config) = entity.get_database_config(all_entities) {
        db_config.conformant_table.clone()
    } else {
        to_snake_case(&entity.name)
    };

    EntityDisplayConfig {
        name: entity.name.clone(),
        table,
        color: entity_color(&entity.name),
        icon: entity_icon(&entity.name).to_string(),
        display_fields: select_display_fields(entity, 5), // First 5 fields
        max_records: 500, // Default cap
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_color_consistency() {
        // Same name should always produce same color
        let color1 = entity_color("Order");
        let color2 = entity_color("Order");
        assert_eq!(color1, color2);
    }

    #[test]
    fn test_entity_icon_patterns() {
        assert_eq!(entity_icon("Order"), "📦");
        assert_eq!(entity_icon("OrderLineItem"), "📄");
        assert_eq!(entity_icon("Customer"), "👤");
        assert_eq!(entity_icon("Payment"), "💳");
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("OrderLineItem"), "order_line_item");
        assert_eq!(to_snake_case("Customer"), "customer");
        assert_eq!(to_snake_case("ProductCategory"), "product_category");
    }

    #[test]
    fn test_database_type_is_mysql_like() {
        assert!(!DatabaseType::PostgreSQL.is_mysql_like());
        assert!(DatabaseType::MySQL.is_mysql_like());
        assert!(DatabaseType::MariaDB.is_mysql_like());
    }
}
