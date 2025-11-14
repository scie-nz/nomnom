# Helm Chart Integration: Buildah + Zot + Entity YAMLs

## Overview

This document describes how to integrate in-cluster image building into the nomnom Helm chart using:
- **Zot** - Lightweight OCI registry (no SSL required)
- **Buildah** - Rootless container image builder
- **Entity YAMLs** - Stored in Helm values and mounted as ConfigMaps

## Architecture

```
Helm Install
    ↓
1. Deploy Zot Registry (PVC + Deployment + Service)
    ↓
2. Create Entity ConfigMaps (from values.yaml)
    ↓
3. Create Buildah Jobs (ingestion-api, worker, dashboard-backend)
    ↓
4. Jobs build images using Dockerfile.component
    ↓
5. Images pushed to Zot registry
    ↓
6. Main deployments use images from Zot
```

## Values.yaml Structure

Add to `nomnom-helm/values.yaml`:

```yaml
codegen:
  enabled: true  # Enable in-cluster code generation

  # Zot Registry Configuration
  registry:
    image:
      repository: ghcr.io/project-zot/zot-linux-amd64
      tag: latest
      pullPolicy: IfNotPresent
    storage:
      size: 20Gi
      storageClass: ""  # Use default
    dedupe: true
    gc: true
    gcDelay: "1h"
    gcInterval: "24h"
    logLevel: "info"
    resources:
      requests:
        memory: "256Mi"
        cpu: "100m"
      limits:
        memory: "512Mi"
        cpu: "500m"

  # Buildah Build Configuration
  buildah:
    image:
      repository: quay.io/buildah/stable
      tag: latest
      pullPolicy: IfNotPresent
    cache:
      size: 10Gi
      storageClass: ""
    resources:
      requests:
        memory: "3Gi"
        cpu: "2000m"
      limits:
        memory: "6Gi"
        cpu: "4000m"
    # Git repository to clone source from
    git:
      repository: "https://github.com/your-org/nomnom.git"
      branch: "main"
      # For private repos:
      # secretName: git-credentials

  # Entity YAMLs - define your data model here!
  entities:
    order.yaml: |
      entity:
        name: Order
        source_type: root
        prefix: "O"
        doc: "Represents a customer order"
        fields:
          - name: order_key
            type: String
            doc: "Unique order identifier"
            nullable: false
          - name: customer_key
            type: String
            doc: "Foreign key to customer"
            nullable: false
          - name: order_status
            type: String
            doc: "Order status"
            nullable: false
          - name: total_price
            type: Float
            doc: "Total order price"
            nullable: false
          - name: line_items
            type: List[Object]
            doc: "Array of line items"
            nullable: false
        persistence:
          database:
            conformant_table: orders
            conformant_id_column: id
            unicity_fields:
              - order_key
          primary_key:
            name: id
            type: Integer
            autogenerate: true

    orderlineitem.yaml: |
      entity:
        name: OrderLineItem
        source_type: derived
        parent: Order
        parent_reference: line_items
        prefix: "L"
        doc: "Order line item - derived from Order.line_items array"
        fields:
          - name: line_number
            type: Integer
            doc: "Line item number within order"
            nullable: false
          - name: part_key
            type: String
            doc: "Part key"
            nullable: false
          - name: quantity
            type: Integer
            doc: "Quantity"
            nullable: false
          - name: extended_price
            type: Float
            doc: "Extended price"
            nullable: false
        persistence:
          database:
            conformant_table: order_line_items
            conformant_id_column: id
            parent_foreign_key:
              column: order_id
              references_table: orders
              references_column: id
          primary_key:
            name: id
            type: Integer
            autogenerate: true

# Update image references to use Zot registry
ingestion:
  api:
    image:
      repository: "{{ .Values.codegen.enabled | ternary (printf \"%s-registry:5000/nomnom-ingestion-api\" (include \"nomnom.fullname\" .)) .Values.ingestion.api.image.repository }}"
      tag: latest
      pullPolicy: Always

  worker:
    image:
      repository: "{{ .Values.codegen.enabled | ternary (printf \"%s-registry:5000/nomnom-worker\" (include \"nomnom.fullname\" .)) .Values.ingestion.worker.image.repository }}"
      tag: latest
      pullPolicy: Always

dashboard:
  backend:
    image:
      repository: "{{ .Values.codegen.enabled | ternary (printf \"%s-registry:5000/nomnom-dashboard-backend\" (include \"nomnom.fullname\" .)) .Values.dashboard.backend.image.repository }}"
      tag: latest
      pullPolicy: Always
```

## Required Helm Templates

### 1. Registry: `templates/registry/zot-deployment.yaml`
Already created - deploys Zot registry with PVC storage

### 2. Entities: `templates/codegen/entity-configmaps.yaml`

```yaml
{{- if .Values.codegen.enabled }}
{{- range $filename, $content := .Values.codegen.entities }}
---
apiVersion: v1
kind: ConfigMap
metadata:
  name: {{ include "nomnom.fullname" $ }}-entity-{{ $filename | replace "." "-" }}
  namespace: {{ $.Values.global.namespace }}
  labels:
    {{- include "nomnom.labels" $ | nindent 4 }}
    app.kubernetes.io/component: codegen
data:
  {{ $filename }}: |
{{ $content | indent 4 }}
{{- end }}
{{- end }}
```

### 3. Buildah RBAC: `templates/codegen/buildah-rbac.yaml`

```yaml
{{- if .Values.codegen.enabled }}
---
apiVersion: v1
kind: ServiceAccount
metadata:
  name: {{ include "nomnom.fullname" . }}-buildah
  namespace: {{ .Values.global.namespace }}

---
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: {{ include "nomnom.fullname" . }}-buildah
  namespace: {{ .Values.global.namespace }}
rules:
- apiGroups: [""]
  resources: ["pods", "pods/log", "configmaps"]
  verbs: ["get", "list"]

---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: {{ include "nomnom.fullname" . }}-buildah
  namespace: {{ .Values.global.namespace }}
subjects:
- kind: ServiceAccount
  name: {{ include "nomnom.fullname" . }}-buildah
roleRef:
  kind: Role
  name: {{ include "nomnom.fullname" . }}-buildah
  apiGroup: rbac.authorization.k8s.io
{{- end }}
```

### 4. Build Cache PVC: `templates/codegen/buildah-cache.yaml`

```yaml
{{- if .Values.codegen.enabled }}
---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: {{ include "nomnom.fullname" . }}-buildah-cache
  namespace: {{ .Values.global.namespace }}
spec:
  accessModes:
    - ReadWriteOnce
  {{- if .Values.codegen.buildah.cache.storageClass }}
  storageClassName: {{ .Values.codegen.buildah.cache.storageClass }}
  {{- end }}
  resources:
    requests:
      storage: {{ .Values.codegen.buildah.cache.size }}
{{- end }}
```

### 5. Build Scripts: `templates/codegen/buildah-scripts.yaml`

```yaml
{{- if .Values.codegen.enabled }}
---
apiVersion: v1
kind: ConfigMap
metadata:
  name: {{ include "nomnom.fullname" . }}-buildah-scripts
  namespace: {{ .Values.global.namespace }}
data:
  build.sh: |
    #!/bin/bash
    set -e

    echo "Starting build for component: ${COMPONENT}"
    echo "Registry: ${REGISTRY}"

    # Copy entities from ConfigMaps
    mkdir -p /workspace/git/config/examples/tpch/entities
    {{- range $filename, $content := .Values.codegen.entities }}
    cp /entities/{{ $filename }} /workspace/git/config/examples/tpch/entities/
    {{- end }}

    # Build with buildah
    buildah --storage-driver=vfs build \
      --format=docker \
      --build-arg COMPONENT=${COMPONENT} \
      -f /workspace/git/Dockerfile.component \
      -t ${REGISTRY}/nomnom-${COMPONENT}:${TAG} \
      /workspace/git

    # Push to registry
    buildah --storage-driver=vfs push \
      --tls-verify=false \
      ${REGISTRY}/nomnom-${COMPONENT}:${TAG}

    echo "Build complete!"
{{- end }}
```

### 6. Build Jobs: `templates/codegen/buildah-jobs.yaml`

```yaml
{{- if .Values.codegen.enabled }}
{{- $components := list "ingestion-api" "worker" "dashboard-backend" }}
{{- range $component := $components }}
---
apiVersion: batch/v1
kind: Job
metadata:
  name: {{ include "nomnom.fullname" $ }}-build-{{ $component }}
  namespace: {{ $.Values.global.namespace }}
  annotations:
    "helm.sh/hook": post-install,post-upgrade
    "helm.sh/hook-weight": "5"
    "helm.sh/hook-delete-policy": before-hook-creation
spec:
  template:
    spec:
      serviceAccountName: {{ include "nomnom.fullname" $ }}-buildah
      restartPolicy: Never

      initContainers:
      - name: git-sync
        image: registry.k8s.io/git-sync/git-sync:v4.2.4
        args:
          - --repo={{ $.Values.codegen.buildah.git.repository }}
          - --branch={{ $.Values.codegen.buildah.git.branch }}
          - --root=/workspace
          - --one-time=true
          - --depth=1
        volumeMounts:
          - name: workspace
            mountPath: /workspace

      containers:
      - name: buildah
        image: "{{ $.Values.codegen.buildah.image.repository }}:{{ $.Values.codegen.buildah.image.tag }}"
        workingDir: /workspace/git
        command: ["/bin/bash", "/scripts/build.sh"]
        env:
        - name: COMPONENT
          value: "{{ $component }}"
        - name: REGISTRY
          value: "{{ include \"nomnom.fullname\" $ }}-registry:5000"
        - name: TAG
          value: "latest"
        volumeMounts:
        - name: workspace
          mountPath: /workspace
        - name: buildah-cache
          mountPath: /var/lib/containers
        - name: scripts
          mountPath: /scripts
        {{- range $filename, $content := $.Values.codegen.entities }}
        - name: entity-{{ $filename | replace "." "-" }}
          mountPath: /entities/{{ $filename }}
          subPath: {{ $filename }}
        {{- end }}
        securityContext:
          capabilities:
            add: ["SETUID", "SETGID"]
          allowPrivilegeEscalation: true
          runAsUser: 0
        resources:
          {{- toYaml $.Values.codegen.buildah.resources | nindent 10 }}

      volumes:
      - name: workspace
        emptyDir: {}
      - name: buildah-cache
        persistentVolumeClaim:
          claimName: {{ include "nomnom.fullname" $ }}-buildah-cache
      - name: scripts
        configMap:
          name: {{ include "nomnom.fullname" $ }}-buildah-scripts
          defaultMode: 0755
      {{- range $filename, $content := $.Values.codegen.entities }}
      - name: entity-{{ $filename | replace "." "-" }}
        configMap:
          name: {{ include "nomnom.fullname" $ }}-entity-{{ $filename | replace "." "-" }}
      {{- end }}
{{- end }}
{{- end }}
```

## Usage

1. **Enable code generation** in your values file:

```bash
helm install nomnom ./nomnom-helm \
  --set codegen.enabled=true \
  --set codegen.buildah.git.repository=https://github.com/your-org/nomnom.git \
  -n nomnom-dev
```

2. **Define entities inline** in values.yaml (see structure above)

3. **Monitor builds**:

```bash
# Watch build jobs
kubectl get jobs -n nomnom-dev -l app.kubernetes.io/component=codegen -w

# View build logs
kubectl logs -n nomnom-dev job/nomnom-build-ingestion-api -f

# Check registry contents
kubectl port-forward -n nomnom-dev svc/nomnom-registry 5000:5000
curl http://localhost:5000/v2/_catalog
```

4. **Access Zot UI**:

```bash
kubectl port-forward -n nomnom-dev svc/nomnom-registry 5000:5000
# Visit http://localhost:5000
```

## Benefits

✅ **Self-contained**: Everything defined in Helm values
✅ **No external registry needed**: Zot runs in-cluster
✅ **Entity versioning**: Entities tracked in Git with values
✅ **Automatic builds**: Helm hooks trigger builds on install/upgrade
✅ **Build caching**: Shared PVC speeds up subsequent builds
✅ **No SSL required**: Zot works over plain HTTP
✅ **Lightweight**: Buildah + Zot are minimal overhead

## Production Considerations

1. **Git credentials**: For private repos, create a secret:

```bash
kubectl create secret generic git-credentials \
  --from-literal=username=myuser \
  --from-literal=password=ghp_token \
  -n nomnom-dev
```

2. **Registry authentication** (if needed):

Add to Zot config in `zot-deployment.yaml`:

```json
"http": {
  "auth": {
    "htpasswd": {
      "path": "/etc/zot/htpasswd"
    }
  }
}
```

3. **Storage**: Use appropriate StorageClass for production PVCs

4. **Resource limits**: Adjust buildah resources based on cluster capacity

## Next Steps

1. Create the remaining template files listed above
2. Test with `helm template` to validate:

```bash
helm template nomnom ./nomnom-helm \
  --set codegen.enabled=true \
  --debug
```

3. Install and monitor:

```bash
helm install nomnom ./nomnom-helm \
  --set codegen.enabled=true \
  -n nomnom-dev \
  --create-namespace \
  --wait
```

## Troubleshooting

**Builds fail with permission errors**:
- Ensure buildah has SETUID/SETGID capabilities
- Check SELinux/AppArmor settings

**Images not found**:
- Verify Zot registry is running: `kubectl get pods -n nomnom-dev | grep registry`
- Check build job logs: `kubectl logs job/nomnom-build-ingestion-api -n nomnom-dev`

**Entity changes not reflected**:
- Entities are baked into images at build time
- Trigger rebuild: `helm upgrade nomnom ./nomnom-helm -n nomnom-dev`
