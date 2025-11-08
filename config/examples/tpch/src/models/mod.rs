//! Diesel models generated from entity YAML configs

use diesel::prelude::*;
use serde::{Serialize, Deserialize};
use bigdecimal::BigDecimal;
use crate::schema::*;

#[derive(Debug, Clone, Queryable, Serialize, Deserialize)]
#[diesel(table_name = order_line_items)]
pub struct OrderLineItem {
    pub id: i32,
    pub order_key: String,
    pub line_number: i32,
    pub part_key: String,
    pub supplier_key: Option<String>,
    pub quantity: i32,
    pub extended_price: BigDecimal,
    pub discount: Option<BigDecimal>,
    pub tax: Option<BigDecimal>,
    pub return_flag: Option<String>,
    pub line_status: Option<String>,
    pub ship_date: Option<String>,
    pub commit_date: Option<String>,
    pub receipt_date: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewOrderLineItem {
    pub order_key: String,
    pub line_number: i32,
    pub part_key: String,
    pub supplier_key: Option<String>,
    pub quantity: i32,
    pub extended_price: BigDecimal,
    pub discount: Option<BigDecimal>,
    pub tax: Option<BigDecimal>,
    pub return_flag: Option<String>,
    pub line_status: Option<String>,
    pub ship_date: Option<String>,
    pub commit_date: Option<String>,
    pub receipt_date: Option<String>,
}

#[derive(Debug, Clone, Queryable, Serialize, Deserialize)]
#[diesel(table_name = orders)]
pub struct Order {
    pub id: i32,
    pub order_key: String,
    pub customer_key: String,
    pub order_status: String,
    pub total_price: BigDecimal,
    pub order_date: String,
    pub order_priority: Option<String>,
    pub clerk: Option<String>,
    pub ship_priority: Option<i32>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewOrder {
    pub order_key: String,
    pub customer_key: String,
    pub order_status: String,
    pub total_price: BigDecimal,
    pub order_date: String,
    pub order_priority: Option<String>,
    pub clerk: Option<String>,
    pub ship_priority: Option<i32>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Queryable, Serialize, Deserialize)]
#[diesel(table_name = customers)]
pub struct Customer {
    pub id: i32,
    pub customer_key: String,
    pub name: String,
    pub address: Option<String>,
    pub nation_key: Option<String>,
    pub phone: Option<String>,
    pub account_balance: BigDecimal,
    pub market_segment: Option<String>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewCustomer {
    pub customer_key: String,
    pub name: String,
    pub address: Option<String>,
    pub nation_key: Option<String>,
    pub phone: Option<String>,
    pub account_balance: BigDecimal,
    pub market_segment: Option<String>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Queryable, Serialize, Deserialize)]
#[diesel(table_name = products)]
pub struct Product {
    pub id: i32,
    pub part_key: String,
    pub name: String,
    pub manufacturer: Option<String>,
    pub brand: Option<String>,
    pub product_type: Option<String>,
    pub size: Option<i32>,
    pub container: Option<String>,
    pub retail_price: BigDecimal,
    pub comment: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewProduct {
    pub part_key: String,
    pub name: String,
    pub manufacturer: Option<String>,
    pub brand: Option<String>,
    pub product_type: Option<String>,
    pub size: Option<i32>,
    pub container: Option<String>,
    pub retail_price: BigDecimal,
    pub comment: Option<String>,
}

