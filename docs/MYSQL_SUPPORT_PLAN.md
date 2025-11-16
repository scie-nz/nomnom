# MySQL Backend Support Plan

## Executive Summary

This plan outlines the strategy for adding MySQL backend support to nomnom, enabling users to easily switch between PostgreSQL and MySQL databases. **Good news:** The codebase already has ~80% of the multi-database infrastructure in place. The primary work involves genericizing the core runtime layer and fixing a few type mappings.

## Current State Analysis

### What's Already Done ✓

1. **DatabaseType Abstraction**
   - `DatabaseType` enum exists in multiple modules supporting PostgreSQL, MySQL, and MariaDB
   - Configuration structures (`WorkerConfig`, `IngestionServerConfig`, `DashboardConfig`) accept database type

2. **Conditional Code Generation**
   - Cargo.toml generation adapts Diesel features based on database type
   - Connection type aliases are conditional (PgConnection vs MysqlConnection)
   - SQL trigger generation has PostgreSQL vs MySQL branches
   - Dashboard backend has database-specific logic

3. **SQL Syntax Abstraction**
   - Trigger generation (`src/codegen/dashboard/sql_triggers.rs`) already handles differences:
     - `BIGSERIAL` (PostgreSQL) vs `BIGINT AUTO_INCREMENT` (MySQL)
     - `row_to_json()` (PostgreSQL) vs `JSON_OBJECT()` (MySQL)
     - Different delimiter and syntax requirements

### What's Missing ✗

1. **Core Runtime is PostgreSQL-Only**
   - `src/diesel_runtime/database.rs` - hardcoded to `PgConnection`
   - `src/diesel_runtime/operations.rs` - `GetOrCreate` and `BulkInsert` traits use only `PgConnection`

2. **Type Mapping Issues**
   - JSONB is hardcoded but is PostgreSQL-specific (MySQL uses JSON)
   - Location: `src/codegen/worker/database_rs.rs:155`

3. **Generated Diesel Operations**
   - `src/codegen/diesel/operations.rs` generates PostgreSQL-only imports

## Implementation Strategy

### Phase 1: Core Runtime Abstraction (CRITICAL)

**Goal:** Make the diesel_runtime module database-agnostic.

**Approach:** Use Diesel's backend traits to write generic code that works with any database.

#### 1.1 Genericize Connection Pool (`src/diesel_runtime/database.rs`)

**Current Code:**
```rust
use diesel::pg::PgConnection;
pub type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;
pub type PooledConnection = r2d2::PooledConnection<ConnectionManager<PgConnection>>;
```

**Strategy Options:**

**Option A - Trait Objects (Simple, runtime cost):**
```rust
use diesel::Connection;
pub type Pool = r2d2::Pool<ConnectionManager<dyn Connection>>;
```

**Option B - Conditional Compilation (Zero cost, requires features):**
```rust
#[cfg(feature = "postgres")]
use diesel::pg::PgConnection;
#[cfg(feature = "postgres")]
pub type DbConnection = PgConnection;

#[cfg(feature = "mysql")]
use diesel::mysql::MysqlConnection;
#[cfg(feature = "mysql")]
pub type DbConnection = MysqlConnection;

pub type Pool = r2d2::Pool<ConnectionManager<DbConnection>>;
```

**Option C - Generic Implementation (Most flexible):**
```rust
use diesel::Connection;
use std::marker::PhantomData;

pub struct DatabasePool<C: Connection + 'static> {
    pool: r2d2::Pool<ConnectionManager<C>>,
}

impl<C: Connection + 'static> DatabasePool<C> {
    pub fn new(database_url: &str) -> Result<Self, r2d2::Error> {
        let manager = ConnectionManager::<C>::new(database_url);
        let pool = r2d2::Pool::builder().build(manager)?;
        Ok(DatabasePool { pool })
    }
}
```

**Recommendation:** Use **Option B** (conditional compilation) because:
- Zero runtime cost
- Matches the pattern already used in code generation
- Simple to understand and maintain
- Aligns with Diesel's feature flag approach

#### 1.2 Genericize Operations (`src/diesel_runtime/operations.rs`)

**Current Code:**
```rust
use diesel::pg::PgConnection;

pub trait GetOrCreate<T> {
    fn get_or_create(&mut self, conn: &mut PgConnection, key_values: &HashMap<String, Value>) -> Result<T, diesel::result::Error>;
}
```

**Strategy:**

Use Diesel's `Connection` trait with generic bounds:

```rust
use diesel::Connection;

pub trait GetOrCreate<T, C: Connection> {
    fn get_or_create(
        &mut self,
        conn: &mut C,
        key_values: &HashMap<String, Value>
    ) -> Result<T, diesel::result::Error>;
}
```

**Alternative (if backend-specific SQL needed):**

```rust
use diesel::backend::Backend;
use diesel::connection::Connection;

pub trait GetOrCreate<T, C>
where
    C: Connection,
    C::Backend: Backend,
{
    fn get_or_create(
        &mut self,
        conn: &mut C,
        key_values: &HashMap<String, Value>
    ) -> Result<T, diesel::result::Error>;
}
```

### Phase 2: Code Generation Updates (IMPORTANT)

#### 2.1 Fix JSONB Type Mapping

**File:** `src/codegen/worker/database_rs.rs:155` and `src/codegen/ingestion_server/database_rs.rs`

**Current Code:**
```rust
let sql_type = match field_type_str {
    "Json" | "Object" | "List[Object]" => "JSONB",
    // ...
};
```

**Updated Code:**
```rust
let sql_type = match field_type_str {
    "Json" | "Object" | "List[Object]" => {
        match config.database_type {
            DatabaseType::PostgreSQL => "JSONB",
            DatabaseType::MySQL | DatabaseType::MariaDB => "JSON",
        }
    },
    // ...
};
```

**Important:** MySQL's JSON type requires different query syntax:
- PostgreSQL: `data->>'field'` or `data->'field'`
- MySQL: `JSON_EXTRACT(data, '$.field')` or `data->'$.field'`

If the codebase generates any JSON queries, those also need database-specific handling.

#### 2.2 Update Generated Diesel Operations

**File:** `src/codegen/diesel/operations.rs:73`

**Current:**
```rust
writeln!(output, "use diesel::pg::PgConnection;")?;
```

**Updated:**
```rust
match config.database_type {
    DatabaseType::PostgreSQL => {
        writeln!(output, "#[cfg(feature = \"postgres\")]")?;
        writeln!(output, "use diesel::pg::PgConnection;")?;
        writeln!(output, "#[cfg(feature = \"postgres\")]")?;
        writeln!(output, "pub type DbConnection = PgConnection;")?;
    }
    DatabaseType::MySQL | DatabaseType::MariaDB => {
        writeln!(output, "#[cfg(feature = \"mysql\")]")?;
        writeln!(output, "use diesel::mysql::MysqlConnection;")?;
        writeln!(output, "#[cfg(feature = \"mysql\")]")?;
        writeln!(output, "pub type DbConnection = MysqlConnection;")?;
    }
}

// Update trait signatures to use DbConnection
writeln!(output, "impl GetOrCreate<{}, DbConnection> for {} {{", ...)?;
```

#### 2.3 Verify String Type Lengths (MySQL-specific)

**Issue:** MySQL typically requires explicit VARCHAR lengths while PostgreSQL TEXT is unbounded.

**Files:** `src/codegen/worker/database_rs.rs`, `src/codegen/ingestion_server/database_rs.rs`

**Strategy:**
```rust
let sql_type = match field_type_str {
    "String" => {
        match config.database_type {
            DatabaseType::PostgreSQL => "TEXT",
            DatabaseType::MySQL | DatabaseType::MariaDB => "VARCHAR(255)", // or make configurable
        }
    },
    // ...
};
```

**Alternative:** Use `TEXT` for both (MySQL supports it since 5.0.3, but with 64KB limit vs PostgreSQL's 1GB).

### Phase 3: User Experience (CRITICAL)

#### 3.1 Configuration Interface

**Goal:** Make database selection simple and obvious.

**Proposed Configuration Structure:**

```yaml
# config/nomnom.yaml (main config file)
database:
  type: postgresql  # or "mysql", "mariadb"
  url: "postgres://user:pass@localhost/dbname"

  # Optional database-specific settings
  postgresql:
    jsonb_operators: true  # Enable ->, ->>, @> operators

  mysql:
    string_length: 255  # Default VARCHAR length
    use_utf8mb4: true   # Use utf8mb4 charset
```

**Or Environment Variable Approach:**
```bash
export NOMNOM_DATABASE_TYPE=postgresql  # or mysql
export DATABASE_URL=postgres://user:pass@localhost/dbname
```

**Or CLI Flag:**
```bash
nomnom build-from-config --database postgresql
nomnom build-from-config --database mysql
```

**Recommendation:** Support all three methods with precedence:
1. CLI flag (highest priority)
2. Environment variable
3. Config file (lowest priority)

#### 3.2 Migration Path for Existing Users

**Backward Compatibility:**
- Default to PostgreSQL if no database type specified
- Detect database type from DATABASE_URL schema (`postgres://` vs `mysql://`)

**Auto-detection:**
```rust
fn detect_database_type(url: &str) -> DatabaseType {
    if url.starts_with("postgres://") || url.starts_with("postgresql://") {
        DatabaseType::PostgreSQL
    } else if url.starts_with("mysql://") {
        DatabaseType::MySQL
    } else {
        DatabaseType::PostgreSQL // default
    }
}
```

#### 3.3 Documentation Updates

**Required Documentation:**

1. **Quick Start for Each Database:**
   ```markdown
   ## PostgreSQL Setup
   1. Install PostgreSQL
   2. Create database: `createdb nomnom`
   3. Set DATABASE_URL: `export DATABASE_URL=postgres://localhost/nomnom`
   4. Run: `nomnom build-from-config --database postgresql`

   ## MySQL Setup
   1. Install MySQL
   2. Create database: `CREATE DATABASE nomnom;`
   3. Set DATABASE_URL: `export DATABASE_URL=mysql://root@localhost/nomnom`
   4. Run: `nomnom build-from-config --database mysql`
   ```

2. **Database Feature Comparison:**
   - Which features work with both
   - PostgreSQL-specific features (JSONB operators, full-text search, etc.)
   - MySQL-specific considerations

3. **Switching Databases:**
   - Data migration guidance
   - Schema differences
   - Performance considerations

### Phase 4: Testing Strategy

#### 4.1 Unit Tests

**Test both database backends for:**
- Connection pooling
- CRUD operations
- GetOrCreate logic
- Bulk insert operations
- JSON field handling

**Approach:**
```rust
#[cfg(test)]
mod tests {
    #[test]
    #[cfg(feature = "postgres")]
    fn test_get_or_create_postgres() {
        // Test with PostgreSQL
    }

    #[test]
    #[cfg(feature = "mysql")]
    fn test_get_or_create_mysql() {
        // Test with MySQL
    }
}
```

#### 4.2 Integration Tests

**Test Cases:**
1. Generate worker code for PostgreSQL
2. Generate worker code for MySQL
3. Verify generated Cargo.toml has correct features
4. Verify generated schema.rs compiles with appropriate backend
5. Run actual database operations against both backends

**Infrastructure:**
- Use Docker containers for both PostgreSQL and MySQL
- CI pipeline runs tests against both databases
- Test data migration scenarios

#### 4.3 Example Projects

Update example configurations to show both:
```
config/examples/
├── tpch-postgresql/
│   └── nomnom.yaml (database: postgresql)
└── tpch-mysql/
    └── nomnom.yaml (database: mysql)
```

## Implementation Phases Timeline

### Phase 1: Foundation (Week 1)
- [ ] Genericize `src/diesel_runtime/database.rs` with conditional compilation
- [ ] Genericize `src/diesel_runtime/operations.rs` traits
- [ ] Add database type detection from DATABASE_URL
- [ ] Update Cargo.toml to support both postgres and mysql features

### Phase 2: Code Generation (Week 1-2)
- [ ] Fix JSONB → JSON type mapping
- [ ] Update `src/codegen/diesel/operations.rs` for conditional imports
- [ ] Update STRING → TEXT vs VARCHAR mapping
- [ ] Verify all type mappings are database-appropriate

### Phase 3: User Interface (Week 2)
- [ ] Add `--database` CLI flag
- [ ] Add config file `database.type` field
- [ ] Implement precedence: CLI > ENV > config
- [ ] Add helpful error messages for database mismatches

### Phase 4: Testing & Documentation (Week 2-3)
- [ ] Write unit tests for both backends
- [ ] Create integration test suite
- [ ] Update README with database selection instructions
- [ ] Create database comparison documentation
- [ ] Add troubleshooting guide

### Phase 5: Examples & Polish (Week 3)
- [ ] Create MySQL example configurations
- [ ] Test against real MySQL installation
- [ ] Performance benchmarking (PostgreSQL vs MySQL)
- [ ] Add migration guide for existing users

## Database Feature Compatibility Matrix

| Feature | PostgreSQL | MySQL | Notes |
|---------|-----------|-------|-------|
| Basic CRUD | ✓ | ✓ | Fully compatible |
| Auto-increment IDs | ✓ (BIGSERIAL) | ✓ (AUTO_INCREMENT) | Different syntax |
| JSON fields | ✓ (JSONB) | ✓ (JSON) | Different operators |
| Timestamps | ✓ | ✓ | Compatible |
| Triggers | ✓ (PL/pgSQL) | ✓ (MySQL syntax) | Already abstracted |
| Foreign Keys | ✓ | ✓ | Compatible |
| Full-text search | ✓ | ✓ | Different syntax |
| Array types | ✓ | ✗ | PostgreSQL only |
| UUID type | ✓ (native) | ✓ (CHAR(36)) | Different storage |

## Potential Issues & Mitigations

### Issue 1: JSON Query Operators

**Problem:** PostgreSQL's `->` and `->>` operators don't work in MySQL.

**Mitigation:**
- If nomnom generates JSON queries, create helper functions
- Document that complex JSON queries may require database-specific code
- Provide macro or trait for cross-database JSON access

### Issue 2: Transaction Isolation Levels

**Problem:** Different default isolation levels.

**Mitigation:**
- Explicitly set isolation level in connection pool configuration
- Document the behavior differences

### Issue 3: Case Sensitivity

**Problem:** MySQL table names are case-sensitive on Linux, case-insensitive on Windows/Mac.

**Mitigation:**
- Use lowercase table names consistently
- Set `lower_case_table_names=1` in MySQL config (document this)

### Issue 4: String Collation

**Problem:** MySQL has complex charset/collation system.

**Mitigation:**
- Default to `utf8mb4_unicode_ci`
- Document charset configuration
- Generate `CHARACTER SET utf8mb4` in CREATE TABLE

## Success Criteria

1. **Code compiles successfully** with both `--features postgres` and `--features mysql`
2. **Generated worker code works** with both database backends
3. **All tests pass** against both PostgreSQL and MySQL
4. **Example projects run** successfully with both databases
5. **Documentation is clear** on how to choose and switch databases
6. **Migration path exists** for existing PostgreSQL users

## Open Questions

1. **Should we support runtime database switching?**
   - Probably not - compile-time is simpler and safer

2. **Should we support using both databases simultaneously?**
   - Edge case, probably defer to future if requested

3. **What about SQLite support?**
   - Diesel already included SQLite feature in Cargo.toml
   - Could be added with similar pattern (Phase 6?)

4. **How to handle database-specific performance optimizations?**
   - Start with compatible subset
   - Add database-specific hints as needed

## Recommended Approach

**Start with minimal viable implementation:**

1. Make core runtime conditional-compiled for postgres/mysql
2. Fix JSONB type mapping
3. Add `--database` flag to CLI
4. Test with simple examples
5. Iterate based on real-world usage

**Don't over-engineer:**
- Avoid runtime abstraction if compile-time works
- Don't create database-agnostic query builders (Diesel handles this)
- Keep it simple - let Diesel do the heavy lifting

## References

- [Diesel Multi-Backend Guide](https://diesel.rs/)
- PostgreSQL vs MySQL JSON comparison
- Diesel feature flags documentation
- r2d2 connection pooling best practices
