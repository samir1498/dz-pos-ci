//! Diesel table definitions for `migrations/2026-09-08-000000_init`.
//! Hand-written to match that SQL. The tables are STRICT, so the migration
//! declares timestamps as TEXT and the boolean as INTEGER: SQLite stores
//! both that way already, and diesel reads `Timestamp` and `Bool` from
//! them. `diesel print-schema` would name the storage type instead of the
//! domain one, so it is a starting point here, not the source.

diesel::table! {
    categories (id) {
        id -> Integer,
        shop_id -> Integer,
        name -> Text,
        default_rate_bps -> Integer,
    }
}

diesel::table! {
    products (id) {
        id -> Integer,
        shop_id -> Integer,
        name -> Text,
        barcode -> Nullable<Text>,
        category_id -> Nullable<Integer>,
        unit -> Text,
        cost_centimes -> BigInt,
        selling_centimes -> BigInt,
        wholesale_centimes -> Nullable<BigInt>,
        qty_on_hand_milli -> BigInt,
        low_stock_at_milli -> BigInt,
        rate_bps -> Integer,
        active -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    settings (shop_id, key, valid_from) {
        shop_id -> Integer,
        key -> Text,
        value -> Text,
        valid_from -> Timestamp,
    }
}

diesel::table! {
    shops (id) {
        id -> Integer,
        name -> Text,
        rc -> Nullable<Text>,
        nif -> Nullable<Text>,
        nis -> Nullable<Text>,
        ai -> Nullable<Text>,
        address -> Nullable<Text>,
        phone -> Nullable<Text>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    users (id) {
        id -> Integer,
        shop_id -> Integer,
        name -> Text,
        role -> Text,
        pin_hash -> Text,
        created_at -> Timestamp,
    }
}

diesel::joinable!(categories -> shops (shop_id));
diesel::joinable!(products -> categories (category_id));
diesel::joinable!(settings -> shops (shop_id));
diesel::joinable!(users -> shops (shop_id));

diesel::allow_tables_to_appear_in_same_query!(categories, products, settings, shops, users);
