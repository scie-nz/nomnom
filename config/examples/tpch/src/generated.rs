// Auto-generated from YAML entity specifications

use crate::entity::{Hl7Entity, ParsingContext, EntityError, FieldValue, IntoOptionString};
use hl7utils::{Segment, FieldPath, safe_extract};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use pyo3::prelude::*;
use sha1::{Sha1, Digest};
use regex::Regex;


/// Represents a single line item within an order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderLineItemCore {
    /// Order key (copied from parent)
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
        for line_item_data in &order.line_items {
            // Extract field: order_key
            let order_key = copy_from_parent(&parent.order_key)
                .map_err(|e| format!("Failed to extract 'order_key': {}", e))?;
            // Extract field: line_number
            let line_number = extract_field(&parent.line_item_data, "line_number")
                .map_err(|e| format!("Failed to extract 'line_number': {}", e))?;
            // Extract field: part_key
            let part_key = extract_field(&parent.line_item_data, "part_key")
                .map_err(|e| format!("Failed to extract 'part_key': {}", e))?;
            // Extract field: supplier_key
            let supplier_key = extract_field(&parent.line_item_data, "supplier_key")
                .map_err(|e| format!("Failed to extract 'supplier_key': {}", e))?;
            // Extract field: quantity
            let quantity = extract_field(&parent.line_item_data, "quantity")
                .map_err(|e| format!("Failed to extract 'quantity': {}", e))?;
            // Extract field: extended_price
            let extended_price = extract_field(&parent.line_item_data, "extended_price")
                .map_err(|e| format!("Failed to extract 'extended_price': {}", e))?;
            // Extract field: discount
            let discount = extract_field(&parent.line_item_data, "discount")
                .map_err(|e| format!("Failed to extract 'discount': {}", e))?;
            // Extract field: tax
            let tax = extract_field(&parent.line_item_data, "tax")
                .map_err(|e| format!("Failed to extract 'tax': {}", e))?;
            // Extract field: return_flag
            let return_flag = extract_field(&parent.line_item_data, "return_flag")
                .map_err(|e| format!("Failed to extract 'return_flag': {}", e))?;
            // Extract field: line_status
            let line_status = extract_field(&parent.line_item_data, "line_status")
                .map_err(|e| format!("Failed to extract 'line_status': {}", e))?;
            // Extract field: ship_date
            let ship_date = extract_field(&parent.line_item_data, "ship_date")
                .map_err(|e| format!("Failed to extract 'ship_date': {}", e))?;
            // Extract field: commit_date
            let commit_date = extract_field(&parent.line_item_data, "commit_date")
                .map_err(|e| format!("Failed to extract 'commit_date': {}", e))?;
            // Extract field: receipt_date
            let receipt_date = extract_field(&parent.line_item_data, "receipt_date")
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

/// Represents a customer order containing line items
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
    /// Number of line items in this order
    pub line_item_count: Option<i64>,
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
        // Extract field: line_item_count
        let line_item_count = count_children("OrderLineItem")
            .map_err(|e| format!("Failed to extract 'line_item_count': {}", e))?;

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
            line_item_count,
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
        map.insert("line_item_count".to_string(), serde_json::to_value(&self.line_item_count).unwrap_or(serde_json::Value::Null));
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

