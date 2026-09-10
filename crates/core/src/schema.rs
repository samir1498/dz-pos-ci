//! Diesel table definitions for the migrations under `migrations/`.
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
    counters (shop_id, name) {
        shop_id -> Integer,
        name -> Text,
        next_value -> BigInt,
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
    preferences (id) {
        id -> Integer,
        shop_id -> Integer,
        key -> Text,
        value -> Text,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    settings (seq) {
        seq -> Integer,
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

// ---- migrations/2026-09-09-000000_documents ----

diesel::table! {
    documents (id) {
        id -> Integer,
        shop_id -> Integer,
        kind -> Text,
        series -> Text,
        number -> BigInt,
        issued_at -> Timestamp,
        user_id -> Integer,
        regime -> Text,
        payment_mode -> Text,
        seller_name -> Text,
        seller_rc -> Nullable<Text>,
        seller_nif -> Nullable<Text>,
        seller_nis -> Nullable<Text>,
        seller_ai -> Nullable<Text>,
        seller_address -> Nullable<Text>,
        seller_phone -> Nullable<Text>,
        customer_id -> Nullable<Integer>,
        buyer_name -> Nullable<Text>,
        buyer_party_kind -> Nullable<Text>,
        buyer_rc -> Nullable<Text>,
        buyer_nif -> Nullable<Text>,
        buyer_nis -> Nullable<Text>,
        buyer_ai -> Nullable<Text>,
        buyer_address -> Nullable<Text>,
        ref_document_id -> Nullable<Integer>,
        total_ht_centimes -> BigInt,
        discount_centimes -> BigInt,
        subtotal_ht_centimes -> BigInt,
        tva_centimes -> BigInt,
        total_ttc_centimes -> BigInt,
        stamp_centimes -> BigInt,
        net_to_pay_centimes -> BigInt,
        tendered_centimes -> Nullable<BigInt>,
        change_centimes -> Nullable<BigInt>,
        old_balance_centimes -> Nullable<BigInt>,
        remaining_debt_centimes -> Nullable<BigInt>,
        total_debt_centimes -> Nullable<BigInt>,
        status -> Text,
        created_at -> Timestamp,
        cancelled_at -> Nullable<Timestamp>,
        cancelled_by -> Nullable<Integer>,
        cancel_reason -> Nullable<Text>,
        cancel_avoir_document_id -> Nullable<Integer>,
        series_year -> Integer,
    }
}

diesel::table! {
    document_lines (id) {
        id -> Integer,
        shop_id -> Integer,
        document_id -> Integer,
        position -> Integer,
        product_id -> Nullable<Integer>,
        name -> Text,
        barcode -> Nullable<Text>,
        qty_milli -> BigInt,
        unit_price_centimes -> BigInt,
        line_discount_centimes -> BigInt,
        rate_bps -> Integer,
        line_total_centimes -> BigInt,
        ref_line_id -> Nullable<Integer>,
    }
}

diesel::table! {
    document_tva (id) {
        id -> Integer,
        shop_id -> Integer,
        document_id -> Integer,
        rate_bps -> Integer,
        base_centimes -> BigInt,
        amount_centimes -> BigInt,
    }
}

diesel::table! {
    stock_movements (id) {
        id -> Integer,
        shop_id -> Integer,
        product_id -> Integer,
        kind -> Text,
        qty_milli -> BigInt,
        unit_cost_centimes -> BigInt,
        document_id -> Nullable<Integer>,
        user_id -> Integer,
        created_at -> Timestamp,
    }
}

diesel::table! {
    audit_log (id) {
        id -> Integer,
        shop_id -> Integer,
        user_id -> Integer,
        action -> Text,
        entity -> Text,
        entity_id -> Nullable<Integer>,
        before -> Nullable<Text>,
        after -> Nullable<Text>,
        created_at -> Timestamp,
    }
}

// ---- migrations/2026-09-09-000002_customers ----

diesel::table! {
    customers (id) {
        id -> Integer,
        shop_id -> Integer,
        name -> Text,
        party_kind -> Text,
        phone -> Nullable<Text>,
        address -> Nullable<Text>,
        rc -> Nullable<Text>,
        nif -> Nullable<Text>,
        nis -> Nullable<Text>,
        ai -> Nullable<Text>,
        credit_limit_centimes -> Nullable<BigInt>,
        warn_threshold_centimes -> Nullable<BigInt>,
        notes -> Nullable<Text>,
        active -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    debt_ledger (id) {
        id -> Integer,
        shop_id -> Integer,
        customer_id -> Integer,
        document_id -> Nullable<Integer>,
        kind -> Text,
        debit_centimes -> BigInt,
        credit_centimes -> BigInt,
        user_id -> Integer,
        note -> Nullable<Text>,
        created_at -> Timestamp,
        // ---- migrations/2026-09-09-000004_debt_payment_mode ----
        // Null on every movement that is not a payment.
        payment_mode -> Nullable<Text>,
    }
}

diesel::table! {
    debt_allocations (id) {
        id -> Integer,
        shop_id -> Integer,
        payment_ledger_id -> Integer,
        document_id -> Integer,
        amount_centimes -> BigInt,
        created_at -> Timestamp,
    }
}

// ---- migrations/2026-09-10-000008_suppliers_purchases_expenses ----

diesel::table! {
    suppliers (id) {
        id -> Integer,
        shop_id -> Integer,
        name -> Text,
        phone -> Nullable<Text>,
        address -> Nullable<Text>,
        rc -> Nullable<Text>,
        nif -> Nullable<Text>,
        nis -> Nullable<Text>,
        ai -> Nullable<Text>,
        notes -> Nullable<Text>,
        active -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    purchases (id) {
        id -> Integer,
        shop_id -> Integer,
        supplier_id -> Integer,
        supplier_document_number -> Nullable<Text>,
        // Days on the shop's calendar, `YYYY-MM-DD`, where `created_at` below
        // is the UTC moment the row was written.
        purchase_date -> Text,
        due_date -> Nullable<Text>,
        transport_centimes -> BigInt,
        extra_costs_centimes -> BigInt,
        status -> Text,
        user_id -> Integer,
        note -> Nullable<Text>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    purchase_lines (id) {
        id -> Integer,
        shop_id -> Integer,
        purchase_id -> Integer,
        product_id -> Integer,
        qty_ordered_milli -> BigInt,
        unit_cost_centimes -> BigInt,
        landed_unit_cost_centimes -> BigInt,
        qty_received_milli -> BigInt,
        qty_returned_milli -> BigInt,
    }
}

diesel::table! {
    purchase_receipts (id) {
        id -> Integer,
        shop_id -> Integer,
        purchase_id -> Integer,
        series -> Text,
        number -> BigInt,
        // A moment on the shop's calendar, the way `issued_at` is.
        received_at -> Timestamp,
        user_id -> Integer,
        note -> Nullable<Text>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    purchase_receipt_lines (id) {
        id -> Integer,
        shop_id -> Integer,
        // Carried so the two keys onto the receipt and onto the line can be
        // composite: a delivery only names a line of its own purchase.
        purchase_id -> Integer,
        receipt_id -> Integer,
        purchase_line_id -> Integer,
        qty_milli -> BigInt,
    }
}

diesel::table! {
    supplier_ledger (id) {
        id -> Integer,
        shop_id -> Integer,
        supplier_id -> Integer,
        purchase_id -> Nullable<Integer>,
        kind -> Text,
        debit_centimes -> BigInt,
        credit_centimes -> BigInt,
        user_id -> Integer,
        note -> Nullable<Text>,
        created_at -> Timestamp,
        // Null on every movement that is not a payment, and never null on one.
        payment_mode -> Nullable<Text>,
    }
}

diesel::table! {
    supplier_allocations (id) {
        id -> Integer,
        shop_id -> Integer,
        payment_ledger_id -> Integer,
        purchase_id -> Integer,
        amount_centimes -> BigInt,
        created_at -> Timestamp,
    }
}

diesel::table! {
    expense_categories (id) {
        id -> Integer,
        shop_id -> Integer,
        // The i18n key, not a label: the desktop reads the three languages
        // from its own files by this key.
        key -> Text,
        sort_order -> Integer,
        active -> Bool,
    }
}

diesel::table! {
    expenses (id) {
        id -> Integer,
        shop_id -> Integer,
        category_id -> Integer,
        amount_centimes -> BigInt,
        expense_date -> Text,
        note -> Nullable<Text>,
        user_id -> Integer,
        created_at -> Timestamp,
    }
}

diesel::table! {
    jobs (id) {
        id -> Integer,
        shop_id -> Integer,
        name -> Text,
        // Null means the job has never run.
        last_run_day -> Nullable<Text>,
        updated_at -> Timestamp,
    }
}

diesel::joinable!(categories -> shops (shop_id));
diesel::joinable!(counters -> shops (shop_id));
diesel::joinable!(products -> categories (category_id));
diesel::joinable!(preferences -> shops (shop_id));
diesel::joinable!(settings -> shops (shop_id));
diesel::joinable!(users -> shops (shop_id));
diesel::joinable!(documents -> shops (shop_id));
diesel::joinable!(documents -> users (user_id));
diesel::joinable!(document_lines -> shops (shop_id));
diesel::joinable!(document_lines -> documents (document_id));
diesel::joinable!(document_lines -> products (product_id));
diesel::joinable!(document_tva -> shops (shop_id));
diesel::joinable!(document_tva -> documents (document_id));
diesel::joinable!(stock_movements -> shops (shop_id));
diesel::joinable!(stock_movements -> products (product_id));
diesel::joinable!(stock_movements -> documents (document_id));
diesel::joinable!(stock_movements -> users (user_id));
diesel::joinable!(audit_log -> shops (shop_id));
diesel::joinable!(audit_log -> users (user_id));
diesel::joinable!(customers -> shops (shop_id));
diesel::joinable!(documents -> customers (customer_id));
diesel::joinable!(debt_ledger -> shops (shop_id));
diesel::joinable!(debt_ledger -> customers (customer_id));
diesel::joinable!(debt_ledger -> documents (document_id));
diesel::joinable!(debt_ledger -> users (user_id));
diesel::joinable!(debt_allocations -> shops (shop_id));
diesel::joinable!(debt_allocations -> debt_ledger (payment_ledger_id));
diesel::joinable!(debt_allocations -> documents (document_id));
diesel::joinable!(suppliers -> shops (shop_id));
diesel::joinable!(purchases -> shops (shop_id));
diesel::joinable!(purchases -> suppliers (supplier_id));
diesel::joinable!(purchases -> users (user_id));
diesel::joinable!(purchase_lines -> shops (shop_id));
diesel::joinable!(purchase_lines -> purchases (purchase_id));
diesel::joinable!(purchase_lines -> products (product_id));
diesel::joinable!(purchase_receipts -> shops (shop_id));
diesel::joinable!(purchase_receipts -> purchases (purchase_id));
diesel::joinable!(purchase_receipts -> users (user_id));
diesel::joinable!(purchase_receipt_lines -> shops (shop_id));
diesel::joinable!(purchase_receipt_lines -> purchase_receipts (receipt_id));
diesel::joinable!(purchase_receipt_lines -> purchase_lines (purchase_line_id));
diesel::joinable!(supplier_ledger -> shops (shop_id));
diesel::joinable!(supplier_ledger -> suppliers (supplier_id));
diesel::joinable!(supplier_ledger -> purchases (purchase_id));
diesel::joinable!(supplier_ledger -> users (user_id));
diesel::joinable!(supplier_allocations -> shops (shop_id));
diesel::joinable!(supplier_allocations -> supplier_ledger (payment_ledger_id));
diesel::joinable!(supplier_allocations -> purchases (purchase_id));
diesel::joinable!(expense_categories -> shops (shop_id));
diesel::joinable!(expenses -> shops (shop_id));
diesel::joinable!(expenses -> expense_categories (category_id));
diesel::joinable!(expenses -> users (user_id));
diesel::joinable!(jobs -> shops (shop_id));

diesel::allow_tables_to_appear_in_same_query!(
    audit_log,
    categories,
    counters,
    customers,
    debt_allocations,
    debt_ledger,
    document_lines,
    document_tva,
    documents,
    expense_categories,
    expenses,
    jobs,
    products,
    purchase_lines,
    purchase_receipt_lines,
    purchase_receipts,
    purchases,
    preferences,
    settings,
    shops,
    stock_movements,
    supplier_allocations,
    supplier_ledger,
    suppliers,
    users
);
