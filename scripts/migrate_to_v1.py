#!/usr/bin/env python3
"""
Migrate legacy entity YAML files to Entity Schema v1 format.

This script converts snake_case legacy format to camelCase K8s-style v1 format:
- Adds apiVersion, kind, metadata, spec structure
- Converts field names to camelCase
- Merges field_overrides into unified field definitions
- Preserves all semantic information
"""

import yaml
import sys
import os
from pathlib import Path
from typing import Any, Dict, List, Optional


def to_camel_case(snake_str: str) -> str:
    """Convert snake_case to camelCase."""
    components = snake_str.split('_')
    return components[0] + ''.join(x.title() for x in components[1:])


def convert_field_to_v1(field: Dict[str, Any], field_override: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
    """Convert a legacy field definition to v1 format."""
    v1_field = {
        'name': to_camel_case(field['name']),
        'type': field['type'].lower(),
    }

    # Build constraints from both field and override
    constraints = {}

    # Nullable from field or override
    if field_override and 'nullable' in field_override:
        constraints['nullable'] = field_override['nullable']
    elif 'nullable' in field:
        constraints['nullable'] = field['nullable']

    # Max length from override args
    if field_override and 'args' in field_override and field_override['args']:
        constraints['maxLength'] = field_override['args'][0]
    elif 'args' in field and field['args']:
        constraints['maxLength'] = field['args'][0]

    # Primary key, index from override
    if field_override:
        if field_override.get('primary_key'):
            constraints['primaryKey'] = True
        if field_override.get('index'):
            constraints['indexed'] = True

    if field.get('primary_key'):
        constraints['primaryKey'] = True
    if field.get('index'):
        constraints['indexed'] = True

    # Default value
    if 'constant' in field:
        constraints['default'] = field['constant']

    if constraints:
        v1_field['constraints'] = constraints

    # Convert source configuration
    source = {}

    # computed_from -> transform
    if 'computed_from' in field:
        cf = field['computed_from']
        source['transform'] = to_camel_case(cf['transform'])
        source['inputs'] = [s if isinstance(s, str) else s.get('source', s.get('parent', ''))
                           for s in cf.get('sources', [])]
        if 'args' in cf and cf['args']:
            if isinstance(cf['args'], list):
                source['args'] = cf['args']
            elif isinstance(cf['args'], dict):
                source['args'] = list(cf['args'].values())

    # extraction -> copyFrom
    if 'extraction' in field:
        ext = field['extraction']
        if 'copy_from_source' in ext and ext['copy_from_source']:
            source['copyFrom'] = ext['copy_from_source']
            # Infer field name from the field's name
            source['field'] = v1_field['name']

    # Constant value
    if 'constant' in field:
        source['constant'] = field['constant']

    if source:
        v1_field['source'] = source

    # Documentation
    if 'doc' in field:
        v1_field['doc'] = field['doc']

    return v1_field


def convert_entity_to_v1(legacy: Dict[str, Any]) -> Dict[str, Any]:
    """Convert a legacy entity to v1 format."""
    entity = legacy['entity']

    # Determine labels
    labels = {'domain': 'healthcare'}

    if entity.get('repetition') == 'repeated':
        labels['persistence'] = 'transient'
    elif 'persistence' in entity or 'database' in entity:
        labels['persistence'] = 'persistent'
    else:
        labels['persistence'] = 'transient'

    # Build v1 structure
    v1 = {
        'apiVersion': 'nomnom.io/v1',
        'kind': 'Entity',
        'metadata': {
            'name': entity['name'],
            'labels': labels,
        },
        'spec': {
            'type': entity.get('source_type', 'derived'),
        },
    }

    # Add annotations if doc exists
    if 'doc' in entity:
        v1['metadata']['annotations'] = {'description': entity['doc']}

    # Repetition
    if 'repetition' in entity:
        v1['spec']['repetition'] = entity['repetition']

    # Derivation
    derivation = {}

    if 'repeated_for' in entity:
        rf = entity['repeated_for']
        derivation['repeatedFor'] = {
            'entity': rf['entity'],
            'field': to_camel_case(rf['field']),
            'itemName': rf['each_known_as'],
        }

    if 'parent' in entity and entity['parent']:
        derivation['parent'] = entity['parent']

    if 'parents' in entity and entity['parents']:
        derivation['parents'] = [
            {
                'name': p['name'],
                'entity': p.get('parent_type', p.get('type', '')),
            }
            for p in entity['parents']
        ]

    if derivation:
        v1['spec']['derivation'] = derivation

    # Convert fields
    field_overrides = {}
    if 'persistence' in entity and 'field_overrides' in entity['persistence']:
        for override in entity['persistence']['field_overrides']:
            field_overrides[override['name']] = override

    v1_fields = []
    for field in entity.get('fields', []):
        override = field_overrides.get(field['name'])
        v1_field = convert_field_to_v1(field, override)
        v1_fields.append(v1_field)

    v1['spec']['fields'] = v1_fields

    # Persistence
    if 'persistence' in entity or 'database' in entity:
        db_config = entity.get('persistence', {}).get('database') or entity.get('database')

        if db_config:
            persistence = {
                'enabled': True,
                'table': db_config.get('conformant_table', ''),
            }

            # Unicity fields
            if db_config.get('unicity_fields'):
                persistence['unicity'] = {
                    'fields': [to_camel_case(f) for f in db_config['unicity_fields']]
                }

            v1['spec']['persistence'] = persistence

    return v1


def migrate_file(input_path: Path, output_path: Path):
    """Migrate a single YAML file from legacy to v1 format."""
    print(f"Migrating {input_path.name}...")

    with open(input_path, 'r') as f:
        legacy = yaml.safe_load(f)

    v1 = convert_entity_to_v1(legacy)

    with open(output_path, 'w') as f:
        yaml.dump(v1, f, default_flow_style=False, sort_keys=False, width=120)

    print(f"  ✓ Wrote {output_path}")


def main():
    if len(sys.argv) < 2:
        print("Usage: python migrate_to_v1.py <input_dir> [output_dir]")
        print("Example: python migrate_to_v1.py /home/bogdan/ingestion/hl7-nomnom-parser/entities")
        sys.exit(1)

    input_dir = Path(sys.argv[1])
    output_dir = Path(sys.argv[2]) if len(sys.argv) > 2 else input_dir / "../entities-v1"

    if not input_dir.exists():
        print(f"Error: Input directory does not exist: {input_dir}")
        sys.exit(1)

    output_dir.mkdir(exist_ok=True, parents=True)

    yaml_files = sorted(input_dir.glob("*.yaml"))

    print(f"Found {len(yaml_files)} YAML files to migrate\n")

    for yaml_file in yaml_files:
        output_file = output_dir / yaml_file.name
        try:
            migrate_file(yaml_file, output_file)
        except Exception as e:
            print(f"  ✗ Error migrating {yaml_file.name}: {e}")
            import traceback
            traceback.print_exc()

    print(f"\n✓ Migration complete! Migrated files are in {output_dir}")


if __name__ == '__main__':
    main()
