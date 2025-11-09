/// Generate error.rs for error handling

use std::path::Path;
use std::error::Error;
use std::io::Write;

pub fn generate_error_rs(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let error_file = output_dir.join("src/error.rs");
    let mut output = std::fs::File::create(&error_file)?;

    writeln!(output, "// Auto-generated error types and handlers")?;
    writeln!(output)?;
    writeln!(output, "use std::fmt;\n")?;

    writeln!(output, "#[derive(Debug)]")?;
    writeln!(output, "pub enum AppError {{")?;
    writeln!(output, "    Database(diesel::result::Error),")?;
    writeln!(output, "    Pool(r2d2::Error),")?;
    writeln!(output, "    ValidationError(String),")?;
    writeln!(output, "    ParseError(String),")?;
    writeln!(output, "    InvalidFormat(String),")?;
    writeln!(output, "    InvalidField(String),")?;
    writeln!(output, "    EmptyMessage,")?;
    writeln!(output, "    UnknownPrefix(String),")?;
    writeln!(output, "    UnknownEntity(String),")?;
    writeln!(output, "}}\n")?;

    writeln!(output, "impl fmt::Display for AppError {{")?;
    writeln!(output, "    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {{")?;
    writeln!(output, "        match self {{")?;
    writeln!(output, "            AppError::Database(e) => write!(f, \"Database error: {{}}\", e),")?;
    writeln!(output, "            AppError::Pool(e) => write!(f, \"Database pool error: {{}}\", e),")?;
    writeln!(output, "            AppError::ValidationError(msg) => write!(f, \"Validation error: {{}}\", msg),")?;
    writeln!(output, "            AppError::ParseError(msg) => write!(f, \"Parse error: {{}}\", msg),")?;
    writeln!(output, "            AppError::InvalidFormat(msg) => write!(f, \"Invalid format: {{}}\", msg),")?;
    writeln!(output, "            AppError::InvalidField(field) => write!(f, \"Invalid or missing field: {{}}\", field),")?;
    writeln!(output, "            AppError::EmptyMessage => write!(f, \"Empty message\"),")?;
    writeln!(output, "            AppError::UnknownPrefix(prefix) => write!(f, \"Unknown message prefix: {{}}\", prefix),")?;
    writeln!(output, "            AppError::UnknownEntity(entity) => write!(f, \"Unknown entity: {{}}\", entity),")?;
    writeln!(output, "        }}")?;
    writeln!(output, "    }}")?;
    writeln!(output, "}}\n")?;

    writeln!(output, "impl std::error::Error for AppError {{}}\n")?;

    writeln!(output, "impl From<diesel::result::Error> for AppError {{")?;
    writeln!(output, "    fn from(e: diesel::result::Error) -> Self {{")?;
    writeln!(output, "        AppError::Database(e)")?;
    writeln!(output, "    }}")?;
    writeln!(output, "}}\n")?;

    writeln!(output, "impl From<r2d2::Error> for AppError {{")?;
    writeln!(output, "    fn from(e: r2d2::Error) -> Self {{")?;
    writeln!(output, "        AppError::Pool(e)")?;
    writeln!(output, "    }}")?;
    writeln!(output, "}}")?;

    Ok(())
}
