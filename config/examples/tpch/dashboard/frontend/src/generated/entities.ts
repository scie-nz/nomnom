// Auto-generated entity configuration
// This file is regenerated when entities change

export interface Entity {
  name: string;
  table: string;
  color: string;
  icon: string;
  fields: string[];
  maxRecords: number;
}

export const ENTITIES: Entity[] = [
  {
    name: "OrderLineItem",
    table: "order_line_items",
    color: "#10b981",
    icon: "📦",
    fields: ["order_key", "line_number", "part_key", "supplier_key", "quantity"],
    maxRecords: 500,
  },
  {
    name: "Order",
    table: "orders",
    color: "#10b981",
    icon: "📦",
    fields: ["order_key", "customer_key", "order_status", "total_price", "order_date"],
    maxRecords: 500,
  },
];
