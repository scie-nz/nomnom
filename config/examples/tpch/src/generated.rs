// Auto-generated from YAML entity specifications

use crate::entity::{Hl7Entity, ParsingContext, EntityError, FieldValue, IntoOptionString};
use hl7utils::{Segment, FieldPath, safe_extract};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use pyo3::prelude::*;
use sha1::{Sha1, Digest};
use regex::Regex;


// ============================================================================
// Auto-generated Transform Functions
// Generated from nomnom.yaml transforms section
// ============================================================================

/// Extract an integer field from a JSON object
///
/// # Arguments
///
/// * `obj` - &serde_json::Value
/// * `field` - &str
///
/// # Returns
///
/// Result<i64, String>
pub fn json_get_int(obj: &serde_json::Value, field: &str) -> Result<i64, String> {
    obj.get(field)
        .and_then(|v| v.as_i64())
        .ok_or_else(|| format!("Missing or invalid integer field '{}'", field))
}

/// Extract an optional string field from a JSON object
///
/// # Arguments
///
/// * `obj` - &serde_json::Value
/// * `field` - &str
///
/// # Returns
///
/// Result<Option<String>, String>
pub fn json_get_optional_string(obj: &serde_json::Value, field: &str) -> Result<Option<String>, String> {
    Ok(obj.get(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string()))
}

/// Extract an optional float field from a JSON object
///
/// # Arguments
///
/// * `obj` - &serde_json::Value
/// * `field` - &str
///
/// # Returns
///
/// Result<Option<f64>, String>
pub fn json_get_optional_float(obj: &serde_json::Value, field: &str) -> Result<Option<f64>, String> {
    Ok(obj.get(field)
        .and_then(|v| v.as_f64()))
}

/// Extract a string field from a JSON object
///
/// # Arguments
///
/// * `obj` - &serde_json::Value
/// * `field` - &str
///
/// # Returns
///
/// Result<String, String>
pub fn json_get_string(obj: &serde_json::Value, field: &str) -> Result<String, String> {
    obj.get(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("Missing or invalid string field '{}'", field))
}

/// Extract a float field from a JSON object
///
/// # Arguments
///
/// * `obj` - &serde_json::Value
/// * `field` - &str
///
/// # Returns
///
/// Result<f64, String>
pub fn json_get_float(obj: &serde_json::Value, field: &str) -> Result<f64, String> {
    obj.get(field)
        .and_then(|v| v.as_f64())
        .ok_or_else(|| format!("Missing or invalid float field '{}'", field))
}

// ============================================================================

/// Represents a single line item within an order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderLineItemCore {
    /// Order key (foreign key to orders table)
    pub order_key: String,
    /// Line item sequence number within order
    pub line_number: i64,
    /// Foreign key to product/part
    pub part_key: String,
    /// Foreign key to supplier
    pub supplier_key: Option<String>,
    /// Quantity ordered
    pub quantity: i64,
    /// Extended price (quantity * price)
    pub extended_price: f64,
    /// Discount percentage (0.00 to 0.10)
    pub discount: Option<f64>,
    /// Tax percentage
    pub tax: Option<f64>,
    /// Return flag (R=returned, A=accepted)
    pub return_flag: Option<String>,
    /// Line status (O=open, F=fulfilled)
    pub line_status: Option<String>,
    /// Ship date (YYYY-MM-DD)
    pub ship_date: Option<String>,
    /// Commit date (YYYY-MM-DD)
    pub commit_date: Option<String>,
    /// Receipt date (YYYY-MM-DD)
    pub receipt_date: Option<String>,
}

impl OrderLineItemCore {
    /// Create entity instances from repeated parent data
    ///
    /// # Arguments
    ///
    /// * `order` - Parent Order entity
    ///
    /// # Returns
    ///
    /// Vector of OrderLineItem instances, one per item in parent.line_items
    pub fn from_parent_repeated(
        order: &OrderCore
    ) -> Result<Vec<Self>, String> {
        let mut instances = Vec::new();

        // Iterate over parent.line_items
        for item in &order.line_items {
            // Extract field: order_key
            let order_key = order.order_key.clone();
            // Extract field: line_number
            let line_number = json_get_int(&Some(item.clone()), "line_number")
                .map_err(|e| format!("Failed to extract 'line_number': {}", e))?;
            // Extract field: part_key
            let part_key = json_get_string(&Some(item.clone()), "part_key")
                .map_err(|e| format!("Failed to extract 'part_key': {}", e))?;
            // Extract field: supplier_key
            let supplier_key = json_get_optional_string(&Some(item.clone()), "supplier_key")
                .map_err(|e| format!("Failed to extract 'supplier_key': {}", e))?;
            // Extract field: quantity
            let quantity = json_get_int(&Some(item.clone()), "quantity")
                .map_err(|e| format!("Failed to extract 'quantity': {}", e))?;
            // Extract field: extended_price
            let extended_price = json_get_float(&Some(item.clone()), "extended_price")
                .map_err(|e| format!("Failed to extract 'extended_price': {}", e))?;
            // Extract field: discount
            let discount = json_get_optional_float(&Some(item.clone()), "discount")
                .map_err(|e| format!("Failed to extract 'discount': {}", e))?;
            // Extract field: tax
            let tax = json_get_optional_float(&Some(item.clone()), "tax")
                .map_err(|e| format!("Failed to extract 'tax': {}", e))?;
            // Extract field: return_flag
            let return_flag = json_get_optional_string(&Some(item.clone()), "return_flag")
                .map_err(|e| format!("Failed to extract 'return_flag': {}", e))?;
            // Extract field: line_status
            let line_status = json_get_optional_string(&Some(item.clone()), "line_status")
                .map_err(|e| format!("Failed to extract 'line_status': {}", e))?;
            // Extract field: ship_date
            let ship_date = json_get_optional_string(&Some(item.clone()), "ship_date")
                .map_err(|e| format!("Failed to extract 'ship_date': {}", e))?;
            // Extract field: commit_date
            let commit_date = json_get_optional_string(&Some(item.clone()), "commit_date")
                .map_err(|e| format!("Failed to extract 'commit_date': {}", e))?;
            // Extract field: receipt_date
            let receipt_date = json_get_optional_string(&Some(item.clone()), "receipt_date")
                .map_err(|e| format!("Failed to extract 'receipt_date': {}", e))?;

            instances.push(Self {
                order_key,
                line_number,
                part_key,
                supplier_key,
                quantity,
                extended_price,
                discount,
                tax,
                return_flag,
                line_status,
                ship_date,
                commit_date,
                receipt_date,
            });
        }

        Ok(instances)
    }

    /// Convert entity to dictionary/map
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert("order_key".to_string(), serde_json::to_value(&self.order_key).unwrap_or(serde_json::Value::Null));
        map.insert("line_number".to_string(), serde_json::to_value(&self.line_number).unwrap_or(serde_json::Value::Null));
        map.insert("part_key".to_string(), serde_json::to_value(&self.part_key).unwrap_or(serde_json::Value::Null));
        map.insert("supplier_key".to_string(), serde_json::to_value(&self.supplier_key).unwrap_or(serde_json::Value::Null));
        map.insert("quantity".to_string(), serde_json::to_value(&self.quantity).unwrap_or(serde_json::Value::Null));
        map.insert("extended_price".to_string(), serde_json::to_value(&self.extended_price).unwrap_or(serde_json::Value::Null));
        map.insert("discount".to_string(), serde_json::to_value(&self.discount).unwrap_or(serde_json::Value::Null));
        map.insert("tax".to_string(), serde_json::to_value(&self.tax).unwrap_or(serde_json::Value::Null));
        map.insert("return_flag".to_string(), serde_json::to_value(&self.return_flag).unwrap_or(serde_json::Value::Null));
        map.insert("line_status".to_string(), serde_json::to_value(&self.line_status).unwrap_or(serde_json::Value::Null));
        map.insert("ship_date".to_string(), serde_json::to_value(&self.ship_date).unwrap_or(serde_json::Value::Null));
        map.insert("commit_date".to_string(), serde_json::to_value(&self.commit_date).unwrap_or(serde_json::Value::Null));
        map.insert("receipt_date".to_string(), serde_json::to_value(&self.receipt_date).unwrap_or(serde_json::Value::Null));
        map
    }

    /// Serialize entity to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Serialize entity to pretty-printed JSON string
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Serialize entity to NDJSON line (newline-delimited JSON)
    pub fn to_ndjson_line(&self) -> Result<String, serde_json::Error> {
        let json = self.to_json()?;
        Ok(format!("{}\n", json))
    }
}

/// Represents a product that can be ordered (reference data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductCore {
    /// Unique part/product identifier
    pub part_key: String,
    /// Product name
    pub name: String,
    /// Manufacturer name
    pub manufacturer: Option<String>,
    /// Product brand
    pub brand: Option<String>,
    /// Type of product
    pub product_type: Option<String>,
    /// Product size
    pub size: Option<i64>,
    /// Container type (e.g., SM CASE, LG BOX)
    pub container: Option<String>,
    /// Retail price
    pub retail_price: f64,
    /// Product comments
    pub comment: Option<String>,
}

/// Represents a customer who can place orders (reference data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerCore {
    /// Unique customer identifier
    pub customer_key: String,
    /// Customer name
    pub name: String,
    /// Customer address
    pub address: Option<String>,
    /// Nation identifier
    pub nation_key: Option<String>,
    /// Customer phone number
    pub phone: Option<String>,
    /// Customer account balance
    pub account_balance: f64,
    /// Market segment (e.g., AUTOMOBILE, BUILDING, FURNITURE)
    pub market_segment: Option<String>,
    /// Customer comments
    pub comment: Option<String>,
}

/// Represents a customer order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderCore {
    /// Unique order identifier
    pub order_key: String,
    /// Foreign key to customer
    pub customer_key: String,
    /// Order status (e.g., O=open, F=fulfilled, P=pending)
    pub order_status: String,
    /// Total order price
    pub total_price: f64,
    /// Date order was placed (YYYY-MM-DD)
    pub order_date: String,
    /// Order priority (e.g., 1-URGENT, 2-HIGH, 3-MEDIUM)
    pub order_priority: Option<String>,
    /// Clerk who processed the order
    pub clerk: Option<String>,
    /// Shipping priority
    pub ship_priority: Option<i64>,
    /// Order comments
    pub comment: Option<String>,
    /// Array of line items in this order (each item is a dict with line item fields)
    pub line_items: String,
}

impl OrderCore {
    /// Create root entity from raw string input
    ///
    /// # Arguments
    ///
    /// * `raw_input` - Raw string input to parse
    pub fn from_string(
        raw_input: &str,
    ) -> Result<Self, String> {

        Ok(Self {
            order_key,
            customer_key,
            order_status,
            total_price,
            order_date,
            order_priority,
            clerk,
            ship_priority,
            comment,
            line_items,
        })
    }

    /// Convert entity to dictionary/map
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert("order_key".to_string(), serde_json::to_value(&self.order_key).unwrap_or(serde_json::Value::Null));
        map.insert("customer_key".to_string(), serde_json::to_value(&self.customer_key).unwrap_or(serde_json::Value::Null));
        map.insert("order_status".to_string(), serde_json::to_value(&self.order_status).unwrap_or(serde_json::Value::Null));
        map.insert("total_price".to_string(), serde_json::to_value(&self.total_price).unwrap_or(serde_json::Value::Null));
        map.insert("order_date".to_string(), serde_json::to_value(&self.order_date).unwrap_or(serde_json::Value::Null));
        map.insert("order_priority".to_string(), serde_json::to_value(&self.order_priority).unwrap_or(serde_json::Value::Null));
        map.insert("clerk".to_string(), serde_json::to_value(&self.clerk).unwrap_or(serde_json::Value::Null));
        map.insert("ship_priority".to_string(), serde_json::to_value(&self.ship_priority).unwrap_or(serde_json::Value::Null));
        map.insert("comment".to_string(), serde_json::to_value(&self.comment).unwrap_or(serde_json::Value::Null));
        map.insert("line_items".to_string(), serde_json::to_value(&self.line_items).unwrap_or(serde_json::Value::Null));
        map
    }

    /// Serialize entity to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Serialize entity to pretty-printed JSON string
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Serialize entity to NDJSON line (newline-delimited JSON)
    pub fn to_ndjson_line(&self) -> Result<String, serde_json::Error> {
        let json = self.to_json()?;
        Ok(format!("{}\n", json))
    }
}


// ============================================================================
// Shared Entity Extraction Helpers
// ============================================================================

/// Holds all permanent entities extracted from a message
#[derive(Debug)]
pub struct PermanentEntities {
    pub order_line_item: OrderLineItemCore,
    pub product: ProductCore,
    pub customer: CustomerCore,
    pub order: OrderCore,
}

/// Extract all permanent entities from a file path
pub fn extract_permanent_entities(file_path: &str) -> Result<PermanentEntities, Box<dyn std::error::Error>> {
    // Parse root entity
    let order = OrderCore::from_string(file_path)?;

    let order_line_item = OrderLineItemCore::from_sources(&order)?;

    Ok(PermanentEntities {
        order_line_item,
        product,
        customer,
        order,
    })
}

