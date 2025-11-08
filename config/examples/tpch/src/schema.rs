// @generated automatically by Diesel CLI.


diesel::table! {
    order_line_items (
        id
    ) {
        id -> Integer,
        order_key -> Text,
        line_number -> Integer,
        part_key -> Text,
        supplier_key -> Nullable<Text>,
        quantity -> Integer,
        extended_price -> Float,
        discount -> Nullable<Float>,
        tax -> Nullable<Float>,
        return_flag -> Nullable<Text>,
        line_status -> Nullable<Text>,
        ship_date -> Nullable<Text>,
        commit_date -> Nullable<Text>,
        receipt_date -> Nullable<Text>,
    }
}

diesel::table! {
    orders (
        id
    ) {
        id -> Integer,
        order_key -> Text,
        customer_key -> Text,
        order_status -> Text,
        total_price -> Float,
        order_date -> Text,
        order_priority -> Nullable<Text>,
        clerk -> Nullable<Text>,
        ship_priority -> Nullable<Integer>,
        comment -> Nullable<Text>,
    }
}

diesel::table! {
    customers (
        id
    ) {
        id -> Integer,
        customer_key -> Text,
        name -> Text,
        address -> Nullable<Text>,
        nation_key -> Nullable<Text>,
        phone -> Nullable<Text>,
        account_balance -> Float,
        market_segment -> Nullable<Text>,
        comment -> Nullable<Text>,
    }
}

diesel::table! {
    products (
        id
    ) {
        id -> Integer,
        part_key -> Text,
        name -> Text,
        manufacturer -> Nullable<Text>,
        brand -> Nullable<Text>,
        product_type -> Nullable<Text>,
        size -> Nullable<Integer>,
        container -> Nullable<Text>,
        retail_price -> Float,
        comment -> Nullable<Text>,
    }
}
