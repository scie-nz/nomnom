//! Auto-generated GetOrCreate implementations

use diesel::prelude::*;
use diesel::result::Error as DieselError;
use diesel::pg::PgConnection;
use crate::models::*;
use crate::schema::*;
use crate::db::operations::GetOrCreate;

// ============================================================================
// OrderLineItem - get_or_create implementation
// ============================================================================

impl GetOrCreate for OrderLineItem {
    fn get_or_create(
        conn: &mut PgConnection,
        instance: &Self,
    ) -> Result<Self, DieselError> {
        use crate::schema::order_line_items::dsl::*;

        // Check if exists by unicity fields
        let mut query = order_line_items.into_boxed();

        query = query.filter(order_key.eq(&instance.order_key));
        query = query.filter(line_number.eq(&instance.line_number));

        let existing = query.first::<OrderLineItem>(conn).optional()?;

        match existing {
            Some(found) => Ok(found),
            None => {
                diesel::insert_into(order_line_items)
                    .values((
                        order_key.eq(&instance.order_key),
                        line_number.eq(instance.line_number),
                        part_key.eq(&instance.part_key),
                        supplier_key.eq(&instance.supplier_key),
                        quantity.eq(instance.quantity),
                        extended_price.eq(&instance.extended_price),
                        discount.eq(&instance.discount),
                        tax.eq(&instance.tax),
                        return_flag.eq(&instance.return_flag),
                        line_status.eq(&instance.line_status),
                        ship_date.eq(&instance.ship_date),
                        commit_date.eq(&instance.commit_date),
                        receipt_date.eq(&instance.receipt_date),
                    ))
                    .execute(conn)?;

                // Query back to get auto-generated id
                order_line_items
                    .order(id.desc())
                    .first::<OrderLineItem>(conn)
            }
        }
    }

    fn unicity_fields() -> Vec<&'static str> {
        vec!["order_key", "line_number"]
    }
}

// ============================================================================
// Order - get_or_create implementation
// ============================================================================

impl GetOrCreate for Order {
    fn get_or_create(
        conn: &mut PgConnection,
        instance: &Self,
    ) -> Result<Self, DieselError> {
        use crate::schema::orders::dsl::*;

        // Check if exists by unicity fields
        let mut query = orders.into_boxed();

        query = query.filter(order_key.eq(&instance.order_key));

        let existing = query.first::<Order>(conn).optional()?;

        match existing {
            Some(found) => Ok(found),
            None => {
                diesel::insert_into(orders)
                    .values((
                        order_key.eq(&instance.order_key),
                        customer_key.eq(&instance.customer_key),
                        order_status.eq(&instance.order_status),
                        total_price.eq(&instance.total_price),
                        order_date.eq(&instance.order_date),
                        order_priority.eq(&instance.order_priority),
                        clerk.eq(&instance.clerk),
                        ship_priority.eq(instance.ship_priority),
                        comment.eq(&instance.comment),
                    ))
                    .execute(conn)?;

                // Query back to get auto-generated id
                orders
                    .order(id.desc())
                    .first::<Order>(conn)
            }
        }
    }

    fn unicity_fields() -> Vec<&'static str> {
        vec!["order_key"]
    }
}

// ============================================================================
// Customer - get_or_create implementation
// ============================================================================

impl GetOrCreate for Customer {
    fn get_or_create(
        conn: &mut PgConnection,
        instance: &Self,
    ) -> Result<Self, DieselError> {
        use crate::schema::customers::dsl::*;

        // Check if exists by unicity fields
        let mut query = customers.into_boxed();

        query = query.filter(customer_key.eq(&instance.customer_key));

        let existing = query.first::<Customer>(conn).optional()?;

        match existing {
            Some(found) => Ok(found),
            None => {
                diesel::insert_into(customers)
                    .values((
                        customer_key.eq(&instance.customer_key),
                        name.eq(&instance.name),
                        address.eq(&instance.address),
                        nation_key.eq(&instance.nation_key),
                        phone.eq(&instance.phone),
                        account_balance.eq(&instance.account_balance),
                        market_segment.eq(&instance.market_segment),
                        comment.eq(&instance.comment),
                    ))
                    .execute(conn)?;

                // Query back to get auto-generated id
                customers
                    .order(id.desc())
                    .first::<Customer>(conn)
            }
        }
    }

    fn unicity_fields() -> Vec<&'static str> {
        vec!["customer_key"]
    }
}

// ============================================================================
// Product - get_or_create implementation
// ============================================================================

impl GetOrCreate for Product {
    fn get_or_create(
        conn: &mut PgConnection,
        instance: &Self,
    ) -> Result<Self, DieselError> {
        use crate::schema::products::dsl::*;

        // Check if exists by unicity fields
        let mut query = products.into_boxed();

        query = query.filter(part_key.eq(&instance.part_key));

        let existing = query.first::<Product>(conn).optional()?;

        match existing {
            Some(found) => Ok(found),
            None => {
                diesel::insert_into(products)
                    .values((
                        part_key.eq(&instance.part_key),
                        name.eq(&instance.name),
                        manufacturer.eq(&instance.manufacturer),
                        brand.eq(&instance.brand),
                        product_type.eq(&instance.product_type),
                        size.eq(instance.size),
                        container.eq(&instance.container),
                        retail_price.eq(&instance.retail_price),
                        comment.eq(&instance.comment),
                    ))
                    .execute(conn)?;

                // Query back to get auto-generated id
                products
                    .order(id.desc())
                    .first::<Product>(conn)
            }
        }
    }

    fn unicity_fields() -> Vec<&'static str> {
        vec!["part_key"]
    }
}

