/// Generate Helm chart templates for Benthos pipelines
///
/// Generates Helm templates that integrate with the existing hl7-nomnom-parser chart:
/// - benthos-deployment.yaml: One Deployment per transient entity
/// - benthos-configmap.yaml: ConfigMap with all pipeline YAML configs
/// - benthos-service.yaml: Service for Prometheus metrics scraping
/// - schema-init-job.yaml: Pre-install Job for MySQL schema initialization
/// - values-benthos.yaml: Per-entity configuration overrides

use crate::codegen::EntityDef;
use super::{BenthosConfig, to_snake_case, generate_pipeline_yaml, generate_mysql_schema};
use std::error::Error;
use std::collections::HashMap;

/// Generate all Helm templates for Benthos integration
pub fn generate_helm_templates(
    entities: &[&EntityDef],
    config: &BenthosConfig,
) -> Result<HelmTemplates, Box<dyn Error>> {
    Ok(HelmTemplates {
        deployment: generate_deployment_template(entities, config)?,
        configmap: generate_configmap_template(entities, config)?,
        service: generate_service_template()?,
        schema_init_job: generate_schema_init_job(entities, config)?,
        values: generate_values_yaml(entities, config)?,
    })
}

/// Container for all generated Helm templates
pub struct HelmTemplates {
    pub deployment: String,
    pub configmap: String,
    pub service: String,
    pub schema_init_job: String,
    pub values: String,
}

/// Generate benthos-deployment.yaml template
fn generate_deployment_template(
    entities: &[&EntityDef],
    _config: &BenthosConfig,
) -> Result<String, Box<dyn Error>> {
    let mut deployments = String::new();

    for (idx, entity) in entities.iter().enumerate() {
        let entity_snake = to_snake_case(&entity.name);
        let entity_kebab = entity_snake.replace('_', "-");

        if idx > 0 {
            deployments.push_str("\n---\n");
        }

        deployments.push_str(&format!(r#"apiVersion: apps/v1
kind: Deployment
metadata:
  name: {{{{ include "hl7-nomnom-parser.fullname" . }}}}-benthos-{entity_kebab}
  labels:
    app.kubernetes.io/name: {{{{ include "hl7-nomnom-parser.name" . }}}}
    app.kubernetes.io/component: benthos-{entity_kebab}
    {{{{- include "hl7-nomnom-parser.labels" . | nindent 4 }}}}
spec:
  replicas: {{{{ .Values.benthos.pipelines.{entity_snake}.replicaCount | default .Values.benthos.replicaCount }}}}
  selector:
    matchLabels:
      app.kubernetes.io/name: {{{{ include "hl7-nomnom-parser.name" . }}}}
      app.kubernetes.io/component: benthos-{entity_kebab}
  template:
    metadata:
      labels:
        app.kubernetes.io/name: {{{{ include "hl7-nomnom-parser.name" . }}}}
        app.kubernetes.io/component: benthos-{entity_kebab}
      annotations:
        checksum/config: {{{{ include (print $.Template.BasePath "/benthos-configmap.yaml") . | sha256sum }}}}
    spec:
      containers:
      - name: benthos
        image: "{{{{ .Values.benthos.image.repository }}}}:{{{{ .Values.benthos.image.tag }}}}"
        imagePullPolicy: {{{{ .Values.benthos.image.pullPolicy }}}}
        volumeMounts:
        - name: config
          mountPath: /benthos.yaml
          subPath: {entity_snake}.yaml
        env:
        - name: NATS_URL
          value: {{{{ tpl .Values.benthos.nats.url . | quote }}}}
        - name: MYSQL_HOST
          value: {{{{ .Values.benthos.warehouse.host | quote }}}}
        - name: MYSQL_PORT
          value: {{{{ .Values.benthos.warehouse.port | quote }}}}
        - name: MYSQL_DATABASE
          value: {{{{ .Values.benthos.warehouse.database | quote }}}}
        - name: MYSQL_USER
          value: {{{{ .Values.benthos.warehouse.username | quote }}}}
        - name: MYSQL_PASSWORD
          valueFrom:
            secretKeyRef:
              name: {{{{ .Values.benthos.warehouse.existingSecret }}}}
              key: password
        - name: LOG_LEVEL
          value: {{{{ .Values.benthos.logLevel | default "INFO" | quote }}}}
        ports:
        - containerPort: 4195
          name: metrics
          protocol: TCP
        resources:
          {{{{- if .Values.benthos.pipelines.{entity_snake}.resources }}}}
          {{{{- toYaml .Values.benthos.pipelines.{entity_snake}.resources | nindent 10 }}}}
          {{{{- else }}}}
          {{{{- toYaml .Values.benthos.resources | nindent 10 }}}}
          {{{{- end }}}}
        livenessProbe:
          httpGet:
            path: /ping
            port: 4195
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /ready
            port: 4195
          initialDelaySeconds: 5
          periodSeconds: 10
      volumes:
      - name: config
        configMap:
          name: {{{{ include "hl7-nomnom-parser.fullname" . }}}}-benthos-pipelines
"#, entity_snake = entity_snake, entity_kebab = entity_kebab));
    }

    Ok(format!(r#"{{{{- if .Values.benthos.enabled }}}}
{}
{{{{- end }}}}
"#, deployments))
}

/// Generate benthos-configmap.yaml template
fn generate_configmap_template(
    entities: &[&EntityDef],
    config: &BenthosConfig,
) -> Result<String, Box<dyn Error>> {
    let mut data_entries = String::new();

    for entity in entities {
        let entity_snake = to_snake_case(&entity.name);
        let pipeline_yaml = generate_pipeline_yaml(entity, config)?;

        // Escape the pipeline YAML for embedding in Helm template
        let escaped = pipeline_yaml.replace("{{", "{{`{{").replace("}}", "}}`}}");

        data_entries.push_str(&format!(r#"  {}.yaml: |
{}
"#, entity_snake, indent_string(&escaped, 4)));
    }

    Ok(format!(r#"{{{{- if .Values.benthos.enabled }}}}
apiVersion: v1
kind: ConfigMap
metadata:
  name: {{{{ include "hl7-nomnom-parser.fullname" . }}}}-benthos-pipelines
  labels:
    {{{{- include "hl7-nomnom-parser.labels" . | nindent 4 }}}}
data:
{}
{{{{- end }}}}
"#, data_entries))
}

/// Generate benthos-service.yaml template for Prometheus metrics
fn generate_service_template() -> Result<String, Box<dyn Error>> {
    Ok(r#"{{- if .Values.benthos.enabled }}
apiVersion: v1
kind: Service
metadata:
  name: {{ include "hl7-nomnom-parser.fullname" . }}-benthos-metrics
  labels:
    app.kubernetes.io/name: {{ include "hl7-nomnom-parser.name" . }}
    app.kubernetes.io/component: benthos-metrics
    {{- include "hl7-nomnom-parser.labels" . | nindent 4 }}
  annotations:
    prometheus.io/scrape: "true"
    prometheus.io/port: "4195"
    prometheus.io/path: "/metrics"
spec:
  type: ClusterIP
  clusterIP: None  # Headless service for pod-level metrics
  selector:
    app.kubernetes.io/name: {{ include "hl7-nomnom-parser.name" . }}
  ports:
  - name: metrics
    port: 4195
    targetPort: 4195
    protocol: TCP
{{- end }}
"#.to_string())
}

/// Generate schema-init-job.yaml template (Helm post-install hook)
fn generate_schema_init_job(
    entities: &[&EntityDef],
    config: &BenthosConfig,
) -> Result<String, Box<dyn Error>> {
    let mut schema_entries = String::new();

    for entity in entities {
        let entity_snake = to_snake_case(&entity.name);
        let schema_sql = generate_mysql_schema(entity, config)?;

        // Escape the SQL for embedding in Helm template
        let escaped = schema_sql.replace("{{", "{{`{{").replace("}}", "}}`}}");

        schema_entries.push_str(&format!(r#"  {}.sql: |
{}
"#, entity_snake, indent_string(&escaped, 4)));
    }

    Ok(format!(r#"{{{{- if and .Values.benthos.enabled .Values.benthos.schemaInit.enabled }}}}
apiVersion: v1
kind: ConfigMap
metadata:
  name: {{{{ include "hl7-nomnom-parser.fullname" . }}}}-mysql-schemas
  labels:
    {{{{- include "hl7-nomnom-parser.labels" . | nindent 4 }}}}
  annotations:
    "helm.sh/hook": post-install,post-upgrade
    "helm.sh/hook-weight": "-10"
data:
{}
---
apiVersion: batch/v1
kind: Job
metadata:
  name: {{{{ include "hl7-nomnom-parser.fullname" . }}}}-schema-init
  labels:
    {{{{- include "hl7-nomnom-parser.labels" . | nindent 4 }}}}
  annotations:
    "helm.sh/hook": post-install,post-upgrade
    "helm.sh/hook-weight": "-5"
    "helm.sh/hook-delete-policy": before-hook-creation,hook-succeeded
spec:
  backoffLimit: 3
  template:
    metadata:
      name: schema-init
    spec:
      restartPolicy: OnFailure
      containers:
      - name: mysql-init
        image: mysql:8.0
        command:
        - /bin/bash
        - -c
        - |
          set -e
          echo "Waiting for MySQL to be ready..."
          until mysql -h $MYSQL_HOST -P $MYSQL_PORT -u $MYSQL_USER -p$MYSQL_PASSWORD -e "SELECT 1" > /dev/null 2>&1; do
            echo "MySQL not ready, waiting..."
            sleep 2
          done

          echo "Creating database if it doesn't exist..."
          mysql -h $MYSQL_HOST -P $MYSQL_PORT -u $MYSQL_USER -p$MYSQL_PASSWORD -e "CREATE DATABASE IF NOT EXISTS \`$MYSQL_DATABASE\`;"
          echo "Database '$MYSQL_DATABASE' ready"

          echo "Initializing warehouse schemas..."
          for schema in /schemas/*.sql; do
            echo "Applying $schema"
            mysql -h $MYSQL_HOST -P $MYSQL_PORT -u $MYSQL_USER -p$MYSQL_PASSWORD $MYSQL_DATABASE < "$schema"
          done
          echo "Schema initialization complete!"
        env:
        - name: MYSQL_HOST
          value: {{{{ .Values.benthos.warehouse.host | quote }}}}
        - name: MYSQL_PORT
          value: {{{{ .Values.benthos.warehouse.port | quote }}}}
        - name: MYSQL_DATABASE
          value: {{{{ .Values.benthos.warehouse.database | quote }}}}
        - name: MYSQL_USER
          value: {{{{ .Values.benthos.warehouse.username | quote }}}}
        - name: MYSQL_PASSWORD
          valueFrom:
            secretKeyRef:
              name: {{{{ .Values.benthos.warehouse.existingSecret }}}}
              key: password
        volumeMounts:
        - name: schemas
          mountPath: /schemas
      volumes:
      - name: schemas
        configMap:
          name: {{{{ include "hl7-nomnom-parser.fullname" . }}}}-mysql-schemas
{{{{- end }}}}
"#, schema_entries))
}

/// Generate values-benthos.yaml with per-entity configuration
fn generate_values_yaml(
    entities: &[&EntityDef],
    _config: &BenthosConfig,
) -> Result<String, Box<dyn Error>> {
    let mut pipelines = String::new();

    for entity in entities {
        let entity_snake = to_snake_case(&entity.name);

        // Determine default replica count based on entity characteristics
        let default_replicas = match &entity.repetition {
            Some(rep) if rep.to_lowercase() == "repeated" => 1,
            _ => 1,
        };

        pipelines.push_str(&format!(r#"
    {}:
      replicaCount: {}
      # Uncomment to override resources for this entity
      # resources:
      #   requests:
      #     memory: "512Mi"
      #     cpu: "200m"
      #   limits:
      #     memory: "1Gi"
      #     cpu: "500m"
"#, entity_snake, default_replicas));
    }

    Ok(format!(r#"# Benthos pipeline configuration for hl7-nomnom-parser Helm chart
# Auto-generated by nomnom from entity definitions
#
# This file provides per-entity configuration overrides for Benthos pipelines.
# Use with: helm install -f values.yaml -f values-benthos.yaml

benthos:
  enabled: true

  # Default replica count for all pipelines
  replicaCount: 1

  # Benthos Docker image
  image:
    repository: jeffail/benthos
    tag: latest
    pullPolicy: IfNotPresent

  # Default resource limits for all pipelines
  resources:
    requests:
      memory: "256Mi"
      cpu: "100m"
    limits:
      memory: "512Mi"
      cpu: "500m"

  # MySQL warehouse database configuration
  warehouse:
    # Host should match your MySQL/MariaDB service name
    # Common values:
    #   - Bundled MySQL subchart: "<release-name>-mysql"
    #   - Bundled MariaDB subchart: "<release-name>-mariadb"
    #   - External database: "mysql.example.com"
    host: "mysql"
    port: 3306
    # Database name where Benthos will write transient entities
    database: "nomnom"
    # MySQL user with write access to the database
    username: "nomnom"
    # Reference to Kubernetes secret containing MySQL password
    # The secret must have a 'password' key
    # Common values:
    #   - Bundled MySQL: "<release-name>-mysql"
    #   - Custom secret: "my-mysql-credentials"
    existingSecret: "mysql-credentials"

  # NATS JetStream connection (reuses existing NATS from chart)
  nats:
    url: "nats://{{{{ .Release.Name }}}}-nats:4222"

  # Logging level
  logLevel: "INFO"

  # Schema initialization settings
  schemaInit:
    enabled: true

  # Per-entity pipeline configuration
  # Override replicaCount or resources for high-volume entities
  pipelines:{}
"#, pipelines))
}

/// Helper function to indent a multi-line string
fn indent_string(s: &str, spaces: usize) -> String {
    let indent = " ".repeat(spaces);
    s.lines()
        .map(|line| if line.is_empty() { String::new() } else { format!("{}{}", indent, line) })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indent_string() {
        let input = "line1\nline2\n\nline4";
        let expected = "  line1\n  line2\n\n  line4";
        assert_eq!(indent_string(input, 2), expected);
    }
}
