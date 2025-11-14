// Test program to verify Entity Schema v1 deserialization and conversion
// Run with: cargo run --example test_v1_schema

use nomnom::codegen::types::{EntityV1, FieldDefV1};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Entity Schema v1 deserialization and conversion\n");

    // Test 1: Transient repeating entity (Diagnosis)
    println!("=== Test 1: Transient Repeating Entity (Diagnosis) ===");
    let diagnosis_yaml = fs::read_to_string("examples/diagnosis-v1.yaml")?;
    let diagnosis_v1: EntityV1 = serde_yaml::from_str(&diagnosis_yaml)?;

    println!("✓ Successfully deserialized diagnosis-v1.yaml");
    println!("  Name: {}", diagnosis_v1.metadata.name);
    println!("  Type: {}", diagnosis_v1.spec.entity_type);
    println!("  Repetition: {:?}", diagnosis_v1.spec.repetition);
    println!("  Labels: {:?}", diagnosis_v1.metadata.labels);
    println!("  Fields: {}", diagnosis_v1.spec.fields.len());

    // Convert to legacy format
    let diagnosis_legacy = diagnosis_v1.to_legacy();
    println!("\n✓ Successfully converted to legacy EntityDef");
    println!("  Name: {}", diagnosis_legacy.name);
    println!("  Source type: {}", diagnosis_legacy.source_type);
    println!("  Repetition: {:?}", diagnosis_legacy.repetition);
    println!("  Fields: {}", diagnosis_legacy.fields.len());

    if let Some(ref repeated_for) = diagnosis_legacy.repeated_for {
        println!("  Repeated for:");
        println!("    Entity: {}", repeated_for.entity);
        println!("    Field: {}", repeated_for.field);
        println!("    Item name: {}", repeated_for.each_known_as);
    }

    // Verify field conversion
    println!("\n  Sample field (diagnosisCode):");
    if let Some(field) = diagnosis_legacy.fields.iter().find(|f| f.name == "diagnosisCode") {
        println!("    Name: {}", field.name);
        println!("    Type: {}", field.field_type);
        println!("    Nullable: {}", field.nullable);
        println!("    Has computed_from: {}", field.computed_from.is_some());
        if let Some(ref computed) = field.computed_from {
            println!("    Transform: {}", computed.transform);
            println!("    Sources: {:?}", computed.sources);
        }
    }

    // Test 2: Persistent derived entity (PatientDiagnosisCore)
    println!("\n=== Test 2: Persistent Derived Entity (PatientDiagnosisCore) ===");
    let patient_diag_yaml = fs::read_to_string("examples/patient-diagnosis-core-v1.yaml")?;
    let patient_diag_v1: EntityV1 = serde_yaml::from_str(&patient_diag_yaml)?;

    println!("✓ Successfully deserialized patient-diagnosis-core-v1.yaml");
    println!("  Name: {}", patient_diag_v1.metadata.name);
    println!("  Type: {}", patient_diag_v1.spec.entity_type);
    println!("  Labels: {:?}", patient_diag_v1.metadata.labels);
    println!("  Fields: {}", patient_diag_v1.spec.fields.len());

    if let Some(ref persistence) = patient_diag_v1.spec.persistence {
        println!("  Persistence:");
        println!("    Enabled: {}", persistence.enabled);
        println!("    Table: {}", persistence.table);
        println!("    Indexes: {}", persistence.indexes.len());
        if let Some(ref unicity) = persistence.unicity {
            println!("    Unicity fields: {:?}", unicity.fields);
        }
    }

    // Convert to legacy format
    let patient_diag_legacy = patient_diag_v1.to_legacy();
    println!("\n✓ Successfully converted to legacy EntityDef");
    println!("  Name: {}", patient_diag_legacy.name);
    println!("  Source type: {}", patient_diag_legacy.source_type);
    println!("  Fields: {}", patient_diag_legacy.fields.len());
    println!("  Parents: {}", patient_diag_legacy.parents.len());

    for parent in &patient_diag_legacy.parents {
        println!("    - {} ({})", parent.name, parent.parent_type);
    }

    if let Some(ref db_config) = patient_diag_legacy.database {
        println!("  Database:");
        println!("    Table: {}", db_config.conformant_table);
        println!("    Unicity fields: {:?}", db_config.unicity_fields);
    }

    // Verify copyFrom field conversion
    println!("\n  Sample field (facilityId):");
    if let Some(field) = patient_diag_legacy.fields.iter().find(|f| f.name == "facilityId") {
        println!("    Name: {}", field.name);
        println!("    Type: {}", field.field_type);
        println!("    Nullable: {}", field.nullable);
        println!("    Indexed: {}", field.index);
        println!("    Has extraction: {}", field.extraction.is_some());
        if let Some(ref extraction) = field.extraction {
            println!("    Copy from source: {:?}", extraction.copy_from_source);
        }
    }

    println!("\n=== All tests passed! ===");

    Ok(())
}
