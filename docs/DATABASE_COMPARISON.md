# Database Backend Comparison

Nomnom supports PostgreSQL, MySQL, and MariaDB as backend databases. This document compares the features, capabilities, and tradeoffs between these options.

## Quick Comparison Table

| Feature | PostgreSQL | MySQL | MariaDB | Notes |
|---------|-----------|-------|---------|-------|
| **Basic CRUD** | ✅ Full | ✅ Full | ✅ Full | All backends fully supported |
| **Auto-increment** | `BIGSERIAL` | `AUTO_INCREMENT` | `AUTO_INCREMENT` | Different syntax, same functionality |
| **JSON fields** | `JSONB` (binary) | `JSON` (text) | `JSON` (text) | PostgreSQL has better JSON performance |
| **Timestamps** | ✅ | ✅ | ✅ | Fully compatible |
| **Transactions** | ✅ ACID | ✅ ACID | ✅ ACID | All support full ACID transactions |
| **Foreign Keys** | ✅ | ✅ | ✅ | Fully compatible |
| **Full-text search** | `tsvector` | `FULLTEXT` | `FULLTEXT` | Different syntax |
| **Array types** | ✅ Native | ❌ | ❌ | PostgreSQL only |
| **UUID type** | ✅ Native | CHAR(36) | CHAR(36) | Different storage methods |
| **Text fields** | TEXT (1GB) | TEXT (64KB) | TEXT (64KB) | Different size limits |
| **Case sensitivity** | Configurable | Platform-dependent | Platform-dependent | See notes below |

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
| Json/Object | **JSONB** | **JSON** |

**Key Difference**: PostgreSQL uses `JSONB` (binary JSON with indexing), while MySQL/MariaDB use `JSON` (text-based).

## Performance Considerations

### PostgreSQL
**Strengths:**
- Excellent JSON query performance with JSONB
- Advanced indexing (GiN, GiST, partial indexes)
- Better support for complex queries
- Native array types
- Large text field support (up to 1GB)

**Tradeoffs:**
- Slightly higher memory usage
- More complex to tune for high-write workloads

**Best For:**
- Complex analytical queries
- Heavy JSON usage
- Applications needing advanced features

### MySQL/MariaDB
**Strengths:**
- Excellent read performance
- Lower memory footprint
- Simpler replication setup
- Wide hosting availability

**Tradeoffs:**
- JSON stored as text (slower queries)
- TEXT fields limited to 64KB
- No native array types

**Best For:**
- Read-heavy workloads
- Simple data models
- Hosting environments where PostgreSQL isn't available

## SQL Syntax Differences

### Auto-increment IDs

**PostgreSQL:**
```sql
CREATE TABLE orders (
    id BIGSERIAL PRIMARY KEY,
    ...
);
```

**MySQL/MariaDB:**
```sql
CREATE TABLE orders (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    ...
);
```

### JSON Operations

**PostgreSQL:**
```sql
-- Query JSON fields
SELECT data->>'customer_name' FROM orders;
SELECT data->'items'->0 FROM orders;

-- JSON indexing
CREATE INDEX idx_customer ON orders USING GIN ((data->'customer'));
```

**MySQL/MariaDB:**
```sql
-- Query JSON fields
SELECT JSON_EXTRACT(data, '$.customer_name') FROM orders;
SELECT JSON_EXTRACT(data, '$.items[0]') FROM orders;

-- JSON indexing (limited)
CREATE INDEX idx_customer ON orders ((CAST(data->>'$.customer' AS CHAR(255))));
```

### Triggers

**PostgreSQL:**
```sql
CREATE OR REPLACE FUNCTION notify_change()
RETURNS TRIGGER AS $$
BEGIN
    -- Function body
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_name
    AFTER INSERT ON table_name
    FOR EACH ROW
    EXECUTE FUNCTION notify_change();
```

**MySQL/MariaDB:**
```sql
DELIMITER $$

CREATE TRIGGER trigger_name
    AFTER INSERT ON table_name
    FOR EACH ROW
BEGIN
    -- Trigger body
END$$

DELIMITER ;
```

## Migration Considerations

### Switching from PostgreSQL to MySQL

**Challenges:**
- JSONB queries need rewriting (different operators)
- TEXT field size limits (64KB in MySQL vs 1GB in PostgreSQL)
- No direct array type support

**Migration Steps:**
1. Review JSON query usage
2. Check text field sizes
3. Convert array types to JSON or separate tables
4. Update trigger syntax
5. Test thoroughly

### Switching from MySQL to PostgreSQL

**Benefits:**
- Better JSON performance
- No text size limits
- Advanced indexing options

**Migration Steps:**
1. Update auto-increment syntax in migrations
2. Change JSON to JSONB
3. Convert JSON_EXTRACT to -> operators
4. Update trigger syntax
5. Consider using PostgreSQL-specific features

## Platform-Specific Considerations

### Case Sensitivity

**PostgreSQL:**
- Table and column names are case-insensitive by default
- Can be made case-sensitive with quoted identifiers
- Consistent across all platforms

**MySQL/MariaDB:**
- Table names are **case-sensitive on Linux/Unix**, **case-insensitive on Windows/macOS**
- Column names are case-insensitive
- Can cause portability issues

**Recommendation:** Always use lowercase table names for cross-platform compatibility.

### Character Sets

**PostgreSQL:**
- Uses UTF-8 by default
- Simple, consistent behavior

**MySQL/MariaDB:**
- Multiple charset options (utf8, utf8mb4, latin1, etc.)
- `utf8mb4` required for full Unicode support (including emojis)
- Can be set per-database, per-table, or per-column

**Recommendation:** Always use `utf8mb4_unicode_ci` for MySQL/MariaDB.

## Connection Strings

### PostgreSQL
```
postgres://username:password@localhost:5432/database_name
postgresql://username:password@localhost:5432/database_name
```

### MySQL
```
mysql://username:password@localhost:3306/database_name
```

### MariaDB
```
mysql://username:password@localhost:3306/database_name
```
(Uses mysql:// scheme for compatibility)

## Testing Both Backends

Nomnom's test suite can run against both databases:

```bash
# Test PostgreSQL code generation
cargo test test_multi_database --features postgres -- test_worker_generation_postgresql

# Test MySQL code generation
cargo test test_multi_database --features postgres -- test_worker_generation_mysql

# Run all multi-database tests
cargo test test_multi_database --features postgres
```

## Feature Compatibility Matrix

| Nomnom Feature | PostgreSQL | MySQL | MariaDB |
|---------------|-----------|-------|---------|
| Basic entities | ✅ | ✅ | ✅ |
| Derived fields | ✅ | ✅ | ✅ |
| Parent relationships | ✅ | ✅ | ✅ |
| Repeated segments | ✅ | ✅ | ✅ |
| JSON extraction | ✅ | ⚠️ Limited | ⚠️ Limited |
| Bulk insert | ✅ | ✅ | ✅ |
| Get-or-create | ✅ | ✅ | ✅ |
| Dashboard triggers | ✅ | ✅ | ✅ |
| Worker generation | ✅ | ✅ | ✅ |
| Ingestion server | ✅ | ✅ | ✅ |

Legend:
- ✅ Full support
- ⚠️ Limited support (see notes)
- ❌ Not supported

## Choosing a Database

### Choose PostgreSQL if:
- You need advanced JSON querying
- You have complex analytical queries
- You use array types
- You need large text fields (> 64KB)
- You want the most SQL features

### Choose MySQL/MariaDB if:
- Your hosting provider doesn't support PostgreSQL
- You have simple, read-heavy workloads
- You need the smallest memory footprint
- You're already familiar with MySQL
- Your team prefers MySQL's simplicity

## Configuration Examples

### PostgreSQL Setup
```bash
# Install PostgreSQL
sudo apt-get install postgresql postgresql-contrib

# Create database
sudo -u postgres createdb nomnom_dev

# Set DATABASE_URL
export DATABASE_URL=postgres://postgres@localhost/nomnom_dev

# Generate code
nomnom build-from-config --database postgresql
```

### MySQL Setup
```bash
# Install MySQL
sudo apt-get install mysql-server

# Create database
mysql -u root -p -e "CREATE DATABASE nomnom_dev CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;"

# Set DATABASE_URL
export DATABASE_URL=mysql://root:password@localhost/nomnom_dev

# Generate code
nomnom build-from-config --database mysql
```

## Troubleshooting

### Error: "Unknown database type"
Make sure you're using one of: `postgresql`, `postgres`, `pg`, `mysql`, or `mariadb`.

### Error: Compile error about PgConnection/MysqlConnection
Ensure you're building with the correct feature flag:
```bash
# PostgreSQL
cargo build --features postgres

# MySQL
cargo build --no-default-features --features mysql
```

### Migration fails with syntax error
Check that your generated SQL matches your database type. Regenerate with the correct `--database` flag.

### JSON queries not working
- PostgreSQL uses `->` and `->>`
- MySQL uses `JSON_EXTRACT()` or `->` with different syntax
- Consider using Diesel's type-safe query builder instead of raw SQL

## See Also

- [Database Configuration Guide](DATABASE_CONFIGURATION.md) - How to configure database selection
- [MySQL Support Plan](MYSQL_SUPPORT_PLAN.md) - Implementation details
- [Main README](../README.md) - General nomnom documentation
