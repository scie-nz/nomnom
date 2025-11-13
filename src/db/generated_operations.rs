//! Auto-generated GetOrCreate implementations

use diesel::prelude::*;
use diesel::result::Error as DieselError;
use crate::models::*;
use crate::schema::*;
use crate::db::operations::GetOrCreate;

// ============================================================================
// OrderLineItem - get_or_create implementation
// ============================================================================

impl GetOrCreate for OrderLineItem {
    fn get_or_create(
        conn: &mut MysqlConnection,
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
                    .values(instance)
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
        conn: &mut MysqlConnection,
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
                    .values(instance)
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
        conn: &mut MysqlConnection,
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
                    .values(instance)
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
        conn: &mut MysqlConnection,
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
                    .values(instance)
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

