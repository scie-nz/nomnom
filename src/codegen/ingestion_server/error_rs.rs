/// Generate error.rs for error handling

use std::path::Path;
use std::error::Error;
use std::io::Write;

pub fn generate_error_rs(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let error_file = output_dir.join("src/error.rs");
    let mut output = std::fs::File::create(&error_file)?;

    writeln!(output, "// Auto-generated error types and handlers")?;
    writeln!(output)?;
    writeln!(output, "use axum::{{")?;
    writeln!(output, "    http::StatusCode,")?;
    writeln!(output, "    response::{{IntoResponse, Response}},")?;
    writeln!(output, "    Json,")?;
    writeln!(output, "}};")?;
    writeln!(output, "use serde_json::json;\n")?;

    writeln!(output, "#[derive(Debug)]")?;
    writeln!(output, "pub enum AppError {{")?;
    writeln!(output, "    Database(diesel::result::Error),")?;
    writeln!(output, "    Pool(r2d2::Error),")?;
    writeln!(output, "    ValidationError(String),")?;
    writeln!(output, "    InternalError(String),")?;
    writeln!(output, "    InvalidFormat(String),")?;
    writeln!(output, "    InvalidField(String),")?;
    writeln!(output, "    EmptyMessage,")?;
    writeln!(output, "    UnknownPrefix(String),")?;
    writeln!(output, "    UnknownEntity(String),")?;
    writeln!(output, "}}\n")?;

    writeln!(output, "impl IntoResponse for AppError {{")?;
    writeln!(output, "    fn into_response(self) -> Response {{")?;
    writeln!(output, "        let (status, message) = match self {{")?;
    writeln!(output, "            AppError::Database(e) => (")?;
    writeln!(output, "                StatusCode::INTERNAL_SERVER_ERROR,")?;
    writeln!(output, "                format!(\"Database error: {{}}\", e),")?;
    writeln!(output, "            ),")?;
    writeln!(output, "            AppError::Pool(e) => (")?;
    writeln!(output, "                StatusCode::SERVICE_UNAVAILABLE,")?;
    writeln!(output, "                format!(\"Database pool error: {{}}\", e),")?;
    writeln!(output, "            ),")?;
    writeln!(output, "            AppError::ValidationError(msg) => (")?;
    writeln!(output, "                StatusCode::BAD_REQUEST,")?;
    writeln!(output, "                format!(\"Validation error: {{}}\", msg),")?;
    writeln!(output, "            ),")?;
    writeln!(output, "            AppError::InternalError(msg) => (")?;
    writeln!(output, "                StatusCode::INTERNAL_SERVER_ERROR,")?;
    writeln!(output, "                format!(\"Internal error: {{}}\", msg),")?;
    writeln!(output, "            ),")?;
    writeln!(output, "            AppError::InvalidFormat(msg) => (")?;
    writeln!(output, "                StatusCode::BAD_REQUEST,")?;
    writeln!(output, "                format!(\"Invalid message format: {{}}\", msg),")?;
    writeln!(output, "            ),")?;
    writeln!(output, "            AppError::InvalidField(field) => (")?;
    writeln!(output, "                StatusCode::BAD_REQUEST,")?;
    writeln!(output, "                format!(\"Invalid field: {{}}\", field),")?;
    writeln!(output, "            ),")?;
    writeln!(output, "            AppError::EmptyMessage => (")?;
    writeln!(output, "                StatusCode::BAD_REQUEST,")?;
    writeln!(output, "                \"Empty message\".to_string(),")?;
    writeln!(output, "            ),")?;
    writeln!(output, "            AppError::UnknownPrefix(prefix) => (")?;
    writeln!(output, "                StatusCode::BAD_REQUEST,")?;
    writeln!(output, "                format!(\"Unknown message prefix: {{}}\", prefix),")?;
    writeln!(output, "            ),")?;
    writeln!(output, "            AppError::UnknownEntity(entity) => (")?;
    writeln!(output, "                StatusCode::BAD_REQUEST,")?;
    writeln!(output, "                format!(\"Unknown entity: {{}}\", entity),")?;
    writeln!(output, "            ),")?;
    writeln!(output, "        }};\n")?;

    writeln!(output, "        (status, Json(json!({{ \"error\": message }}))).into_response()")?;
    writeln!(output, "    }}")?;
    writeln!(output, "}}\n")?;

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
