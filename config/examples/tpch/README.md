# TPC-H Style E-commerce Example

This example demonstrates how to use nomnom to build a complete order processing system inspired by the TPC-H benchmark. It showcases entity definitions, database persistence, and parent-child relationships.

## Overview

This configuration tracks:
- **Customers** - Users who place orders
- **Products** - Items that can be ordered
- **Orders** - Customer orders with metadata
- **OrderLineItems** - Individual line items within orders (derived entity)

## Architecture

```
Customer (root entity)
Product (root entity)
Order (root entity)
  └─ OrderLineItem (derived entity, repeated_for: line_items)
```

## Entity Definitions

### Customer (`entities/customer.yaml`)

Represents a customer who can place orders.

**Key fields:**
- `customer_key` - Unique customer identifier
- `name` - Customer name
- `account_balance` - Current account balance
- `market_segment` - Market segment (BUILDING, AUTOMOBILE, FURNITURE, etc.)

**Database:** Persisted to `customers` table with auto-generated ID

### Product (`entities/product.yaml`)

Represents a product/part that can be ordered.

**Key fields:**
- `part_key` - Unique product identifier
- `name` - Product name
- `retail_price` - Retail price
- `manufacturer`, `brand`, `product_type` - Product metadata

**Database:** Persisted to `products` table with auto-generated ID

### Order (`entities/order.yaml`)

Represents a customer order containing line items.

**Key fields:**
- `order_key` - Unique order identifier
- `customer_key` - Foreign key to customer
- `order_status` - Status (O=open, F=fulfilled, P=pending)
- `total_price` - Total order price
- `order_date` - Date order was placed
- `line_item_count` - Computed field counting child line items

**Database:** Persisted to `orders` table with auto-generated ID

### OrderLineItem (`entities/orderlineitem.yaml`)

Represents a single line item within an order. This is a **derived entity** that extracts line items from the parent Order's `line_items` array.

**Key fields:**
- `order_key` - Copied from parent order
- `line_number` - Line item sequence number
- `part_key` - Foreign key to product
- `quantity` - Quantity ordered
- `extended_price` - Line item total price
- `discount`, `tax` - Pricing modifiers

**Database:** Persisted to `order_line_items` table with composite unique key on (order_key, line_number)

## Usage

### 1. Generate Code

Generate Rust entities, database models, and Python bindings:

```bash
nomnom build-from-config --config config/examples/tpch/nomnom.yaml
```

This generates:
- `src/generated.rs` - Rust entity structs
- `src/models/mod.rs` - Diesel ORM models
- `src/db/generated_operations.rs` - Database operations (get_or_create)
- `src/schema.rs` - Diesel schema definitions
- `src/generated_bindings.rs` - PyO3 Python bindings

### 2. Set Up Database

Create the database and run migrations:

```bash
# Set database connection
export DATABASE_URL="mysql://user:password@localhost/tpch_example"

# Create database
mysql -u user -p -e "CREATE DATABASE tpch_example"

# Run migrations
diesel migration run
```

### 3. Load Reference Data

Load customers and products into the database:

```rust
use tpch_example::generated::{CustomerCore, ProductCore};
use tpch_example::db::operations::GetOrCreate;

// Load customers
let customer_data = std::fs::read_to_string("config/examples/tpch/example_customers.json")?;
let customers: Vec<CustomerCore> = serde_json::from_str(&customer_data)?;

for customer in customers {
    let saved = customer.get_or_create(&mut conn)?;
    println!("Loaded customer: {}", saved.name);
}

// Load products
let product_data = std::fs::read_to_string("config/examples/tpch/example_products.json")?;
let products: Vec<ProductCore> = serde_json::from_str(&product_data)?;

for product in products {
    let saved = product.get_or_create(&mut conn)?;
    println!("Loaded product: {}", saved.name);
}
```

### 4. Process Orders

Process an order JSON file:

```rust
use tpch_example::generated::{OrderCore, OrderLineItemCore};

// Parse order
let order_data = std::fs::read_to_string("config/examples/tpch/example_order.json")?;
let order: OrderCore = serde_json::from_str(&order_data)?;

// Save order
let saved_order = order.get_or_create(&mut conn)?;
println!("Saved order: {}", saved_order.order_key);

// Extract and save line items
for line_item in OrderLineItemCore::from_parent(&order) {
    let saved_line = line_item.get_or_create(&mut conn)?;
    println!("  Line {}: {} units of {}",
        saved_line.line_number,
        saved_line.quantity,
        saved_line.part_key
    );
}
```

### 5. Use from Python

```python
from tpch_example._rust import (
    CustomerCore,
    ProductCore,
    OrderCore,
    OrderLineItemCore,
    customer_get_or_create,
    order_get_or_create,
)

# Create a customer
customer = CustomerCore(
    customer_key="C1001",
    name="Acme Corporation",
    account_balance=5432.10,
    market_segment="BUILDING",
)

# Save to database
saved_customer = customer_get_or_create(py, customer, database)
print(f"Customer ID: {saved_customer.id}")

# Create an order
order = OrderCore(
    order_key="O12345",
    customer_key="C1001",
    order_status="O",
    total_price=1234.56,
    order_date="2025-01-15",
)

saved_order = order_get_or_create(py, order, database)
print(f"Order ID: {saved_order.id}")
```

## Example Data Files

The example includes sample data:

- **`example_customers.json`** - 3 sample customers
- **`example_products.json`** - 3 sample products
- **`example_order.json`** - 1 order with 2 line items

## Database Schema

The generated schema creates these tables:

```sql
CREATE TABLE customers (
    id INT AUTO_INCREMENT PRIMARY KEY,
    customer_key VARCHAR(255) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    address VARCHAR(255),
    nation_key VARCHAR(255),
    phone VARCHAR(255),
    account_balance DOUBLE NOT NULL,
    market_segment VARCHAR(255),
    comment TEXT
);

CREATE TABLE products (
    id INT AUTO_INCREMENT PRIMARY KEY,
    part_key VARCHAR(255) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    manufacturer VARCHAR(255),
    brand VARCHAR(255),
    product_type VARCHAR(255),
    size INT,
    container VARCHAR(255),
    retail_price DOUBLE NOT NULL,
    comment TEXT
);

CREATE TABLE orders (
    id INT AUTO_INCREMENT PRIMARY KEY,
    order_key VARCHAR(255) NOT NULL UNIQUE,
    customer_key VARCHAR(255) NOT NULL,
    order_status VARCHAR(255) NOT NULL,
    total_price DOUBLE NOT NULL,
    order_date VARCHAR(255) NOT NULL,
    order_priority VARCHAR(255),
    clerk VARCHAR(255),
    ship_priority INT,
    comment TEXT
);

CREATE TABLE order_line_items (
    id INT AUTO_INCREMENT PRIMARY KEY,
    order_key VARCHAR(255) NOT NULL,
    line_number INT NOT NULL,
    part_key VARCHAR(255) NOT NULL,
    supplier_key VARCHAR(255),
    quantity INT NOT NULL,
    extended_price DOUBLE NOT NULL,
    discount DOUBLE,
    tax DOUBLE,
    return_flag VARCHAR(255),
    line_status VARCHAR(255),
    ship_date VARCHAR(255),
    commit_date VARCHAR(255),
    receipt_date VARCHAR(255),
    UNIQUE KEY (order_key, line_number)
);
```

## Key Nomnom Features Demonstrated

1. **Root Entities** - Customer, Product, and Order are root entities loaded from data files

2. **Derived Entities** - OrderLineItem is derived from Order using the `repeated_for` pattern

3. **Parent-Child Relationships** - OrderLineItem copies `order_key` from its parent Order

4. **Database Persistence** - All entities configured with Diesel ORM models and operations

5. **Unique Constraints** - Each entity has unicity fields to prevent duplicates

6. **Computed Fields** - Order has a `line_item_count` computed from children

7. **Field Extraction** - OrderLineItem extracts fields from JSON using `extract_field` transform

8. **Type Safety** - Generated Rust code ensures type safety at compile time

9. **Python Bindings** - PyO3 bindings allow using entities from Python

## Next Steps

- Customize entities to match your business domain
- Add more transforms for complex field computations
- Add validation rules
- Create additional derived entities (e.g., OrderSummary, CustomerMetrics)
- Build a processing pipeline to handle batch imports
- Add custom business logic in Rust or Python

## Related Documentation

- [Transform YAML Schema](../../../docs/transform_yaml_schema.md)
- [Main README](../../../README.md)
