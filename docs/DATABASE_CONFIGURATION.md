# Database Configuration Guide

Nomnom supports PostgreSQL, MySQL, and MariaDB backends. This guide explains how to configure which database to use.

## Quick Start

### Using CLI Flag (Recommended for testing)
```bash
# PostgreSQL (default)
nomnom build-from-config --database postgresql

# MySQL
nomnom build-from-config --database mysql

# MariaDB
nomnom build-from-config --database mariadb
```

### Using Config File (Recommended for projects)
Add to your `nomnom.yaml`:
```yaml
database:
  type: postgresql  # Options: postgresql, mysql, mariadb
  url: postgres://user:pass@localhost/dbname  # Optional
```

### Using Environment Variables
```bash
# Option 1: Explicit database type
export NOMNOM_DATABASE_TYPE=mysql
nomnom build-from-config

# Option 2: Auto-detect from DATABASE_URL
export DATABASE_URL=mysql://user:pass@localhost/mydb
nomnom build-from-config  # Auto-detects MySQL
```

## Configuration Precedence

Database type is determined in this order (highest to lowest priority):

1. **CLI Flag**: `--database mysql`
2. **Environment Variable**: `NOMNOM_DATABASE_TYPE=mysql`
3. **Config File**: `database.type: mysql` in `nomnom.yaml`
4. **DATABASE_URL Detection**: Auto-detect from URL scheme
5. **Default**: PostgreSQL

## Complete Example

### nomnom.yaml
```yaml
project:
  name: my_project
  module_name: my_project._rust
  version: 1.0.0

database:
  type: mysql
  # url is optional - can use DATABASE_URL env var

paths:
  config_dir: config/entities
  # ... other paths
```

### .env file
```bash
# Database connection
DATABASE_URL=mysql://user:password@localhost:3306/mydb

# NATS configuration (if using workers)
NATS_URL=nats://localhost:4222
```

### Command
```bash
# Use config file setting (mysql)
nomnom build-from-config

# Override to use PostgreSQL
nomnom build-from-config --database postgresql
```

## Database Type Auto-Detection

Nomnom can automatically detect the database type from the `DATABASE_URL`:

- `postgres://...` or `postgresql://...` → PostgreSQL
- `mysql://...` → MySQL

This means you can often skip explicit configuration:

```bash
# Auto-detects PostgreSQL
export DATABASE_URL=postgres://localhost/mydb
nomnom build-from-config

# Auto-detects MySQL
export DATABASE_URL=mysql://localhost/mydb
nomnom build-from-config
```

## Generated Code Differences

### PostgreSQL
```rust
use diesel::pg::PgConnection;
pub type DbConnection = PgConnection;
```

```sql
-- JSON type
CREATE TABLE orders (
    data JSONB NOT NULL
);

-- Auto-increment
CREATE TABLE customers (
    id BIGSERIAL PRIMARY KEY
);
```

### MySQL
```rust
use diesel::mysql::MysqlConnection;
pub type DbConnection = MysqlConnection;
```

```sql
-- JSON type
CREATE TABLE orders (
    data JSON NOT NULL
);

-- Auto-increment
CREATE TABLE customers (
    id BIGINT AUTO_INCREMENT PRIMARY KEY
);
```

## Feature Flags (Build Time)

When building the generated code, you must select the database backend:

```bash
# Build for PostgreSQL
cd generated-worker
cargo build --features postgres

# Build for MySQL
cargo build --no-default-features --features mysql
```

**Note**: The generated code is database-specific at compile time. To switch databases, you must regenerate the code with the appropriate database type.

## Supported Database Types

| Database | Type String | URL Scheme | Status |
|----------|------------|------------|--------|
| PostgreSQL | `postgresql`, `postgres`, `pg` | `postgres://`, `postgresql://` | ✅ Fully supported |
| MySQL | `mysql` | `mysql://` | ✅ Fully supported |
| MariaDB | `mariadb` | (use `mysql://`) | ✅ MySQL-compatible |
| SQLite | `sqlite` | `sqlite://` | 🚧 Infrastructure exists, not tested |

## Type Mappings

| Nomnom Type | PostgreSQL | MySQL/MariaDB |
|-------------|------------|---------------|
| String | TEXT | TEXT |
| Integer (i32) | INTEGER | INTEGER |
| BigInt (i64) | BIGINT | BIGINT |
| Float/Decimal | NUMERIC | NUMERIC |
| Boolean | BOOLEAN | BOOLEAN |
| Date | DATE | DATE |
| DateTime | TIMESTAMP | TIMESTAMP |
| Json/Object | JSONB | JSON |

## Troubleshooting

### Error: "Unsupported database type"
- Check spelling: `postgresql`, `mysql`, or `mariadb`
- Aliases: `postgres` and `pg` work for PostgreSQL

### Error: Compile error about missing PgConnection
- Make sure you're building with the correct feature flag
- PostgreSQL: `cargo build --features postgres`
- MySQL: `cargo build --no-default-features --features mysql`

### Generated code doesn't match my database
- Regenerate code with the correct `--database` flag
- Check `nomnom.yaml` has correct `database.type`
- Clear the output directory before regenerating

## See Also

- [MySQL Support Plan](MYSQL_SUPPORT_PLAN.md) - Implementation details
- [Main README](../README.md) - General nomnom documentation
