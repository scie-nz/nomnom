{{/*
Expand the name of the chart.
*/}}
{{- define "nomnom.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "nomnom.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "nomnom.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "nomnom.labels" -}}
helm.sh/chart: {{ include "nomnom.chart" . }}
{{ include "nomnom.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "nomnom.selectorLabels" -}}
app.kubernetes.io/name: {{ include "nomnom.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Component labels for NATS
*/}}
{{- define "nomnom.nats.labels" -}}
{{ include "nomnom.labels" . }}
app.kubernetes.io/component: nats
{{- end }}

{{/*
Component labels for PostgreSQL
*/}}
{{- define "nomnom.postgres.labels" -}}
{{ include "nomnom.labels" . }}
app.kubernetes.io/component: postgres
{{- end }}

{{/*
Component labels for Ingestion API
*/}}
{{- define "nomnom.ingestion-api.labels" -}}
{{ include "nomnom.labels" . }}
app.kubernetes.io/component: ingestion-api
{{- end }}

{{/*
Component labels for Worker
*/}}
{{- define "nomnom.worker.labels" -}}
{{ include "nomnom.labels" . }}
app.kubernetes.io/component: worker
{{- end }}

{{/*
Component labels for Dashboard Backend
*/}}
{{- define "nomnom.dashboard-backend.labels" -}}
{{ include "nomnom.labels" . }}
app.kubernetes.io/component: dashboard-backend
{{- end }}

{{/*
Component labels for Dashboard Frontend
*/}}
{{- define "nomnom.dashboard-frontend.labels" -}}
{{ include "nomnom.labels" . }}
app.kubernetes.io/component: dashboard-frontend
{{- end }}

{{/*
NATS fullname
*/}}
{{- define "nomnom.nats.fullname" -}}
{{- printf "%s-nats" (include "nomnom.fullname" .) | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
PostgreSQL fullname
*/}}
{{- define "nomnom.postgres.fullname" -}}
{{- printf "%s-postgres" (include "nomnom.fullname" .) | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Ingestion API fullname
*/}}
{{- define "nomnom.ingestion-api.fullname" -}}
{{- printf "%s-ingestion-api" (include "nomnom.fullname" .) | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Worker fullname
*/}}
{{- define "nomnom.worker.fullname" -}}
{{- printf "%s-worker" (include "nomnom.fullname" .) | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Dashboard backend fullname
*/}}
{{- define "nomnom.dashboard-backend.fullname" -}}
{{- printf "%s-dashboard-backend" (include "nomnom.fullname" .) | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Dashboard frontend fullname
*/}}
{{- define "nomnom.dashboard-frontend.fullname" -}}
{{- printf "%s-dashboard-frontend" (include "nomnom.fullname" .) | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Database URL
*/}}
{{- define "nomnom.database.url" -}}
{{- printf "postgresql://%s:%s@%s:%d/%s" .Values.database.username .Values.database.password (include "nomnom.postgres.fullname" .) (int .Values.postgresql.service.port) .Values.database.name }}
{{- end }}

{{/*
NATS URL
*/}}
{{- define "nomnom.nats.url" -}}
{{- printf "nats://%s-client:%d" (include "nomnom.nats.fullname" .) (int .Values.nats.service.client.port) }}
{{- end }}
