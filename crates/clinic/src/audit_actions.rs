//! The `ACTION_*` tags the clinic writes to `dzpos_kernel::services::audit`,
//! kept in this crate the way `dzpos_retail::audit_actions` keeps the
//! shop's: the audit service records whatever string it is handed, and the
//! vocabulary belongs to the module that raises it.
//!
//! Named `<thing>.<what happened>`, the kernel's scheme. Every caller names
//! one explicitly (`crate::audit_actions::ACTION_X`), never through a glob.
//!
//! The audit row's `entity_id` column is an integer and a patient's id (and
//! a queue entry's, and an appointment's) is a UUID, so every row about one
//! of them carries `entity_id: None` and the id inside its `before`/`after`
//! JSON instead. The slot length is the shop's, so its row names the shop.

/// A patient file opened. `after` is the file as written.
pub const ACTION_PATIENT_CREATE: &str = "patient.create";
/// A patient file corrected. `before` and `after` are the file on either
/// side of the change.
pub const ACTION_PATIENT_UPDATE: &str = "patient.update";
/// A patient file taken out of the search. Never a delete: the queue and
/// the book will point at it.
pub const ACTION_PATIENT_ARCHIVE: &str = "patient.archive";

/// A patient added to the day's queue. `after` is the entry as written,
/// the patient's id with it.
pub const ACTION_QUEUE_ADD: &str = "queue.add";
/// A patient called in, next in line or out of order alike. `before` and
/// `after` are the entry on either side.
pub const ACTION_QUEUE_CALL: &str = "queue.call";
/// A patient seen by the doctor.
pub const ACTION_QUEUE_SEEN: &str = "queue.seen";
/// A patient who left the waiting room without being seen.
pub const ACTION_QUEUE_LEFT: &str = "queue.left";
/// A booked patient marked arrived (C6b): an entry added to the day's
/// queue for the appointment, or a walk-in's entry of the same patient
/// linked to it. `after` is the entry with its `appointment_id`; `before`
/// is the walk-in's entry when one was linked, none when one was added.
pub const ACTION_QUEUE_ARRIVE: &str = "queue.arrive";
/// The desk put the day's queue in a new order (C6b). `before` and `after`
/// are the day and its entry ids in the order on either side; no entry's
/// own JSON, since nothing else about them moved.
pub const ACTION_QUEUE_REORDER: &str = "queue.reorder";

/// A patient given a slot in the book. `after` is the appointment as
/// written, the patient's id with it and the note left out.
pub const ACTION_APPOINTMENT_BOOK: &str = "appointment.book";
/// A slot given back. `before` and `after` are the row on either side.
pub const ACTION_APPOINTMENT_CANCEL: &str = "appointment.cancel";
/// An appointment moved to another start, the same row kept. `before` and
/// `after` carry both starts.
pub const ACTION_APPOINTMENT_MOVE: &str = "appointment.move";
/// The book's slot length changed. `entity_id` is the shop, the way the
/// shop's own dated settings record theirs.
pub const ACTION_SLOT_MINUTES_SET: &str = "slot_minutes.set";

/// The cabinet's working week replaced. `entity_id` is the shop; `before`
/// is the week it replaced (none the first time) and `after` the new one,
/// each as seven lists of `[opens, closes]` minutes, Sunday first.
pub const ACTION_WORKING_HOURS_SET: &str = "working_hours.set";

/// An absence block made. `after` is the block as written, its id inside.
pub const ACTION_ABSENCE_BLOCK_CREATE: &str = "absence_block.create";
/// An absence block removed; the row is deleted and `before` is all that
/// stays of it.
pub const ACTION_ABSENCE_BLOCK_REMOVE: &str = "absence_block.remove";

/// A visit type added to the settings. `after` is the type as written.
pub const ACTION_VISIT_TYPE_CREATE: &str = "visit_type.create";
/// A visit type renamed or its length changed. Bookings already made keep
/// the length they copied.
pub const ACTION_VISIT_TYPE_UPDATE: &str = "visit_type.update";
/// A visit type removed; the row is deleted and `before` is what stays of
/// it.
pub const ACTION_VISIT_TYPE_REMOVE: &str = "visit_type.remove";

/// A past appointment marked as one the patient did not come to. `before`
/// and `after` are the row on either side.
pub const ACTION_APPOINTMENT_NO_SHOW: &str = "appointment.no_show";
/// A no-show mark taken back, the desk having marked the wrong patient.
pub const ACTION_APPOINTMENT_NO_SHOW_CLEAR: &str = "appointment.no_show_clear";

/// What came of the desk's confirmation call recorded on an appointment
/// (C6b), or a recorded one cleared. `before` and `after` are the row on
/// either side, the outcome and its moment in them.
pub const ACTION_APPOINTMENT_CALL: &str = "appointment.call";
