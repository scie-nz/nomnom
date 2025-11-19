/// Generate Kubernetes deployment manifests

use crate::codegen::EntityDef;
use super::{BenthosConfig, to_snake_case};
use std::error::Error;

pub fn generate_kubernetes_manifests(
    entities: &[&EntityDef],
    config: &BenthosConfig,
) -> Result<(String, String, String), Box<dyn Error>> {
    let deployment = generate_deployment(entities, config)?;
    let configmap = generate_configmap(entities, config)?;
    let service = generate_service()?;

    Ok((deployment, configmap, service))
}

fn generate_deployment(
    entities: &[&EntityDef],
    config: &BenthosConfig,
) -> Result<String, Box<dyn Error>> {
    let mut deployments = String::new();

    for entity in entities {
        let entity_snake = to_snake_case(&entity.name);

        deployments.push_str(&format!(r#"---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: benthos-{}
  labels:
    app: benthos-{}
    entity: {}
spec:
  replicas: 3
  selector:
    matchLabels:
      app: benthos-{}
  template:
    metadata:
      labels:
        app: benthos-{}
    spec:
      containers:
      - name: benthos
        image: jeffail/benthos:latest
        volumeMounts:
        - name: config
          mountPath: /benthos.yaml
          subPath: {}.yaml
        env:
        - name: NATS_URL
          value: "{}"
        - name: MYSQL_HOST
          valueFrom:
            secretKeyRef:
              name: mysql-credentials
              key: host
        - name: MYSQL_PORT
          valueFrom:
            secretKeyRef:
              name: mysql-credentials
              key: port
        - name: MYSQL_DATABASE
          valueFrom:
            secretKeyRef:
              name: mysql-credentials
              key: database
        - name: MYSQL_USER
          valueFrom:
            secretKeyRef:
              name: mysql-credentials
              key: username
        - name: MYSQL_PASSWORD
          valueFrom:
            secretKeyRef:
              name: mysql-credentials
              key: password
        resources:
          requests:
            memory: "256Mi"
            cpu: "100m"
          limits:
            memory: "512Mi"
            cpu: "500m"
        ports:
        - containerPort: 4195
          name: metrics
      volumes:
      - name: config
        configMap:
          name: benthos-pipelines

"#,
            entity_snake, entity_snake, entity.name,
            entity_snake, entity_snake,
            entity_snake,
            config.nats_url
        ));
    }

    Ok(deployments)
}

fn generate_configmap(
    entities: &[&EntityDef],
    _config: &BenthosConfig,
) -> Result<String, Box<dyn Error>> {
    Ok(format!(r#"---
apiVersion: v1
kind: ConfigMap
metadata:
  name: benthos-pipelines
data:
  # Pipeline configurations will be mounted from generated YAML files
  # To update: kubectl create configmap benthos-pipelines --from-file=pipelines/ --dry-run=client -o yaml | kubectl apply -f -
  # For now, this is a placeholder. Use: kubectl create configmap benthos-pipelines --from-file=pipelines/
"#))
}

fn generate_service() -> Result<String, Box<dyn Error>> {
    Ok(r#"---
apiVersion: v1
kind: Service
metadata:
  name: benthos-metrics
  labels:
    app: benthos
spec:
  type: ClusterIP
  ports:
  - port: 4195
    targetPort: 4195
    protocol: TCP
    name: metrics
  selector:
    app: benthos
"#.to_string())
}
