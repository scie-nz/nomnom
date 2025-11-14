# Nomnom Entity Schema v1 Specification

**Version**: v1
**API Version**: `nomnom.io/v1`
**Status**: Draft

## Table of Contents

- [Overview](#overview)
- [Design Principles](#design-principles)
- [Naming Conventions](#naming-conventions)
- [Resource Structure](#resource-structure)
- [Metadata](#metadata)
- [Spec](#spec)
- [Field Definitions](#field-definitions)
- [Type System](#type-system)
- [Source Configuration](#source-configuration)
- [Persistence Configuration](#persistence-configuration)
- [Complete Examples](#complete-examples)
- [Migration Guide](#migration-guide)

---

## Overview

The Nomnom Entity Schema v1 is a declarative configuration format for defining data entities, their fields, relationships, and persistence mappings. It follows Kubernetes-style conventions for familiarity and tooling compatibility.

### Key Features

- **Single source of truth**: Field definitions include all metadata (logical + database)
- **No duplication**: Eliminates `fieldOverrides` pattern
- **K8s-inspired**: Uses `apiVersion`, `kind`, `metadata`, `spec` structure
- **Strongly typed**: Rich constraint system with validation
- **Composable**: Clear parent/child relationships

---

## Design Principles

1. **Convention over configuration**: Sensible defaults minimize boilerplate
2. **Progressive disclosure**: Simple cases are simple, complex cases are possible
3. **Self-documenting**: Schema is human-readable and explains intent
4. **Validation-friendly**: Can be validated with standard JSON Schema tools
5. **Backward compatible**: Legacy configs can coexist during migration

---

## Naming Conventions

Following Kubernetes API conventions, we use **camelCase** for all YAML field names:

| Context | Convention | Examples |
|---------|------------|----------|
| YAML fields | camelCase | `sourceType`, `maxLength`, `primaryKey` |
| Entity names | PascalCase | `PatientDiagnosis`, `Hl7v2Message` |
| Database fields | snake_case | `patient_id`, `created_at` |
| Table names | snake_case | `patient_diagnosis_conformant` |

**Rationale**: CamelCase is standard in Kubernetes, JSON, and most modern APIs. It provides consistency with tooling and feels natural to developers familiar with K8s.

---

## Resource Structure

All entity definitions follow this top-level structure:

```yaml
apiVersion: nomnom.io/v1
kind: Entity
metadata:
  # Resource identification and organization
spec:
  # Entity specification
status:
  # Runtime status (managed by system, not user-editable)
```

---

## Metadata

The `metadata` section contains resource identification and organizational information.

### Schema

```yaml
metadata:
  # Required: Unique entity name (PascalCase)
  name: string

  # Optional: Key-value labels for organization/filtering
  labels:
    key: value

  # Optional: Free-form annotations for documentation
  annotations:
    key: value
```

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Unique entity name in PascalCase (e.g., `PatientDiagnosis`) |
| `labels` | map[string]string | No | Structured metadata for filtering/grouping |
| `annotations` | map[string]string | No | Unstructured metadata for documentation |

### Standard Labels

| Label | Values | Description |
|-------|--------|-------------|
| `domain` | healthcare, finance, etc. | Business domain |
| `category` | clinical, administrative, etc. | Subcategory |
| `segment` | DG1, PR1, PID, etc. | HL7 segment type (if applicable) |
| `persistence` | transient, persistent | Storage strategy |

### Standard Annotations

| Annotation | Description |
|------------|-------------|
| `description` | Human-readable description |
| `owner` | Team or person responsible |
| `deprecated` | Deprecation notice |
| `migration.from` | Previous entity name |

### Example

```yaml
metadata:
  name: PatientDiagnosisCore
  labels:
    domain: healthcare
    category: clinical
    segment: DG1
    persistence: persistent
  annotations:
    description: "Patient diagnosis extracted from HL7 DG1 segment"
    owner: "clinical-data-team"
```

---

## Spec

The `spec` section defines the entity's structure, behavior, and relationships.

### Top-Level Schema

```yaml
spec:
  # Entity classification
  type: string                    # "root" | "derived"
  repetition: string              # "singleton" | "repeated"

  # Derivation configuration (for derived entities)
  derivation:
    # ... see Derivation section

  # Field definitions (unified - no more overrides!)
  fields:
    - name: string
      type: string
      constraints: object
      source: object

  # Persistence configuration
  persistence:
    enabled: boolean
    table: string
    # ... see Persistence section
```

### Type Field

Defines entity classification:

- **`root`**: Top-level entity (e.g., file-based source)
- **`derived`**: Derived from parent entity/entities

### Repetition Field

Defines cardinality relative to parent:

- **`singleton`** (default): One instance per parent
- **`repeated`**: Multiple instances per parent (e.g., diagnoses from DG1 segments)

---

## Field Definitions

Fields are the core of entity definitions. Each field definition is a complete specification including logical type, database constraints, and data source.

### Field Schema

```yaml
fields:
  - name: string                  # Required: Field name (camelCase)
    type: string                  # Required: Logical type

    # Constraints (database + validation)
    constraints:
      nullable: boolean           # Default: false
      maxLength: integer          # For strings
      minLength: integer          # For strings
      minimum: number             # For numeric types
      maximum: number             # For numeric types
      pattern: string             # Regex pattern
      enum: [string]              # Allowed values
      default: any                # Default value
      primaryKey: boolean         # Is primary key
      indexed: boolean            # Create index
      unique: boolean             # Unique constraint

    # Source configuration (where data comes from)
    source:
      # Option 1: Copy from parent field
      copyFrom: string            # Parent entity name
      field: string               # Field name in parent

      # Option 2: Transform/compute
      transform: string           # Transform function name
      inputs: [string]            # Input sources
      args: [any]                 # Transform arguments

      # Option 3: Constant value
      constant: any

    # Documentation
    doc: string                   # Field description
```

### Constraint Details

#### String Constraints

```yaml
type: string
constraints:
  maxLength: 255        # VARCHAR(255)
  minLength: 3          # Minimum length
  pattern: "^[A-Z]"     # Regex validation
  enum: ["A", "B", "C"] # Allowed values
  default: "unknown"    # Default value
```

#### Numeric Constraints

```yaml
type: integer
constraints:
  minimum: 0            # Minimum value (inclusive)
  maximum: 100          # Maximum value (inclusive)
  default: 0            # Default value
```

#### Timestamp Constraints

```yaml
type: timestamp
constraints:
  default: now()        # SQL: DEFAULT CURRENT_TIMESTAMP
  nullable: false
```

#### Index and Key Constraints

```yaml
constraints:
  primaryKey: true      # Primary key (implies indexed + unique + not null)
  indexed: true         # Create B-tree index
  unique: true          # Unique constraint
```

---

## Type System

Nomnom uses a logical type system that maps to database types.

### Primitive Types

| Nomnom Type | PostgreSQL | MySQL | Description |
|-------------|------------|-------|-------------|
| `string` | VARCHAR(255) | VARCHAR(255) | Text (default 255) |
| `integer` | INTEGER | INT | 32-bit integer |
| `bigint` | BIGINT | BIGINT | 64-bit integer |
| `float` | DOUBLE PRECISION | DOUBLE | Floating point |
| `decimal` | NUMERIC | DECIMAL | Fixed precision |
| `boolean` | BOOLEAN | TINYINT(1) | True/false |
| `timestamp` | TIMESTAMP | TIMESTAMP | Date + time |
| `date` | DATE | DATE | Date only |
| `json` | JSONB | JSON | JSON data |
| `uuid` | UUID | CHAR(36) | UUID |

### Type Modifiers

Types can be modified with constraints:

```yaml
# Variable-length string
type: string
constraints:
  maxLength: 100        # VARCHAR(100)

# Fixed precision decimal
type: decimal
constraints:
  precision: 10         # NUMERIC(10,2)
  scale: 2
```

### Array Types

```yaml
type: array
constraints:
  items:
    type: string        # Array of strings
    maxLength: 50
```

PostgreSQL: `VARCHAR(50)[]`
MySQL: Serialized as JSON

---

## Source Configuration

The `source` field defines where field data originates.

### Copy From Parent

Copy value from parent entity field:

```yaml
source:
  copyFrom: Filename    # Parent entity name
  field: facilityId     # Field name in parent
```

### Transform/Compute

Apply transformation function:

```yaml
source:
  transform: extractFromHl7Segment
  inputs: [segment]     # Input sources (variables in scope)
  args: ["DG1.3.1"]     # Transform-specific arguments
```

**Input Sources**:
- Parent entity names (e.g., `Filename`, `EventType`)
- Loop variables from `repeatedFor.itemName` (e.g., `segment`)

### Constant Value

Set constant value:

```yaml
source:
  constant: "SYSTEM"
```

### Derived Field (No Source)

Fields without a source are computed at runtime or provided by application logic:

```yaml
# No source specified - application must provide
- name: createdAt
  type: timestamp
  constraints:
    default: now()
```

---

## Derivation Configuration

Defines how derived entities relate to parent entities.

### Single Parent (Simple)

```yaml
spec:
  type: derived
  derivation:
    parent: Hl7v2Message
```

### Multiple Parents

```yaml
spec:
  type: derived
  derivation:
    parents:
      - name: filename      # Variable name in scope
        entity: Filename    # Entity type
      - name: diagnosis
        entity: Diagnosis
```

### Repeated Entity

For entities derived from repeating segments:

```yaml
spec:
  type: derived
  repetition: repeated
  derivation:
    repeatedFor:
      entity: Hl7v2Message     # Parent entity
      field: dg1Segments       # Field containing array
      itemName: segment        # Variable name for loop item
```

Generated code pattern:
```rust
for segment in &hl7v2message.dg1_segments {
    // segment is now in scope for field sources
    let diagnosis_code = extract_from_hl7_segment(segment.as_str(), "DG1.3.1");
}
```

---

## Persistence Configuration

Defines database storage for persistent entities.

### Basic Persistence

```yaml
spec:
  persistence:
    enabled: true
    table: patient_diagnosis_conformant
```

### Advanced Persistence

```yaml
spec:
  persistence:
    enabled: true
    table: patient_diagnosis_conformant

    # Composite indexes
    indexes:
      - name: idx_facility_patient
        fields: [facilityId, patientIdentifier]
        unique: true

      - name: idx_diagnosis_date
        fields: [facilityId, diagnosisDatetime]

    # Unicity constraint (for upsert logic)
    unicity:
      fields: [facilityId, patientIdentifier, diagnosisCode]

    # Legacy table mapping (for migration)
    legacyMapping:
      enabled: true
      table: patient_diagnosis_legacy
      idColumn: legacy_id
```

### Persistence Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `enabled` | boolean | No (default: true) | Enable database persistence |
| `table` | string | Yes | Table name |
| `indexes` | array | No | Composite index definitions |
| `unicity` | object | No | Fields for upsert conflict detection |
| `legacyMapping` | object | No | Legacy table mapping |

### Index Definition

```yaml
indexes:
  - name: string          # Index name
    fields: [string]      # Fields to index (camelCase in YAML)
    unique: boolean       # Unique constraint (default: false)
    method: string        # Index method: btree, hash, gin, gist (default: btree)
```

---

## Complete Examples

### Example 1: Transient Root Entity

```yaml
apiVersion: nomnom.io/v1
kind: Entity
metadata:
  name: Filename
  labels:
    domain: system
    persistence: transient

spec:
  type: root

  fields:
    - name: filename
      type: string
      constraints:
        maxLength: 500
      doc: "Full filename"

    - name: facilityId
      type: string
      constraints:
        maxLength: 50
      source:
        transform: extractFilenameComponent
        inputs: [filename]
        args: ["facilityId"]
      doc: "Facility ID extracted from filename"

    - name: fileDate
      type: date
      source:
        transform: extractFilenameComponent
        inputs: [filename]
        args: ["date"]
      doc: "Date extracted from filename"
```

### Example 2: Repeating Transient Entity

```yaml
apiVersion: nomnom.io/v1
kind: Entity
metadata:
  name: Diagnosis
  labels:
    domain: healthcare
    category: clinical
    segment: DG1
    persistence: transient

spec:
  type: derived
  repetition: repeated

  derivation:
    repeatedFor:
      entity: Hl7v2Message
      field: dg1Segments
      itemName: segment

  fields:
    - name: setId
      type: string
      constraints:
        maxLength: 10
        nullable: true
      source:
        transform: extractFromHl7Segment
        inputs: [segment]
        args: ["DG1.1"]
      doc: "DG1-1: Set ID"

    - name: diagnosisCode
      type: string
      constraints:
        maxLength: 50
        nullable: true
      source:
        transform: extractFromHl7Segment
        inputs: [segment]
        args: ["DG1.3.1"]
      doc: "DG1-3.1: Diagnosis code (e.g., ICD-10)"

    - name: diagnosisCodeText
      type: string
      constraints:
        maxLength: 255
        nullable: true
      source:
        transform: extractFromHl7Segment
        inputs: [segment]
        args: ["DG1.3.2"]
      doc: "DG1-3.2: Diagnosis code description"

    - name: diagnosisCodeSystem
      type: string
      constraints:
        maxLength: 50
        nullable: true
      source:
        transform: extractFromHl7Segment
        inputs: [segment]
        args: ["DG1.3.3"]
      doc: "DG1-3.3: Coding system (ICD10, ICD9, etc.)"
```

### Example 3: Persistent Derived Entity

```yaml
apiVersion: nomnom.io/v1
kind: Entity
metadata:
  name: PatientDiagnosisCore
  labels:
    domain: healthcare
    category: clinical
    persistence: persistent
  annotations:
    description: "Patient diagnosis record for database persistence"
    owner: "clinical-data-team"

spec:
  type: derived

  derivation:
    parents:
      - name: filename
        entity: Filename
      - name: eventType
        entity: EventType
      - name: patientIdentification
        entity: PatientIdentification
      - name: diagnosis
        entity: Diagnosis

  fields:
    # Business key fields
    - name: facilityId
      type: string
      constraints:
        maxLength: 255
        nullable: false
        indexed: true
      source:
        copyFrom: filename
        field: facilityId
      doc: "Facility identifier"

    - name: patientIdentifier
      type: string
      constraints:
        maxLength: 100
        nullable: false
        indexed: true
      source:
        copyFrom: patientIdentification
        field: patientIdentifier
      doc: "Patient MRN or account number"

    - name: diagnosisCode
      type: string
      constraints:
        maxLength: 50
        nullable: true
        indexed: true
      source:
        copyFrom: diagnosis
        field: diagnosisCode
      doc: "Diagnosis code (ICD-10, etc.)"

    - name: diagnosisCodeText
      type: string
      constraints:
        maxLength: 255
        nullable: true
      source:
        copyFrom: diagnosis
        field: diagnosisCodeText
      doc: "Diagnosis description"

    - name: diagnosisCodeSystem
      type: string
      constraints:
        maxLength: 50
        nullable: true
      source:
        copyFrom: diagnosis
        field: diagnosisCodeSystem
      doc: "Coding system"

    - name: eventTimestamp
      type: timestamp
      constraints:
        nullable: true
      source:
        copyFrom: eventType
        field: eventTimestamp
      doc: "Event occurrence timestamp"

    # Audit fields
    - name: createdAt
      type: timestamp
      constraints:
        nullable: false
        default: now()
      doc: "Record creation timestamp"

    - name: updatedAt
      type: timestamp
      constraints:
        nullable: false
        default: now()
      doc: "Record last update timestamp"

  persistence:
    enabled: true
    table: patient_diagnosis_conformant

    indexes:
      - name: idx_facility_patient_diagnosis
        fields: [facilityId, patientIdentifier, diagnosisCode]
        unique: true

      - name: idx_event_timestamp
        fields: [facilityId, eventTimestamp]

    unicity:
      fields: [facilityId, patientIdentifier, diagnosisCode]
```

### Example 4: Entity with Legacy Mapping

```yaml
apiVersion: nomnom.io/v1
kind: Entity
metadata:
  name: MPI
  labels:
    domain: healthcare
    category: administrative
    persistence: persistent

spec:
  type: derived

  derivation:
    parents:
      - name: filename
        entity: Filename
      - name: patientIdentification
        entity: PatientIdentification

  fields:
    - name: patientIdentifier
      type: string
      constraints:
        maxLength: 100
        nullable: false
        primaryKey: true
      source:
        copyFrom: patientIdentification
        field: patientIdentifier

    - name: facilityId
      type: string
      constraints:
        maxLength: 255
        nullable: false
        indexed: true
      source:
        copyFrom: filename
        field: facilityId

  persistence:
    enabled: true
    table: mpi_id_conformant

    legacyMapping:
      enabled: true
      table: mpi_id_legacy
      idColumn: mpi_id

    unicity:
      fields: [facilityId, patientIdentifier]
```

---

## Migration Guide

### From Legacy Format to v1

#### Step 1: Add Metadata Section

**Before**:
```yaml
entity:
  name: PatientDiagnosisCore
  source_type: derived
```

**After**:
```yaml
apiVersion: nomnom.io/v1
kind: Entity
metadata:
  name: PatientDiagnosisCore
  labels:
    persistence: persistent
```

#### Step 2: Convert Entity Header

**Before**:
```yaml
entity:
  name: PatientDiagnosisCore
  source_type: derived
  parent: Hl7v2Message
  doc: "Patient diagnosis..."
```

**After**:
```yaml
spec:
  type: derived
  derivation:
    parent: Hl7v2Message
```

(Move `doc` to `metadata.annotations.description`)

#### Step 3: Merge Field and Overrides with camelCase

**Before**:
```yaml
fields:
  - name: facility_id
    type: String
    nullable: false
    extraction:
      copy_from_source: Filename

persistence:
  field_overrides:
    - name: facility_id
      type: String
      args: [255]
      nullable: false
      index: true
```

**After**:
```yaml
fields:
  - name: facilityId          # camelCase!
    type: string              # lowercase type
    constraints:
      maxLength: 255          # camelCase!
      nullable: false
      indexed: true           # camelCase!
    source:
      copyFrom: Filename      # camelCase!
      field: facilityId       # camelCase!
```

#### Step 4: Convert Persistence Config

**Before**:
```yaml
persistence:
  database:
    conformant_table: patient_diagnosis_conformant
    autogenerate_conformant_id: false
    unicity_fields: [facility_id, patient_identifier]
```

**After**:
```yaml
persistence:
  enabled: true
  table: patient_diagnosis_conformant
  unicity:
    fields: [facilityId, patientIdentifier]  # camelCase!
```

### Automated Migration Tool

```bash
# Convert single file
nomnom migrate entity.yaml --to v1 --output entity-v1.yaml

# Convert directory
nomnom migrate entities/ --to v1 --output entities-v1/

# Dry run (show diff)
nomnom migrate entity.yaml --to v1 --dry-run
```

---

## Validation

Entity definitions can be validated against JSON Schema:

```bash
# Validate single file
nomnom validate entity.yaml

# Validate directory
nomnom validate entities/

# Show validation errors in detail
nomnom validate entity.yaml --verbose
```

### Validation Rules

1. **Required fields**: `apiVersion`, `kind`, `metadata.name`, `spec.type`
2. **Type compatibility**: Source field types must match target field types
3. **Parent existence**: All referenced parent entities must exist
4. **Transform registry**: All transform functions must be registered
5. **Constraint consistency**: `primaryKey` implies `indexed` and `unique`
6. **Table uniqueness**: Table names must be unique across entities

---

## Tooling Support

### CLI Commands

```bash
# List entities
nomnom get entities
nomnom get entities -l domain=healthcare

# Describe entity
nomnom describe entity PatientDiagnosisCore

# Validate
nomnom validate entities/

# Migrate
nomnom migrate entities/ --to v1

# Generate code
nomnom generate worker --entities entities-v1/

# Diff entities
nomnom diff entity-old.yaml entity-new.yaml
```

### IDE Support

- **VS Code**: YAML schema validation
- **IntelliJ**: Auto-completion for field names
- **vim**: Syntax highlighting

Schema location: `https://schemas.nomnom.io/v1/entity.json`

---

## Future Enhancements (v2+)

Potential features for future versions:

1. **Computed fields**: Server-side computed columns
2. **Relationships**: Explicit foreign key relationships
3. **Triggers**: Event-driven actions on entity changes
4. **Versioning**: Schema evolution tracking
5. **Partitioning**: Table partitioning strategies
6. **Soft deletes**: Logical deletion support
7. **Audit logging**: Automatic change tracking
8. **Custom validators**: User-defined validation functions

---

## References

- [Kubernetes API Conventions](https://github.com/kubernetes/community/blob/master/contributors/devel/sig-architecture/api-conventions.md)
- [OpenAPI Specification](https://swagger.io/specification/)
- [JSON Schema](https://json-schema.org/)

---

## Changelog

### v1 (Draft)

- Initial specification
- K8s-inspired structure
- Unified field definitions with camelCase naming
- Rich constraint system
- Eliminates field_overrides pattern
