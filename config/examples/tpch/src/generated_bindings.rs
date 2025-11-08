// Auto-generated Python bindings and module registration
// DO NOT EDIT - regenerate with build script

use pyo3::prelude::*;
use crate::generated::*;


#[pyclass(name = "Order")]
#[derive(Clone)]
pub struct PyOrderCore {
    inner: OrderCore,
}

#[pymethods]
impl PyOrderCore {
    #[staticmethod]
    fn from_string(raw_input: &str) -> PyResult<Self> {
        // Use singleton transform registry (lazy_static or once_cell)
        // No registry needed - transforms are injected directly
        let inner = OrderCore::from_string(raw_input)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))?;
        Ok(Self { inner })
    }

    fn to_dict(&self) -> PyResult<PyObject> {
        Python::with_gil(|py| {
            let json_str = serde_json::to_string(&self.inner)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            let json_mod = py.import("json")?;
            let dict = json_mod.call_method1("loads", (json_str,))?;
            Ok(dict.to_object(py))
        })
    }

    fn __getattr__(&self, py: Python, name: &str) -> PyResult<PyObject> {
        let dict = self.to_dict()?;
        let dict_ref = dict.as_ref(py);
        
        if let Ok(value) = dict_ref.get_item(name) {
            Ok(value.to_object(py))
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyAttributeError, _>(
                format!("'Order' has no attribute '{}'", name)
            ))
        }
    }

    fn __repr__(&self) -> String {
        format!("Order(...)")
    }
}

#[pyclass(name = "Customer")]
#[derive(Clone)]
pub struct PyCustomerCore {
    inner: CustomerCore,
}

#[pymethods]
impl PyCustomerCore {
    #[staticmethod]
    fn from_string(raw_input: &str) -> PyResult<Self> {
        // Use singleton transform registry (lazy_static or once_cell)
        // No registry needed - transforms are injected directly
        let inner = CustomerCore::from_string(raw_input)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))?;
        Ok(Self { inner })
    }

    fn to_dict(&self) -> PyResult<PyObject> {
        Python::with_gil(|py| {
            let json_str = serde_json::to_string(&self.inner)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            let json_mod = py.import("json")?;
            let dict = json_mod.call_method1("loads", (json_str,))?;
            Ok(dict.to_object(py))
        })
    }

    fn __getattr__(&self, py: Python, name: &str) -> PyResult<PyObject> {
        let dict = self.to_dict()?;
        let dict_ref = dict.as_ref(py);
        
        if let Ok(value) = dict_ref.get_item(name) {
            Ok(value.to_object(py))
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyAttributeError, _>(
                format!("'Customer' has no attribute '{}'", name)
            ))
        }
    }

    fn __repr__(&self) -> String {
        format!("Customer(...)")
    }
}

#[pyclass(name = "Product")]
#[derive(Clone)]
pub struct PyProductCore {
    inner: ProductCore,
}

#[pymethods]
impl PyProductCore {
    #[staticmethod]
    fn from_string(raw_input: &str) -> PyResult<Self> {
        // Use singleton transform registry (lazy_static or once_cell)
        // No registry needed - transforms are injected directly
        let inner = ProductCore::from_string(raw_input)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))?;
        Ok(Self { inner })
    }

    fn to_dict(&self) -> PyResult<PyObject> {
        Python::with_gil(|py| {
            let json_str = serde_json::to_string(&self.inner)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            let json_mod = py.import("json")?;
            let dict = json_mod.call_method1("loads", (json_str,))?;
            Ok(dict.to_object(py))
        })
    }

    fn __getattr__(&self, py: Python, name: &str) -> PyResult<PyObject> {
        let dict = self.to_dict()?;
        let dict_ref = dict.as_ref(py);
        
        if let Ok(value) = dict_ref.get_item(name) {
            Ok(value.to_object(py))
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyAttributeError, _>(
                format!("'Product' has no attribute '{}'", name)
            ))
        }
    }

    fn __repr__(&self) -> String {
        format!("Product(...)")
    }
}

#[pyclass(name = "OrderLineItemCore")]
#[derive(Clone)]
pub struct PyOrderLineItemCore {
    inner: OrderLineItemCore,
}

#[pymethods]
impl PyOrderLineItemCore {
    #[getter]
    fn order_key(&self) -> String {
        self.inner.order_key.clone()
    }

    #[getter]
    fn line_number(&self) -> i64 {
        self.inner.line_number.clone()
    }

    #[getter]
    fn part_key(&self) -> String {
        self.inner.part_key.clone()
    }

    #[getter]
    fn supplier_key(&self) -> Option<String> {
        self.inner.supplier_key.clone()
    }

    #[getter]
    fn quantity(&self) -> i64 {
        self.inner.quantity.clone()
    }

    #[getter]
    fn extended_price(&self) -> f64 {
        self.inner.extended_price.clone()
    }

    #[getter]
    fn discount(&self) -> Option<f64> {
        self.inner.discount.clone()
    }

    #[getter]
    fn tax(&self) -> Option<f64> {
        self.inner.tax.clone()
    }

    #[getter]
    fn return_flag(&self) -> Option<String> {
        self.inner.return_flag.clone()
    }

    #[getter]
    fn line_status(&self) -> Option<String> {
        self.inner.line_status.clone()
    }

    #[getter]
    fn ship_date(&self) -> Option<String> {
        self.inner.ship_date.clone()
    }

    #[getter]
    fn commit_date(&self) -> Option<String> {
        self.inner.commit_date.clone()
    }

    #[getter]
    fn receipt_date(&self) -> Option<String> {
        self.inner.receipt_date.clone()
    }

    /// Create multiple OrderLineItem instances from parent entity's list field.
    #[staticmethod]
    fn from_parent_repeated(order: &PyOrderCore) -> PyResult<Vec<Self>> {
        // Use singleton transform registry (lazy_static or once_cell)
        use once_cell::sync::Lazy;
        // No registry needed - transforms are injected directly
        let instances = OrderLineItemCore::from_parent_repeated(&order.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{:?}", e)))?;
        Ok(instances.into_iter().map(|inner| Self { inner }).collect())
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = pyo3::types::PyDict::new(py);
        for (key, value) in self.inner.to_dict() {
            let py_value = match value {
                serde_json::Value::String(s) => s.into_py(py),
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        i.into_py(py)
                    } else if let Some(f) = n.as_f64() {
                        f.into_py(py)
                    } else {
                        n.to_string().into_py(py)
                    }
                },
                serde_json::Value::Bool(b) => b.into_py(py),
                serde_json::Value::Array(arr) => {
                    let list: Vec<String> = arr.iter()
                        .map(|v| v.as_str().unwrap_or("").to_string())
                        .collect();
                    list.into_py(py)
                },
                serde_json::Value::Null => py.None(),
                serde_json::Value::Object(_) => py.None(), // Skip nested objects
            };
            dict.set_item(key, py_value)?;
        }
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        self.inner.to_json()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("JSON error: {:?}", e)))
    }

    fn __getattr__(&self, py: Python, name: &str) -> PyResult<PyObject> {
        let dict = self.inner.to_dict();
        match dict.get(name) {
            Some(value) => {
                let py_value = match value {
                    serde_json::Value::String(s) => s.clone().into_py(py),
                    serde_json::Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            i.into_py(py)
                        } else if let Some(f) = n.as_f64() {
                            f.into_py(py)
                        } else {
                            n.to_string().into_py(py)
                        }
                    },
                    serde_json::Value::Bool(b) => b.into_py(py),
                    serde_json::Value::Array(arr) => {
                        let list: Vec<String> = arr.iter()
                            .map(|v| v.as_str().unwrap_or("").to_string())
                            .collect();
                        list.into_py(py)
                    },
                    serde_json::Value::Null => py.None(),
                    serde_json::Value::Object(_) => py.None(), // Skip nested objects
                };
                Ok(py_value)
            }
            None => Err(PyErr::new::<pyo3::exceptions::PyAttributeError, _>(
                format!("'OrderLineItemCore' object has no attribute '{}'", name)
            ))
        }
    }

    fn __repr__(&self) -> String {
        format!("OrderLineItemCore({:?})", self.inner.to_dict())
    }
}

/// Register all Rust entities with the Python module
pub fn register_all_entities(m: &PyModule) -> PyResult<()> {
    m.add_class::<PyOrderLineItemCore>()?;
    m.add_class::<PyOrderCore>()?;
    m.add_class::<PyCustomerCore>()?;
    m.add_class::<PyProductCore>()?;
    Ok(())
}

