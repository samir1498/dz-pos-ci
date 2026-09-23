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
