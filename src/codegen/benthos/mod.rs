/// Benthos pipeline generation for NATS JetStream to MySQL streaming
///
/// Generates Benthos pipeline configurations that:
/// - Consume messages from NATS JetStream
/// - Batch write to MySQL with connection pooling
/// - Auto-cleanup NATS streams via consumer ACKs
/// - Include MySQL schema definitions
/// - Generate deployment manifests (Docker Compose, Kubernetes)

use crate::codegen::EntityDef;
use crate::codegen::utils::to_snake_case;
use std::path::Path;
use std::error::Error;

mod pipeline_yaml;
mod mysql_schema;
mod nats_streams;
mod docker_compose;
mod kubernetes;
pub mod helm;

pub use pipeline_yaml::generate_pipeline_yaml;
pub use mysql_schema::generate_mysql_schema;
pub use nats_streams::generate_nats_setup_scripts;
pub use docker_compose::generate_docker_compose;
pub use kubernetes::generate_kubernetes_manifests;
pub use helm::generate_helm_templates;

/// Database type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseType {
    PostgreSQL,
    MySQL,
    MariaDB,
}

impl DatabaseType {
    pub fn as_str(&self) -> &str {
        match self {
            DatabaseType::PostgreSQL => "postgresql",
            DatabaseType::MySQL => "mysql",
            DatabaseType::MariaDB => "mariadb",
        }
    }

    pub fn from_str(s: &str) -> Result<DatabaseType, String> {
        match s.to_lowercase().as_str() {
            "postgresql" | "postgres" | "pg" => Ok(DatabaseType::PostgreSQL),
            "mysql" => Ok(DatabaseType::MySQL),
            "mariadb" => Ok(DatabaseType::MariaDB),
            _ => Err(format!("Unsupported database type: '{}'", s)),
        }
    }

    pub fn is_mysql_like(&self) -> bool {
        matches!(self, DatabaseType::MySQL | DatabaseType::MariaDB)
    }
}

/// Benthos generation configuration
#[derive(Debug, Clone)]
pub struct BenthosConfig {
    pub database_type: DatabaseType,
    pub nats_url: String,
    pub mysql_host: String,
    pub mysql_port: u16,
    pub mysql_database: String,
}

impl Default for BenthosConfig {
    fn default() -> Self {
        Self {
            database_type: DatabaseType::MySQL,
            nats_url: "nats://nats:4222".to_string(),
            mysql_host: "mysql".to_string(),
            mysql_port: 3306,
            mysql_database: "warehouse".to_string(),
        }
    }
}

/// Generate all Benthos artifacts
///
/// Creates the complete directory structure with:
/// - Benthos pipeline YAML files (one per transient entity)
/// - MySQL schema SQL files
/// - NATS stream setup scripts
/// - Docker Compose configuration
/// - Kubernetes deployment manifests
pub fn generate_all(
    entities: &[EntityDef],
    output_dir: &Path,
    config: &BenthosConfig,
) -> Result<(), Box<dyn Error>> {
    use std::fs;

    println!("🔧 Generating Benthos pipelines...\n");

    // Create output directory structure
    let pipelines_dir = output_dir.join("pipelines");
    let schema_dir = output_dir.join("schema");
    let nats_dir = output_dir.join("nats");
    let kubernetes_dir = output_dir.join("kubernetes");

    fs::create_dir_all(&pipelines_dir)?;
    fs::create_dir_all(&schema_dir)?;
    fs::create_dir_all(&nats_dir)?;
    fs::create_dir_all(&kubernetes_dir)?;

    // Filter entities that should have Benthos pipelines
    // - type: derived (transient entities)
    // - Not abstract
    let transient_entities: Vec<_> = entities.iter()
        .filter(|e| {
            e.source_type.to_lowercase() == "derived" && !e.is_abstract
        })
        .collect();

    println!("  ✓ Found {} transient entities for Benthos pipelines", transient_entities.len());
    for entity in &transient_entities {
        println!("    - {}", entity.name);
    }
    println!();

    // Generate Benthos pipeline YAML for each entity
    println!("📝 Generating Benthos pipeline configurations...");
    for entity in &transient_entities {
        let pipeline = generate_pipeline_yaml(entity, config)?;
        let pipeline_path = pipelines_dir.join(format!("{}.yaml", to_snake_case(&entity.name)));
        fs::write(&pipeline_path, pipeline)?;
        println!("  ✓ Generated {}", pipeline_path.display());
    }
    println!();

    // Generate MySQL schema files
    println!("📝 Generating MySQL schema files...");
    for entity in &transient_entities {
        let schema = generate_mysql_schema(entity, config)?;
        let schema_path = schema_dir.join(format!("{}.sql", to_snake_case(&entity.name)));
        fs::write(&schema_path, schema)?;
        println!("  ✓ Generated {}", schema_path.display());
    }
    println!();

    // Generate NATS stream setup scripts
    println!("📝 Generating NATS stream setup scripts...");
    let (transient_script, persistent_script) = generate_nats_setup_scripts(entities, config)?;

    let transient_path = nats_dir.join("setup-transient-streams.sh");
    fs::write(&transient_path, transient_script)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&transient_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&transient_path, perms)?;
    }
    println!("  ✓ Generated {}", transient_path.display());

    let persistent_path = nats_dir.join("setup-persistent-streams.sh");
    fs::write(&persistent_path, persistent_script)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&persistent_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&persistent_path, perms)?;
    }
    println!("  ✓ Generated {}", persistent_path.display());
    println!();

    // Generate Docker Compose configuration
    println!("📝 Generating Docker Compose configuration...");
    let docker_compose = generate_docker_compose(&transient_entities, config)?;
    let docker_compose_path = output_dir.join("docker-compose.yaml");
    fs::write(&docker_compose_path, docker_compose)?;
    println!("  ✓ Generated {}", docker_compose_path.display());
    println!();

    // Generate Kubernetes manifests
    println!("📝 Generating Kubernetes deployment manifests...");
    let (deployment, configmap, service) = generate_kubernetes_manifests(&transient_entities, config)?;

    let deployment_path = kubernetes_dir.join("deployment.yaml");
    fs::write(&deployment_path, deployment)?;
    println!("  ✓ Generated {}", deployment_path.display());

    let configmap_path = kubernetes_dir.join("configmap.yaml");
    fs::write(&configmap_path, configmap)?;
    println!("  ✓ Generated {}", configmap_path.display());

    let service_path = kubernetes_dir.join("service.yaml");
    fs::write(&service_path, service)?;
    println!("  ✓ Generated {}", service_path.display());
    println!();

    // Generate README
    println!("📝 Generating README...");
    let readme = generate_readme(&transient_entities, config)?;
    let readme_path = output_dir.join("README.md");
    fs::write(&readme_path, readme)?;
    println!("  ✓ Generated {}", readme_path.display());

    println!("\n✨ Benthos pipeline generation complete!");
    println!("📁 Output directory: {}", output_dir.display());
    println!("\n📖 Next steps:");
    println!("  1. Review generated files:");
    println!("     - Pipelines: {}/", pipelines_dir.display());
    println!("     - Schemas:   {}/", schema_dir.display());
    println!("     - NATS:      {}/", nats_dir.display());
    println!();
    println!("  2. Initialize MySQL schemas:");
    println!("     cd {}", output_dir.display());
    println!("     for schema in schema/*.sql; do");
    println!("       mysql -h {} -u $MYSQL_USER -p {} < \"$schema\"", config.mysql_host, config.mysql_database);
    println!("     done");
    println!();
    println!("  3. Create NATS streams:");
    println!("     ./nats/setup-transient-streams.sh");
    println!();
    println!("  4. Deploy Benthos pipelines:");
    println!("     docker-compose up -d  # For local testing");
    println!("     # OR");
    println!("     kubectl apply -f kubernetes/  # For production");

    Ok(())
}

/// Generate README file
fn generate_readme(
    entities: &[&EntityDef],
    config: &BenthosConfig,
) -> Result<String, Box<dyn Error>> {
    Ok(format!(r#"# Benthos Pipelines for NATS to MySQL

Auto-generated Benthos pipeline configurations for streaming transient entities from NATS JetStream to MySQL.

## Generated Files

- **pipelines/** - Benthos pipeline YAML files ({} entities)
- **schema/** - MySQL CREATE TABLE scripts
- **nats/** - NATS stream setup scripts
- **kubernetes/** - Kubernetes deployment manifests
- **docker-compose.yaml** - Docker Compose for local testing

## Entities Included

{}

## Quick Start

### 1. Initialize MySQL Schemas

```bash
# Create all entity tables in MySQL
for schema in schema/*.sql; do
  mysql -h {} -u $MYSQL_USER -p {} < "$schema"
done
```

### 2. Create NATS Streams

```bash
# Setup NATS streams with proper retention policies
chmod +x nats/*.sh
./nats/setup-transient-streams.sh
```

### 3. Deploy with Docker Compose (Local)

```bash
# Start NATS, MySQL, and all Benthos pipelines
docker-compose up -d

# View logs
docker-compose logs -f

# Stop all services
docker-compose down
```

### 4. Deploy to Kubernetes (Production)

```bash
# Create namespace
kubectl create namespace benthos

# Apply manifests
kubectl apply -f kubernetes/ -n benthos

# Check status
kubectl get pods -n benthos
kubectl logs -f deployment/benthos-diagnosis -n benthos
```

## Configuration

### Environment Variables

Each Benthos pipeline uses these environment variables:

- `NATS_URL` - NATS JetStream URL (default: `{}`)
- `MYSQL_HOST` - MySQL hostname (default: `{}`)
- `MYSQL_PORT` - MySQL port (default: `{}`)
- `MYSQL_DATABASE` - MySQL database name (default: `{}`)
- `MYSQL_USER` - MySQL username
- `MYSQL_PASSWORD` - MySQL password

### Customize Pipelines

To customize a pipeline, edit the corresponding file in `pipelines/`:

- **Batch size**: Adjust `batching.count` (default: 100)
- **Batch period**: Adjust `batching.period` (default: 1s)
- **Connection pool**: Adjust `conn_max_idle` and `conn_max_open`
- **ACK timeout**: Adjust `ack_wait` in NATS input

## Monitoring

Each Benthos pipeline exposes Prometheus metrics on port 4195:

- `input_received` - Messages pulled from NATS
- `output_sent` - Rows written to MySQL
- `output_error` - Failed writes
- `buffer_backlog` - Messages waiting in buffer

Access metrics: `http://localhost:4195/metrics`

## Architecture

```
HL7 Worker → NATS JetStream → Benthos Pipelines → MySQL Warehouse
                    ↓
            Consumer ACKs trigger cleanup
```

### NATS Stream Retention

- **Transient entities**: Interest-based retention (removes after all consumers ACK)
- **Persistent entities**: Workqueue retention (removes after first ACK)
- **Safety net**: Max age fallback (7d for transient, 24h for persistent)

### Idempotency

MySQL schemas include unique constraints on `(message_id, set_id)` to handle duplicate messages gracefully.

## Troubleshooting

### Pipeline not consuming messages

1. Check NATS connection:
   ```bash
   nats stream ls
   nats consumer ls STREAM_NAME
   ```

2. Check Benthos logs:
   ```bash
   docker-compose logs benthos-ENTITY_NAME
   ```

### MySQL connection errors

1. Verify credentials in `.env`
2. Check MySQL is accessible: `mysql -h $MYSQL_HOST -u $MYSQL_USER -p`
3. Verify database exists: `SHOW DATABASES;`

### High memory usage

1. Reduce batch size in pipeline YAML
2. Reduce connection pool size (`conn_max_open`)
3. Add resource limits in Kubernetes deployment

## Generated by nomnom

These files were auto-generated from entity YAML definitions. To regenerate:

```bash
nomnom generate-benthos \
  --entities /path/to/entities \
  --output /path/to/output \
  --database mysql \
  --nats-url nats://nats:4222
```

See `NOMNOM_BENTHOS_GENERATION.md` for details.
"#,
        entities.len(),
        entities.iter()
            .map(|e| format!("- **{}** - {}", e.name, e.doc.as_deref().unwrap_or("No description")))
            .collect::<Vec<_>>()
            .join("\n"),
        config.mysql_host,
        config.mysql_database,
        config.nats_url,
        config.mysql_host,
        config.mysql_port,
        config.mysql_database,
    ))
}

