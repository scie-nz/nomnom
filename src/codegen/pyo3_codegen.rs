//! PyO3 Python bindings code generation.
//!
//! This module generates Python wrappers for Rust entity cores using PyO3,
//! enabling seamless interop between Rust and Python code.
//!
//! This is a generic implementation that can be configured for different
//! domain-specific needs (HL7, EDI, CSV, JSON, etc.).

use crate::codegen::types::{EntityDef, FieldDef};
use crate::codegen::utils::to_snake_case;
use std::io::Write;

/// Configuration for PyO3 code generation
#[derive(Clone)]
pub struct PyO3Config {
    /// Path to transform registry type (e.g., "crate::python_transform::PyTransformRegistry")
    pub transform_registry_type: String,

    /// Path to FieldValue enum (e.g., "crate::entity::FieldValue")
    pub field_value_type: String,

    /// Whether to generate database reconstruction constructors
    pub generate_database_constructors: bool,

    /// Additional imports to include in generated file
    pub additional_imports: Vec<String>,
}

impl Default for PyO3Config {
    fn default() -> Self {
        Self {
            transform_registry_type: "crate::transform_registry::TransformRegistry".to_string(),
            field_value_type: "crate::entity::FieldValue".to_string(),
            generate_database_constructors: false,
            additional_imports: vec![],
        }
    }
}

/// Generate Python bindings for all entities
pub fn generate_python_bindings<W: Write>(
    writer: &mut W,
    all_entities: &[EntityDef],
    config: &PyO3Config,
) -> Result<(), std::io::Error> {
    // Header
    writeln!(writer, "// Auto-generated Python bindings and module registration")?;
    writeln!(writer, "// DO NOT EDIT - regenerate with build script\n")?;
    writeln!(writer, "use pyo3::prelude::*;")?;
    writeln!(writer, "use crate::generated::*;\n")?;

    // Additional imports
    for import in &config.additional_imports {
        writeln!(writer, "use {};", import)?;
    }
    writeln!(writer)?;

    // Separate entities by type
    // Include all non-abstract entities (be inclusive rather than restrictive)
    let root_entities: Vec<&EntityDef> = all_entities.iter()
        .filter(|e| e.source_type == "root" && !e.is_abstract)
        .collect();

    // For derived entities, include any entity that's not root and not abstract
    // This handles derived, transient, and permanent entities uniformly
    let derived_entities: Vec<&EntityDef> = all_entities.iter()
        .filter(|e| e.source_type != "root" && !e.is_abstract && !e.fields.is_empty())
        .collect();

    // Generate wrappers for root entities
    println!("Generating Python wrappers for {} root entities", root_entities.len());
    for entity in root_entities {
        println!("Generating Python wrapper for root entity: {}", entity.name);
        generate_root_python_wrapper(writer, entity, config)?;
    }

    // Generate wrappers for derived entities
    println!("Generating Python wrappers for {} derived entities", derived_entities.len());
    for entity in derived_entities {
        println!("Generating Python wrapper for derived entity: {}", entity.name);
        if entity.repeated_for.is_some() {
            generate_repeated_for_wrapper(writer, entity, config)?;
        } else {
            generate_derived_python_wrapper(writer, entity, config)?;
        }
    }

    // Generate module registration function
    generate_module_registration(writer, all_entities)?;

    Ok(())
}

/// Generate Python wrapper for a root entity
fn generate_root_python_wrapper<W: Write>(
    writer: &mut W,
    entity: &EntityDef,
    _config: &PyO3Config,
) -> Result<(), std::io::Error> {
    // Entity names in YAML are already PascalCase, use as-is
    let core_name = format!("{}Core", entity.name);
    let py_class_name = format!("Py{}", core_name);
    let python_name = entity.name.clone();

    // Generate PyClass wrapper struct
    writeln!(writer, "#[pyclass(name = \"{}\")]", python_name)?;
    writeln!(writer, "#[derive(Clone)]")?;
    writeln!(writer, "pub struct {} {{", py_class_name)?;
    writeln!(writer, "    inner: {},", core_name)?;
    writeln!(writer, "}}\n")?;

    // Generate Python methods
    writeln!(writer, "#[pymethods]")?;
    writeln!(writer, "impl {} {{", py_class_name)?;

    // from_string static method
    writeln!(writer, "    #[staticmethod]")?;
    writeln!(writer, "    fn from_string(raw_input: &str) -> PyResult<Self> {{")?;
    writeln!(writer, "        // Use singleton transform registry (lazy_static or once_cell)")?;
    writeln!(writer, "        // No registry needed - transforms are injected directly")?;
    writeln!(writer, "        let inner = {}::from_string(raw_input)", core_name)?;
    writeln!(writer, "            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))?;")?;
    writeln!(writer, "        Ok(Self {{ inner }})")?;
    writeln!(writer, "    }}\n")?;

    // to_dict method
    writeln!(writer, "    fn to_dict(&self) -> PyResult<PyObject> {{")?;
    writeln!(writer, "        Python::with_gil(|py| {{")?;
    writeln!(writer, "            let json_str = serde_json::to_string(&self.inner)")?;
    writeln!(writer, "                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;")?;
    writeln!(writer, "            let json_mod = py.import(\"json\")?;")?;
    writeln!(writer, "            let dict = json_mod.call_method1(\"loads\", (json_str,))?;")?;
    writeln!(writer, "            Ok(dict.to_object(py))")?;
    writeln!(writer, "        }})")?;
    writeln!(writer, "    }}\n")?;

    // __getattr__ for field access
    writeln!(writer, "    fn __getattr__(&self, py: Python, name: &str) -> PyResult<PyObject> {{")?;
    writeln!(writer, "        let dict = self.to_dict()?;")?;
    writeln!(writer, "        let dict_ref = dict.as_ref(py);")?;
    writeln!(writer, "        ")?;
    writeln!(writer, "        if let Ok(value) = dict_ref.get_item(name) {{")?;
    writeln!(writer, "            Ok(value.to_object(py))")?;
    writeln!(writer, "        }} else {{")?;
    writeln!(writer, "            Err(PyErr::new::<pyo3::exceptions::PyAttributeError, _>(")?;
    writeln!(writer, "                format!(\"'{}' has no attribute '{{}}'\", name)", python_name)?;
    writeln!(writer, "            ))")?;
    writeln!(writer, "        }}")?;
    writeln!(writer, "    }}\n")?;

    // __repr__
    writeln!(writer, "    fn __repr__(&self) -> String {{")?;
    writeln!(writer, "        format!(\"{}(...)\")", python_name)?;
    writeln!(writer, "    }}")?;

    writeln!(writer, "}}\n")?;

    Ok(())
}

/// Generate Python wrapper for a derived entity
fn generate_derived_python_wrapper<W: Write>(
    writer: &mut W,
    entity: &EntityDef,
    config: &PyO3Config,
) -> Result<(), std::io::Error> {
    // Entity names in YAML are already PascalCase, use as-is
    let core_name = format!("{}Core", entity.name);
    let py_class_name = format!("Py{}", core_name);

    // Get parent information
    let parents = entity.get_parents();
    let is_single_parent = parents.len() == 1;

    // Generate PyClass wrapper struct
    writeln!(writer, "#[pyclass(name = \"{}\")]", core_name)?;
    writeln!(writer, "#[derive(Clone)]")?;
    writeln!(writer, "pub struct {} {{", py_class_name)?;
    writeln!(writer, "    inner: {},", core_name)?;
    writeln!(writer, "}}\n")?;

    // Generate Python methods
    writeln!(writer, "#[pymethods]")?;
    writeln!(writer, "impl {} {{", py_class_name)?;

    // Generate factory method
    if is_single_parent {
        generate_single_parent_constructor(writer, entity, &core_name, &parents[0], config)?;
    } else {
        generate_multi_parent_constructor(writer, entity, &core_name, &parents, config)?;
    }

    // Generate database reconstruction constructor if configured
    if config.generate_database_constructors && entity.database.is_some() {
        generate_database_constructor(writer, entity, &core_name)?;
        generate_field_getters(writer, entity)?;
    }

    // Common methods
    generate_common_methods(writer, &core_name, &config.field_value_type)?;

    writeln!(writer, "}}\n")?;

    Ok(())
}

/// Generate Python wrapper for a repeated_for entity
fn generate_repeated_for_wrapper<W: Write>(
    writer: &mut W,
    entity: &EntityDef,
    config: &PyO3Config,
) -> Result<(), std::io::Error> {
    // Entity names in YAML are already PascalCase, use as-is
    let core_name = format!("{}Core", entity.name);
    let py_class_name = format!("Py{}", core_name);

    let repeated_for = entity.repeated_for.as_ref().unwrap();
    let parent_entity = &repeated_for.entity;
    let parent_snake = to_snake_case(parent_entity);
    let parent_py_class = format!("Py{}Core", parent_entity);

    // Generate PyClass wrapper struct
    writeln!(writer, "#[pyclass(name = \"{}\")]", core_name)?;
    writeln!(writer, "#[derive(Clone)]")?;
    writeln!(writer, "pub struct {} {{", py_class_name)?;
    writeln!(writer, "    inner: {},", core_name)?;
    writeln!(writer, "}}\n")?;

    // Generate Python methods
    writeln!(writer, "#[pymethods]")?;
    writeln!(writer, "impl {} {{", py_class_name)?;

    // Generate field accessors (properties)
    for field in &entity.fields {
        let rust_type = map_field_type(&field.field_type, field.nullable);
        writeln!(writer, "    #[getter]")?;
        writeln!(writer, "    fn {}(&self) -> {} {{", field.name, rust_type)?;
        writeln!(writer, "        self.inner.{}.clone()", field.name)?;
        writeln!(writer, "    }}\n")?;
    }

    // from_parent_repeated static method
    writeln!(writer, "    /// Create multiple {} instances from parent entity's list field.", entity.name)?;
    writeln!(writer, "    #[staticmethod]")?;
    writeln!(writer, "    fn from_parent_repeated({}: &{}) -> PyResult<Vec<Self>> {{",
             parent_snake, parent_py_class)?;
    writeln!(writer, "        // Use singleton transform registry (lazy_static or once_cell)")?;
    writeln!(writer, "        use once_cell::sync::Lazy;")?;
    writeln!(writer, "        // No registry needed - transforms are injected directly")?;
    writeln!(writer, "        let instances = {}::from_parent_repeated(&{}.inner)",
             core_name, parent_snake)?;
    writeln!(writer, "            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(\"{{:?}}\", e)))?;")?;
    writeln!(writer, "        Ok(instances.into_iter().map(|inner| Self {{ inner }}).collect())")?;
    writeln!(writer, "    }}\n")?;

    // Common methods
    generate_common_methods(writer, &core_name, &config.field_value_type)?;

    writeln!(writer, "}}\n")?;

    Ok(())
}

/// Generate constructor for single-parent derived entity
fn generate_single_parent_constructor<W: Write>(
    writer: &mut W,
    _entity: &EntityDef,
    core_name: &str,
    parent_name: &str,
    _config: &PyO3Config,
) -> Result<(), std::io::Error> {
    let parent_snake = to_snake_case(parent_name);
    let parent_py_class = format!("Py{}Core", parent_name);
    // Always use from_sources for consistency (even for single parent)
    let method_name = "from_sources";

    writeln!(writer, "    #[staticmethod]")?;
    writeln!(writer, "    fn {}({}: &{}) -> PyResult<Self> {{",
             method_name, parent_snake, parent_py_class)?;
    writeln!(writer, "        // No registry needed - transforms are injected directly")?;
    writeln!(writer, "        let inner = {}::{}(&{}.inner)",
             core_name, method_name, parent_snake)?;
    writeln!(writer, "            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(\"{{:?}}\", e)))?;")?;
    writeln!(writer, "        Ok(Self {{ inner }})")?;
    writeln!(writer, "    }}\n")?;

    Ok(())
}

/// Generate constructor for multi-parent derived entity
fn generate_multi_parent_constructor<W: Write>(
    writer: &mut W,
    _entity: &EntityDef,
    core_name: &str,
    parents: &[String],
    _config: &PyO3Config,
) -> Result<(), std::io::Error> {
    let mut source_params = Vec::new();
    let mut source_args = Vec::new();

    for parent_name in parents {
        let param_name = to_snake_case(parent_name);
        let py_type = format!("Py{}Core", parent_name);
        source_params.push((param_name.clone(), py_type));
        source_args.push(format!("{}.inner", param_name));
    }

    writeln!(writer, "    #[staticmethod]")?;
    writeln!(writer, "    fn from_sources(")?;
    for (param_name, py_type) in &source_params {
        writeln!(writer, "        {}: &{},", param_name, py_type)?;
    }
    writeln!(writer, "    ) -> PyResult<Self> {{")?;
    writeln!(writer, "        // Use singleton transform registry (lazy_static or once_cell)")?;
    writeln!(writer, "        // No registry needed - transforms are injected directly")?;

    let args_str = source_args.iter()
        .map(|arg| format!("&{}", arg))
        .collect::<Vec<_>>()
        .join(", ");

    writeln!(writer, "        let inner = {}::from_sources({})",
             core_name, args_str)?;
    writeln!(writer, "            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(\"{{:?}}\", e)))?;")?;
    writeln!(writer, "        Ok(Self {{ inner }})")?;
    writeln!(writer, "    }}\n")?;

    Ok(())
}

/// Generate database reconstruction constructor (#[new])
fn generate_database_constructor<W: Write>(
    writer: &mut W,
    entity: &EntityDef,
    core_name: &str,
) -> Result<(), std::io::Error> {
    let db_config = entity.database.as_ref().unwrap();
    let autogenerate_pk = db_config.autogenerate_conformant_id;

    // Filter out primary key fields if autogenerated
    let constructor_fields: Vec<&FieldDef> = entity.fields.iter()
        .filter(|f| !f.primary_key || !autogenerate_pk)
        .collect();

    writeln!(writer, "    /// Create {} from field values (for reconstruction from database).", core_name)?;
    writeln!(writer, "    #[new]")?;
    writeln!(writer, "    #[pyo3(signature = (")?;

    for (i, field) in constructor_fields.iter().enumerate() {
        if i == constructor_fields.len() - 1 {
            writeln!(writer, "        {} = None", field.name)?;
        } else {
            writeln!(writer, "        {} = None,", field.name)?;
        }
    }
    writeln!(writer, "    ))]")?;

    writeln!(writer, "    fn new(")?;
    for field in &constructor_fields {
        writeln!(writer, "        {}: Option<String>,", field.name)?;
    }
    writeln!(writer, "    ) -> Self {{")?;

    writeln!(writer, "        Self {{")?;
    writeln!(writer, "            inner: {} {{", core_name)?;
    for field in &constructor_fields {
        writeln!(writer, "                {},", field.name)?;
    }
    writeln!(writer, "            }}")?;
    writeln!(writer, "        }}")?;
    writeln!(writer, "    }}\n")?;

    Ok(())
}

/// Generate field getter methods
fn generate_field_getters<W: Write>(
    writer: &mut W,
    entity: &EntityDef,
) -> Result<(), std::io::Error> {
    for field in &entity.fields {
        writeln!(writer, "    /// Get the {} field", field.name)?;
        writeln!(writer, "    #[getter]")?;
        writeln!(writer, "    fn {}(&self) -> Option<String> {{", field.name)?;
        writeln!(writer, "        self.inner.{}.clone()", field.name)?;
        writeln!(writer, "    }}\n")?;
    }
    Ok(())
}

/// Generate common methods (to_dict, to_json, __getattr__, __repr__)
fn generate_common_methods<W: Write>(
    writer: &mut W,
    core_name: &str,
    _field_value_type: &str,
) -> Result<(), std::io::Error> {
    // to_dict method - uses serde_json::Value
    writeln!(writer, "    fn to_dict(&self, py: Python) -> PyResult<PyObject> {{")?;
    writeln!(writer, "        let dict = pyo3::types::PyDict::new(py);")?;
    writeln!(writer, "        for (key, value) in self.inner.to_dict() {{")?;
    writeln!(writer, "            let py_value = match value {{")?;
    writeln!(writer, "                serde_json::Value::String(s) => s.into_py(py),")?;
    writeln!(writer, "                serde_json::Value::Number(n) => {{")?;
    writeln!(writer, "                    if let Some(i) = n.as_i64() {{")?;
    writeln!(writer, "                        i.into_py(py)")?;
    writeln!(writer, "                    }} else if let Some(f) = n.as_f64() {{")?;
    writeln!(writer, "                        f.into_py(py)")?;
    writeln!(writer, "                    }} else {{")?;
    writeln!(writer, "                        n.to_string().into_py(py)")?;
    writeln!(writer, "                    }}")?;
    writeln!(writer, "                }},")?;
    writeln!(writer, "                serde_json::Value::Bool(b) => b.into_py(py),")?;
    writeln!(writer, "                serde_json::Value::Array(arr) => {{")?;
    writeln!(writer, "                    let list: Vec<String> = arr.iter()")?;
    writeln!(writer, "                        .map(|v| v.as_str().unwrap_or(\"\").to_string())")?;
    writeln!(writer, "                        .collect();")?;
    writeln!(writer, "                    list.into_py(py)")?;
    writeln!(writer, "                }},")?;
    writeln!(writer, "                serde_json::Value::Null => py.None(),")?;
    writeln!(writer, "                serde_json::Value::Object(_) => py.None(), // Skip nested objects")?;
    writeln!(writer, "            }};")?;
    writeln!(writer, "            dict.set_item(key, py_value)?;")?;
    writeln!(writer, "        }}")?;
    writeln!(writer, "        Ok(dict.into())")?;
    writeln!(writer, "    }}\n")?;

    // to_json method
    writeln!(writer, "    fn to_json(&self) -> PyResult<String> {{")?;
    writeln!(writer, "        self.inner.to_json()")?;
    writeln!(writer, "            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(\"JSON error: {{:?}}\", e)))")?;
    writeln!(writer, "    }}\n")?;

    // __getattr__ method - uses serde_json::Value
    writeln!(writer, "    fn __getattr__(&self, py: Python, name: &str) -> PyResult<PyObject> {{")?;
    writeln!(writer, "        let dict = self.inner.to_dict();")?;
    writeln!(writer, "        match dict.get(name) {{")?;
    writeln!(writer, "            Some(value) => {{")?;
    writeln!(writer, "                let py_value = match value {{")?;
    writeln!(writer, "                    serde_json::Value::String(s) => s.clone().into_py(py),")?;
    writeln!(writer, "                    serde_json::Value::Number(n) => {{")?;
    writeln!(writer, "                        if let Some(i) = n.as_i64() {{")?;
    writeln!(writer, "                            i.into_py(py)")?;
    writeln!(writer, "                        }} else if let Some(f) = n.as_f64() {{")?;
    writeln!(writer, "                            f.into_py(py)")?;
    writeln!(writer, "                        }} else {{")?;
    writeln!(writer, "                            n.to_string().into_py(py)")?;
    writeln!(writer, "                        }}")?;
    writeln!(writer, "                    }},")?;
    writeln!(writer, "                    serde_json::Value::Bool(b) => b.into_py(py),")?;
    writeln!(writer, "                    serde_json::Value::Array(arr) => {{")?;
    writeln!(writer, "                        let list: Vec<String> = arr.iter()")?;
    writeln!(writer, "                            .map(|v| v.as_str().unwrap_or(\"\").to_string())")?;
    writeln!(writer, "                            .collect();")?;
    writeln!(writer, "                        list.into_py(py)")?;
    writeln!(writer, "                    }},")?;
    writeln!(writer, "                    serde_json::Value::Null => py.None(),")?;
    writeln!(writer, "                    serde_json::Value::Object(_) => py.None(), // Skip nested objects")?;
    writeln!(writer, "                }};")?;
    writeln!(writer, "                Ok(py_value)")?;
    writeln!(writer, "            }}")?;
    writeln!(writer, "            None => Err(PyErr::new::<pyo3::exceptions::PyAttributeError, _>(")?;
    writeln!(writer, "                format!(\"'{}' object has no attribute '{{}}'\", name)", core_name)?;
    writeln!(writer, "            ))")?;
    writeln!(writer, "        }}")?;
    writeln!(writer, "    }}\n")?;

    // __repr__ method
    writeln!(writer, "    fn __repr__(&self) -> String {{")?;
    writeln!(writer, "        format!(\"{}({{:?}})\", self.inner.to_dict())", core_name)?;
    writeln!(writer, "    }}")?;

    Ok(())
}

/// Generate module registration function
fn generate_module_registration<W: Write>(
    writer: &mut W,
    all_entities: &[EntityDef],
) -> Result<(), std::io::Error> {
    writeln!(writer, "/// Register all Rust entities with the Python module")?;
    writeln!(writer, "pub fn register_all_entities(m: &PyModule) -> PyResult<()> {{")?;

    for entity in all_entities {
        if entity.is_abstract {
            continue;
        }
        // Entity names in YAML are already PascalCase, use as-is
        let py_class_name = format!("Py{}Core", entity.name);
        writeln!(writer, "    m.add_class::<{}>()?;", py_class_name)?;
    }

    writeln!(writer, "    Ok(())")?;
    writeln!(writer, "}}\n")?;

    Ok(())
}

/// Map field type to Rust type
fn map_field_type(field_type: &str, nullable: bool) -> String {
    let base_type = match field_type {
        "String" => "String",
        "Int" | "Integer" => "i64",
        "Float" | "Double" => "f64",
        "Bool" | "Boolean" => "bool",
        "DateTime" | "Date" => "String",
        "List[String]" => "Vec<String>",
        "List[Object]" | "List[Json]" => "Vec<serde_json::Value>",
        "Object" | "Json" => "serde_json::Value",
        _ => "String",
    }.to_string();

    if nullable && field_type != "List[String]" && field_type != "List[Object]" && field_type != "List[Json]" {
        format!("Option<{}>", base_type)
    } else {
        base_type
    }
}
