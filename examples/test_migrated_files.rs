// Test program to verify migrated v1 entity files can be loaded
// Run with: cargo run --example test_migrated_files

use nomnom::codegen::yaml_loader::load_entities;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing migrated Entity Schema v1 files\n");

    // Test loading all migrated v1 entities
    let entities_dir = "/home/bogdan/ingestion/hl7-nomnom-parser/entities-v1";

    println!("Loading entities from: {}", entities_dir);
    let entities = load_entities(entities_dir)?;

    println!("✓ Successfully loaded {} entities\n", entities.len());

    // Categorize and display
    let mut transient = vec![];
    let mut persistent = vec![];
    let mut repeating = vec![];

    for entity in &entities {
        if entity.is_persistent() {
            persistent.push(&entity.name);
        } else {
            transient.push(&entity.name);
        }

        if entity.repetition.as_deref() == Some("repeated") {
            repeating.push(&entity.name);
        }
    }

    println!("Transient entities ({}):", transient.len());
    for name in &transient {
        println!("  - {}", name);
    }

    println!("\nPersistent entities ({}):", persistent.len());
    for name in &persistent {
        println!("  - {}", name);
    }

    println!("\nRepeating entities ({}):", repeating.len());
    for name in &repeating {
        println!("  - {}", name);
    }

    // Verify a few specific entities
    println!("\n=== Detailed Verification ===");

    if let Some(diagnosis) = entities.iter().find(|e| e.name == "Diagnosis") {
        println!("\n✓ Diagnosis entity:");
        println!("  Source type: {}", diagnosis.source_type);
        println!("  Repetition: {:?}", diagnosis.repetition);
        println!("  Fields: {}", diagnosis.fields.len());
        if let Some(ref rf) = diagnosis.repeated_for {
            println!("  Repeated for: {}.{}", rf.entity, rf.field);
        }
    }

    if let Some(patient_diag) = entities.iter().find(|e| e.name == "PatientDiagnosisCore") {
        println!("\n✓ PatientDiagnosisCore entity:");
        println!("  Source type: {}", patient_diag.source_type);
        println!("  Parents: {}", patient_diag.parents.len());
        println!("  Fields: {}", patient_diag.fields.len());
        if let Some(ref db) = patient_diag.database {
            println!("  Table: {}", db.conformant_table);
            println!("  Unicity fields: {:?}", db.unicity_fields);
        }
    }

    if let Some(filename) = entities.iter().find(|e| e.name == "Filename") {
        println!("\n✓ Filename entity:");
        println!("  Source type: {}", filename.source_type);
        println!("  Fields: {}", filename.fields.len());
        // Check for a sample field
        if let Some(field) = filename.fields.iter().find(|f| f.name == "facilityId") {
            println!("  Has facilityId field: ✓ (type: {})", field.field_type);
        }
    }

    println!("\n=== All migrated files validated successfully! ===");

    Ok(())
}
