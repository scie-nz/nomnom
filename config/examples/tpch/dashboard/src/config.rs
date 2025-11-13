// Auto-generated entity configuration

#[derive(Debug, Clone)]
pub struct EntityConfig {
    pub name: &'static str,
    pub table: &'static str,
    pub color: &'static str,
    pub icon: &'static str,
    pub fields: &'static [&'static str],
    pub max_records: usize,
}

/// All entity configurations
pub const ENTITIES: &[EntityConfig] = &[
    EntityConfig {
        name: "OrderLineItem",
        table: "order_line_items",
        color: "#10b981",
        icon: "📦",
        fields: &["order_key", "line_number", "part_key", "supplier_key", "quantity"],
        max_records: 500,
    },
    EntityConfig {
        name: "Order",
        table: "orders",
        color: "#10b981",
        icon: "📦",
        fields: &["order_key", "customer_key", "order_status", "total_price", "order_date"],
        max_records: 500,
    },
];
