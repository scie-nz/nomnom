/// Generate transforms.rs with helper functions for derived entity field extraction

use std::path::Path;
use std::error::Error;
use std::io::Write;
use std::collections::HashMap;
use crate::codegen::project_config::RustTransformDef;

pub fn generate_transforms_rs(
    output_dir: &Path,
    transforms: Option<&HashMap<String, RustTransformDef>>,
) -> Result<(), Box<dyn Error>> {
    let transforms_file = output_dir.join("src/transforms.rs");
    let mut output = std::fs::File::create(&transforms_file)?;

    writeln!(output, "// Auto-generated transform functions")?;
    writeln!(output, "// Generated from nomnom.yaml transforms configuration\n")?;

    // Generate custom transforms from nomnom.yaml
    if let Some(transforms) = transforms {
        generate_custom_transforms(&mut output, transforms)?;
    } else {
        // If no transforms provided, generate an empty module
        writeln!(output, "// No custom transforms defined in nomnom.yaml")?;
    }

    Ok(())
}

/// Generate custom transform functions from nomnom.yaml
fn generate_custom_transforms(
    output: &mut std::fs::File,
    transforms: &HashMap<String, RustTransformDef>,
) -> Result<(), Box<dyn Error>> {
    writeln!(output, "// Custom transform functions from nomnom.yaml\n")?;

    // Collect all unique imports
    let mut all_imports = std::collections::HashSet::new();
    for transform in transforms.values() {
        for import in &transform.imports {
            all_imports.insert(import.clone());
        }
    }

    // Generate imports
    let has_imports = !all_imports.is_empty();
    for import in all_imports {
        writeln!(output, "use {};", import)?;
    }
    if has_imports {
        writeln!(output)?;
    }

    // Generate each transform function
    for (name, transform) in transforms {
        // Generate documentation
        if let Some(ref doc) = transform.doc {
            writeln!(output, "/// {}", doc)?;
        }

        // Generate function signature
        write!(output, "pub fn {}(", name)?;
        for (i, arg) in transform.args.iter().enumerate() {
            if i > 0 {
                write!(output, ", ")?;
            }
            write!(output, "{}: {}", arg.name, arg.arg_type)?;
        }
        writeln!(output, ") -> {} {{", transform.return_type)?;

        // Generate function body (indent each line)
        for line in transform.code.lines() {
            writeln!(output, "    {}", line)?;
        }

        writeln!(output, "}}\n")?;
    }

    Ok(())
}
