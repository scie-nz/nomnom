//! Filesystem utilities for code generation

use std::fs;
use std::io::{self};
use std::path::Path;

/// Write content to a file, creating parent directories if needed
pub fn write_file<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> io::Result<()> {
    let path = path.as_ref();

    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write the file
    fs::write(path, contents)
}

/// Write content to a file using File::create, creating parent directories if needed
pub fn create_file<P: AsRef<Path>>(path: P) -> io::Result<fs::File> {
    let path = path.as_ref();

    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Create the file
    fs::File::create(path)
}
