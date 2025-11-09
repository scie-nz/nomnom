/// Generate models.rs for request/response types

use std::path::Path;
use std::error::Error;
use std::io::Write;

pub fn generate_models_rs(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let models_file = output_dir.join("src/models.rs");
    let mut output = std::fs::File::create(&models_file)?;

    writeln!(output, "// Auto-generated models")?;
    writeln!(output, "// Worker-specific models can be added here if needed")?;
    writeln!(output)?;

    Ok(())
}
