//! Diesel models generated from entity YAML configs

use diesel::prelude::*;
use serde::{Serialize, Deserialize};
use bigdecimal::{BigDecimal, FromPrimitive};
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

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = order_line_items)]
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

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = orders)]
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

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = customers)]
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

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = products)]
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


// From trait implementations for Core -> New conversions

impl From<&crate::generated::OrderLineItemCore> for NewOrderLineItem {
    fn from(core: &crate::generated::OrderLineItemCore) -> Self {
        Self {
            order_key: core.order_key.clone(),
            line_number: core.line_number as i32,
            part_key: core.part_key.clone(),
            supplier_key: core.supplier_key.clone(),
            quantity: core.quantity as i32,
            extended_price: BigDecimal::from_f64(core.extended_price).unwrap_or_else(|| BigDecimal::from(0)),
            discount: core.discount.and_then(BigDecimal::from_f64),
            tax: core.tax.and_then(BigDecimal::from_f64),
            return_flag: core.return_flag.clone(),
            line_status: core.line_status.clone(),
            ship_date: core.ship_date.clone(),
            commit_date: core.commit_date.clone(),
            receipt_date: core.receipt_date.clone(),
        }
    }
}

impl From<&crate::generated::OrderCore> for NewOrder {
    fn from(core: &crate::generated::OrderCore) -> Self {
        Self {
            order_key: core.order_key.clone(),
            customer_key: core.customer_key.clone(),
            order_status: core.order_status.clone(),
            total_price: BigDecimal::from_f64(core.total_price).unwrap_or_else(|| BigDecimal::from(0)),
            order_date: core.order_date.clone(),
            order_priority: core.order_priority.clone(),
            clerk: core.clerk.clone(),
            ship_priority: core.ship_priority.map(|v| v as i32),
            comment: core.comment.clone(),
        }
    }
}

impl From<&crate::generated::CustomerCore> for NewCustomer {
    fn from(core: &crate::generated::CustomerCore) -> Self {
        Self {
            customer_key: core.customer_key.clone(),
            name: core.name.clone(),
            address: core.address.clone(),
            nation_key: core.nation_key.clone(),
            phone: core.phone.clone(),
            account_balance: BigDecimal::from_f64(core.account_balance).unwrap_or_else(|| BigDecimal::from(0)),
            market_segment: core.market_segment.clone(),
            comment: core.comment.clone(),
        }
    }
}

impl From<&crate::generated::ProductCore> for NewProduct {
    fn from(core: &crate::generated::ProductCore) -> Self {
        Self {
            part_key: core.part_key.clone(),
            name: core.name.clone(),
            manufacturer: core.manufacturer.clone(),
            brand: core.brand.clone(),
            product_type: core.product_type.clone(),
            size: core.size.map(|v| v as i32),
            container: core.container.clone(),
            retail_price: BigDecimal::from_f64(core.retail_price).unwrap_or_else(|| BigDecimal::from(0)),
            comment: core.comment.clone(),
        }
    }
}

