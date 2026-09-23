//! The clinic's tables, as diesel sees them. Hand-written beside the
//! migration that creates each one (`crates/kernel/migrations`, the one
//! shared list), rather than added to `dzpos_kernel::schema`: the kernel
//! declares the tables every trade shares and the shop's, and a clinic table
//! there would put a clinic word in the domain-free crate. Nothing here joins
//! a kernel table; the one join is the queue's to the patient it names.

// ---- migrations/2026-09-23-000019_patients ----

diesel::table! {
    patients (id) {
        // A UUID v7 as text, made by `services::patients`, never by SQLite.
        id -> Text,
        shop_id -> Integer,
        first_name -> Text,
        last_name -> Text,
        sex -> Nullable<Text>,
        // ISO date text; the migration's CHECK refuses anything else.
        date_of_birth -> Nullable<Date>,
        phone -> Nullable<Text>,
        notes -> Nullable<Text>,
        // The shop's clock, like every other stamp this service writes.
        archived_at -> Nullable<Timestamp>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

// ---- migrations/2026-09-23-000020_queue_entries ----

diesel::table! {
    queue_entries (id) {
        // A UUID v7 as text, made by `services::queue`, never by SQLite.
        id -> Text,
        shop_id -> Integer,
        patient_id -> Text,
        // The shop clock's local day of the arrival, ISO text.
        day -> Date,
        // The shop's clock, like every other stamp this crate writes.
        arrived_at -> Timestamp,
        called_at -> Nullable<Timestamp>,
        seen_at -> Nullable<Timestamp>,
        left_at -> Nullable<Timestamp>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

// The day's list reads each entry's patient names beside it.
diesel::joinable!(queue_entries -> patients (patient_id));
diesel::allow_tables_to_appear_in_same_query!(patients, queue_entries);
