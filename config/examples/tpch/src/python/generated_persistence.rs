//! Auto-generated PyO3 get_or_create methods

use pyo3::prelude::*;
use crate::db::operations::GetOrCreate;
use crate::models::*;
use crate::python::PyDatabase;
use crate::generated::*;

/// Get or create OrderLineItem in database
#[pyfunction]
pub fn orderlineitem_get_or_create(
    py: Python<'_>,
    core: &PyAny,
    database: &PyDatabase,
) -> PyResult<PyObject> {
    // Convert Core to Diesel model
    let diesel_model = OrderLineItem {
        id: 0,  // Auto-generated, placeholder value
        order_key: core.getattr("order_key")?.extract::<Option<String>>()?
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "order_key is required"))?,
        line_number: core.getattr("line_number")?.extract::<Option<String>>()?
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "line_number is required"))?,
        part_key: core.getattr("part_key")?.extract::<Option<String>>()?
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "part_key is required"))?,
        supplier_key: core.getattr("supplier_key")?.extract()?,
        quantity: core.getattr("quantity")?.extract::<Option<String>>()?
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "quantity is required"))?,
        extended_price: core.getattr("extended_price")?.extract::<Option<String>>()?
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "extended_price is required"))?,
        discount: core.getattr("discount")?.extract()?,
        tax: core.getattr("tax")?.extract()?,
        return_flag: core.getattr("return_flag")?.extract()?,
        line_status: core.getattr("line_status")?.extract()?,
        ship_date: core.getattr("ship_date")?.extract()?,
        commit_date: core.getattr("commit_date")?.extract()?,
        receipt_date: core.getattr("receipt_date")?.extract()?,
    };

    // Get connection and perform get_or_create
    let mut conn = database.get_connection()?;
    let result = OrderLineItem::get_or_create(&mut conn, &diesel_model)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Database error: {}", e)))?;

    // Convert back to PyObject (Core class)
    let core_class = py.import("data_processor._rust")?.getattr("OrderLineItemCore")?;
    let kwargs = pyo3::types::PyDict::new(py);
    kwargs.set_item("id", result.id)?;
    kwargs.set_item("order_key", Some(result.order_key))?;
    kwargs.set_item("line_number", Some(result.line_number))?;
    kwargs.set_item("part_key", Some(result.part_key))?;
    kwargs.set_item("supplier_key", result.supplier_key)?;
    kwargs.set_item("quantity", Some(result.quantity))?;
    kwargs.set_item("extended_price", Some(result.extended_price))?;
    kwargs.set_item("discount", result.discount)?;
    kwargs.set_item("tax", result.tax)?;
    kwargs.set_item("return_flag", result.return_flag)?;
    kwargs.set_item("line_status", result.line_status)?;
    kwargs.set_item("ship_date", result.ship_date)?;
    kwargs.set_item("commit_date", result.commit_date)?;
    kwargs.set_item("receipt_date", result.receipt_date)?;
    let instance = core_class.call((), Some(kwargs))?;
    Ok(instance.to_object(py))
}

/// Get or create Order in database
#[pyfunction]
pub fn order_get_or_create(
    py: Python<'_>,
    core: &PyAny,
    database: &PyDatabase,
) -> PyResult<PyObject> {
    // Convert Core to Diesel model
    let diesel_model = Order {
        id: 0,  // Auto-generated, placeholder value
        order_key: core.getattr("order_key")?.extract::<Option<String>>()?
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "order_key is required"))?,
        customer_key: core.getattr("customer_key")?.extract::<Option<String>>()?
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "customer_key is required"))?,
        order_status: core.getattr("order_status")?.extract::<Option<String>>()?
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "order_status is required"))?,
        total_price: core.getattr("total_price")?.extract::<Option<String>>()?
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "total_price is required"))?,
        order_date: core.getattr("order_date")?.extract::<Option<String>>()?
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "order_date is required"))?,
        order_priority: core.getattr("order_priority")?.extract()?,
        clerk: core.getattr("clerk")?.extract()?,
        ship_priority: core.getattr("ship_priority")?.extract()?,
        comment: core.getattr("comment")?.extract()?,
    };

    // Get connection and perform get_or_create
    let mut conn = database.get_connection()?;
    let result = Order::get_or_create(&mut conn, &diesel_model)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Database error: {}", e)))?;

    // Convert back to PyObject (Core class)
    let core_class = py.import("data_processor._rust")?.getattr("OrderCore")?;
    let kwargs = pyo3::types::PyDict::new(py);
    kwargs.set_item("id", result.id)?;
    kwargs.set_item("order_key", Some(result.order_key))?;
    kwargs.set_item("customer_key", Some(result.customer_key))?;
    kwargs.set_item("order_status", Some(result.order_status))?;
    kwargs.set_item("total_price", Some(result.total_price))?;
    kwargs.set_item("order_date", Some(result.order_date))?;
    kwargs.set_item("order_priority", result.order_priority)?;
    kwargs.set_item("clerk", result.clerk)?;
    kwargs.set_item("ship_priority", result.ship_priority)?;
    kwargs.set_item("comment", result.comment)?;
    let instance = core_class.call((), Some(kwargs))?;
    Ok(instance.to_object(py))
}

/// Get or create Customer in database
#[pyfunction]
pub fn customer_get_or_create(
    py: Python<'_>,
    core: &PyAny,
    database: &PyDatabase,
) -> PyResult<PyObject> {
    // Convert Core to Diesel model
    let diesel_model = Customer {
        id: 0,  // Auto-generated, placeholder value
        customer_key: core.getattr("customer_key")?.extract::<Option<String>>()?
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "customer_key is required"))?,
        name: core.getattr("name")?.extract::<Option<String>>()?
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "name is required"))?,
        address: core.getattr("address")?.extract()?,
        nation_key: core.getattr("nation_key")?.extract()?,
        phone: core.getattr("phone")?.extract()?,
        account_balance: core.getattr("account_balance")?.extract::<Option<String>>()?
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "account_balance is required"))?,
        market_segment: core.getattr("market_segment")?.extract()?,
        comment: core.getattr("comment")?.extract()?,
    };

    // Get connection and perform get_or_create
    let mut conn = database.get_connection()?;
    let result = Customer::get_or_create(&mut conn, &diesel_model)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Database error: {}", e)))?;

    // Convert back to PyObject (Core class)
    let core_class = py.import("data_processor._rust")?.getattr("CustomerCore")?;
    let kwargs = pyo3::types::PyDict::new(py);
    kwargs.set_item("id", result.id)?;
    kwargs.set_item("customer_key", Some(result.customer_key))?;
    kwargs.set_item("name", Some(result.name))?;
    kwargs.set_item("address", result.address)?;
    kwargs.set_item("nation_key", result.nation_key)?;
    kwargs.set_item("phone", result.phone)?;
    kwargs.set_item("account_balance", Some(result.account_balance))?;
    kwargs.set_item("market_segment", result.market_segment)?;
    kwargs.set_item("comment", result.comment)?;
    let instance = core_class.call((), Some(kwargs))?;
    Ok(instance.to_object(py))
}

/// Get or create Product in database
#[pyfunction]
pub fn product_get_or_create(
    py: Python<'_>,
    core: &PyAny,
    database: &PyDatabase,
) -> PyResult<PyObject> {
    // Convert Core to Diesel model
    let diesel_model = Product {
        id: 0,  // Auto-generated, placeholder value
        part_key: core.getattr("part_key")?.extract::<Option<String>>()?
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "part_key is required"))?,
        name: core.getattr("name")?.extract::<Option<String>>()?
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "name is required"))?,
        manufacturer: core.getattr("manufacturer")?.extract()?,
        brand: core.getattr("brand")?.extract()?,
        product_type: core.getattr("product_type")?.extract()?,
        size: core.getattr("size")?.extract()?,
        container: core.getattr("container")?.extract()?,
        retail_price: core.getattr("retail_price")?.extract::<Option<String>>()?
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "retail_price is required"))?,
        comment: core.getattr("comment")?.extract()?,
    };

    // Get connection and perform get_or_create
    let mut conn = database.get_connection()?;
    let result = Product::get_or_create(&mut conn, &diesel_model)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Database error: {}", e)))?;

    // Convert back to PyObject (Core class)
    let core_class = py.import("data_processor._rust")?.getattr("ProductCore")?;
    let kwargs = pyo3::types::PyDict::new(py);
    kwargs.set_item("id", result.id)?;
    kwargs.set_item("part_key", Some(result.part_key))?;
    kwargs.set_item("name", Some(result.name))?;
    kwargs.set_item("manufacturer", result.manufacturer)?;
    kwargs.set_item("brand", result.brand)?;
    kwargs.set_item("product_type", result.product_type)?;
    kwargs.set_item("size", result.size)?;
    kwargs.set_item("container", result.container)?;
    kwargs.set_item("retail_price", Some(result.retail_price))?;
    kwargs.set_item("comment", result.comment)?;
    let instance = core_class.call((), Some(kwargs))?;
    Ok(instance.to_object(py))
}

/// Register all get_or_create functions with Python module
pub fn register_persistence_functions(m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(orderlineitem_get_or_create, m)?)?;
    m.add_function(wrap_pyfunction!(order_get_or_create, m)?)?;
    m.add_function(wrap_pyfunction!(customer_get_or_create, m)?)?;
    m.add_function(wrap_pyfunction!(product_get_or_create, m)?)?;
    Ok(())
}
