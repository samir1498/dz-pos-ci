// @generated automatically by Diesel CLI.
// Manually edited:
// - PK columns: Nullable<Integer> → Integer (Diesel SQLite quirk)
// - REAL columns: Float → Double (f64 compatibility)

diesel::table! {
    expense_categories (id) {
        id -> Integer,
        name -> Text,
        created_at -> Text,
    }
}

diesel::table! {
    expenses (id) {
        id -> Integer,
        date -> Text,
        category -> Text,
        description -> Nullable<Text>,
        amount -> Double,
        created_at -> Text,
    }
}

diesel::table! {
    products (id) {
        id -> Integer,
        name -> Text,
        barcode -> Nullable<Text>,
        cost_price -> Double,
        selling_price -> Double,
        quantity_on_hand -> Integer,
        created_at -> Text,
    }
}

diesel::table! {
    purchase_order_items (id) {
        id -> Integer,
        purchase_order_id -> Integer,
        product_name -> Text,
        quantity -> Integer,
        unit_cost -> Double,
        line_total -> Double,
    }
}

diesel::table! {
    purchase_orders (id) {
        id -> Integer,
        purchase_order_number -> Text,
        supplier_id -> Nullable<Integer>,
        status -> Text,
        notes -> Nullable<Text>,
        total -> Double,
        created_at -> Text,
    }
}

diesel::table! {
    suppliers (id) {
        id -> Integer,
        name -> Text,
        phone -> Nullable<Text>,
        notes -> Nullable<Text>,
    }
}

diesel::table! {
    transaction_items (id) {
        id -> Integer,
        transaction_id -> Integer,
        product_id -> Integer,
        product_name -> Text,
        quantity -> Integer,
        selling_price -> Double,
        cost_price -> Double,
        line_total -> Double,
    }
}

diesel::table! {
    transactions (id) {
        id -> Integer,
        timestamp -> Text,
        total -> Double,
    }
}

diesel::joinable!(purchase_order_items -> purchase_orders (purchase_order_id));
diesel::joinable!(purchase_orders -> suppliers (supplier_id));
diesel::joinable!(transaction_items -> products (product_id));
diesel::joinable!(transaction_items -> transactions (transaction_id));

diesel::allow_tables_to_appear_in_same_query!(
    expense_categories,
    expenses,
    products,
    purchase_order_items,
    purchase_orders,
    suppliers,
    transaction_items,
    transactions,
);
