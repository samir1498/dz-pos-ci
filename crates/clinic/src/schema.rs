//! The clinic's tables, as diesel sees them. Hand-written beside the
//! migration that creates each one (`crates/kernel/migrations`, the one
//! shared list), rather than added to `dzpos_kernel::schema`: the kernel
//! declares the tables every trade shares and the shop's, and a clinic table
//! there would put a clinic word in the domain-free crate. Nothing here joins
//! a kernel table; the two joins are the queue's and the book's, each to the
//! patient its row names.

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

// ---- migrations/2026-09-23-000021_appointments ----

diesel::table! {
    appointments (id) {
        // A UUID v7 as text, made by `services::appointments`, never by SQLite.
        id -> Text,
        shop_id -> Integer,
        patient_id -> Text,
        // The shop clock's local start, on a whole minute.
        starts_at -> Timestamp,
        // The visit type's length, or the slot length, when it was booked.
        slot_minutes -> Integer,
        note -> Nullable<Text>,
        // NULL while the appointment is live.
        cancelled_at -> Nullable<Timestamp>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        // migrations/2026-09-23-000025_appointment_no_show: when the desk
        // marked it missed; NULL while unmarked.
        no_show_at -> Nullable<Timestamp>,
    }
}

// ---- migrations/2026-09-23-000022_working_hours ----

diesel::table! {
    working_hours (id) {
        // A UUID v7 as text, made by `services::working_hours`.
        id -> Text,
        shop_id -> Integer,
        // 0 is Sunday, 6 Saturday.
        weekday -> Integer,
        // Minutes from midnight, half-open; 1440 closes at midnight.
        opens_minute -> Integer,
        closes_minute -> Integer,
        created_at -> Timestamp,
    }
}

// ---- migrations/2026-09-23-000023_absence_blocks ----

diesel::table! {
    absence_blocks (id) {
        // A UUID v7 as text, made by `services::absence_blocks`.
        id -> Text,
        shop_id -> Integer,
        // The shop clock's local start and end, half-open.
        starts_at -> Timestamp,
        ends_at -> Timestamp,
        label -> Nullable<Text>,
        created_at -> Timestamp,
    }
}

// ---- migrations/2026-09-23-000024_visit_types ----

diesel::table! {
    visit_types (id) {
        // A UUID v7 as text, made by `services::visit_types`.
        id -> Text,
        shop_id -> Integer,
        name -> Text,
        // 5 to 240 in steps of 5.
        minutes -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

// The day's list and the book both read each row's patient names beside it.
diesel::joinable!(queue_entries -> patients (patient_id));
diesel::joinable!(appointments -> patients (patient_id));
diesel::allow_tables_to_appear_in_same_query!(patients, queue_entries, appointments, working_hours);
