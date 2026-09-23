//! The clinic's gate rows (the clinic plan's C3 to C6b), built only with
//! the `clinic` feature and joined after the kernel's and the shop's in
//! `table.rs`. Split from that file when the book tools took it past its
//! 600-line limit. Grouped by what they serve (the patient file, the queue,
//! the day list and blocks, the book, the settings), not sorted by path.

use super::Gate;
use dzpos_core::services::permissions::Permission;

pub const CLINIC_GATES: &[Gate] = &[
    Gate {
        method: "GET",
        path: "/patients",
        permission: Some(Permission::ViewPatients),
        why: "the whole patient list, and a search over names and phone numbers: medical identity, not an ordinary shop list, so it is gated like the lists that carry a shop's data out (clinic plan C3)",
    },
    Gate {
        method: "POST",
        path: "/patients",
        permission: Some(Permission::EditPatients),
        why: "opening a patient's file is the desk's work; EditPatients is the clinic plan's one permission for writing a file (C3)",
    },
    Gate {
        method: "GET",
        path: "/patients/{id}",
        permission: Some(Permission::ViewPatients),
        why: "one patient's file; the same gate as the list it is opened from (clinic plan C3). Notes are a field-level rule on top, ViewPatientNotes, not a second row (C3b)",
    },
    Gate {
        method: "PUT",
        path: "/patients/{id}",
        permission: Some(Permission::EditPatients),
        why: "correcting a patient's file; the same permission that opened it (clinic plan C3)",
    },
    Gate {
        method: "POST",
        path: "/patients/{id}/archive",
        permission: Some(Permission::EditPatients),
        why: "archiving takes a file out of the search and deletes nothing, so it is a correction to the file and asks what an edit asks (clinic plan C3)",
    },
    Gate {
        method: "GET",
        path: "/queue",
        permission: Some(Permission::ViewPatients),
        why: "today's waiting room: who came in, when, and whether they have been seen, by name; it reads patients, so it asks what reading a patient's file asks (clinic plan C4, no permission of its own)",
    },
    Gate {
        method: "POST",
        path: "/queue",
        permission: Some(Permission::EditPatients),
        why: "adding an arrival to the day's queue is the desk's write on a patient; the file's write permission, reused rather than a queue one (clinic plan C4)",
    },
    Gate {
        method: "POST",
        path: "/queue/next",
        permission: Some(Permission::EditPatients),
        why: "calling in the next patient writes the entry's called_at; the same write permission as adding them (clinic plan C4)",
    },
    Gate {
        method: "POST",
        path: "/queue/{id}/call",
        permission: Some(Permission::EditPatients),
        why: "calling one patient in out of order; the same write as calling the next (clinic plan C4)",
    },
    Gate {
        method: "POST",
        path: "/queue/{id}/seen",
        permission: Some(Permission::EditPatients),
        why: "marking a called patient seen closes their entry; the same write permission (clinic plan C4)",
    },
    Gate {
        method: "POST",
        path: "/queue/{id}/left",
        permission: Some(Permission::EditPatients),
        why: "marking a patient gone unseen closes their entry the other way; the same write permission (clinic plan C4)",
    },
    Gate {
        method: "PUT",
        path: "/queue/order",
        permission: Some(Permission::EditPatients),
        why: "putting the day's queue in the desk's order decides who goes in next; the same write permission as calling them (clinic plan C6b)",
    },
    Gate {
        method: "GET",
        path: "/day-list",
        permission: Some(Permission::ViewPatients),
        why: "a day's appointments and walk-ins by name, the book and the queue read together, so it asks what reading either asks (clinic plan C5b)",
    },
    Gate {
        method: "POST",
        path: "/absence-blocks",
        permission: Some(Permission::EditPatients),
        why: "closing a period of the book is the desk's work on the book, and the answer names every patient booked inside it; the book's write permission (clinic plan C5b)",
    },
    Gate {
        method: "POST",
        path: "/absence-blocks/{id}/remove",
        permission: Some(Permission::EditPatients),
        why: "opening a closed period again is the same write on the book as closing it (clinic plan C5b)",
    },
    Gate {
        method: "GET",
        path: "/appointments",
        permission: Some(Permission::ViewPatients),
        why: "a day or a week of the book: who is coming, when, by name; it reads patients, so it asks what reading a patient's file asks (clinic plan C5, no permission of its own)",
    },
    Gate {
        method: "POST",
        path: "/appointments",
        permission: Some(Permission::EditPatients),
        why: "booking a patient into a slot is the desk's write on a patient; the file's write permission, reused (clinic plan C5)",
    },
    Gate {
        method: "GET",
        path: "/appointments/next-free",
        permission: Some(Permission::ViewPatients),
        why: "the next free slot answers which slots are taken across 60 days, so it is a read of the book, and asks what reading the book asks (clinic plan C5b review)",
    },
    Gate {
        method: "GET",
        path: "/appointments/{id}",
        permission: Some(Permission::ViewPatients),
        why: "one appointment with its patient's names, read the way the day's list is (clinic plan C5)",
    },
    Gate {
        method: "POST",
        path: "/appointments/{id}/cancel",
        permission: Some(Permission::EditPatients),
        why: "giving a slot back frees it for another patient; the same write as booking it (clinic plan C5)",
    },
    Gate {
        method: "POST",
        path: "/appointments/{id}/move",
        permission: Some(Permission::EditPatients),
        why: "moving an appointment takes a new slot on a booking's terms; the same write as booking (clinic plan C5)",
    },
    Gate {
        method: "POST",
        path: "/appointments/{id}/no-show",
        permission: Some(Permission::EditPatients),
        why: "marking a patient as not having come is the desk's write on the book, like a cancel (clinic plan C5b)",
    },
    Gate {
        method: "POST",
        path: "/appointments/{id}/no-show/clear",
        permission: Some(Permission::EditPatients),
        why: "taking a no-show mark back is the same write as setting it (clinic plan C5b)",
    },
    Gate {
        method: "POST",
        path: "/appointments/{id}/arrive",
        permission: Some(Permission::EditPatients),
        why: "marking a booked patient arrived adds them to the day's queue, the same write as adding a walk-in (clinic plan C6b)",
    },
    Gate {
        method: "POST",
        path: "/appointments/{id}/call-outcome",
        permission: Some(Permission::EditPatients),
        why: "recording what came of the confirmation call is the desk's note on a booking, a write on the book like the no-show mark (clinic plan C6b)",
    },
    Gate {
        method: "POST",
        path: "/appointments/{id}/call-outcome/clear",
        permission: Some(Permission::EditPatients),
        why: "clearing a recorded call is the same write as recording it (clinic plan C6b)",
    },
    Gate {
        method: "PUT",
        path: "/settings/working-hours",
        permission: Some(Permission::EditSettings),
        why: "the working week decides which hours the book takes at all, a setting of the cabinet beside the slot length, so it asks what that asks; its read stays open (clinic plan C5b)",
    },
    Gate {
        method: "PUT",
        path: "/settings/slot-minutes",
        permission: Some(Permission::EditSettings),
        why: "the slot length sets the grid every later booking sits on, a setting of the cabinet like the store block, so it asks what every other /settings write asks; its read stays open like GET /settings (clinic plan C5)",
    },
    Gate {
        method: "POST",
        path: "/settings/visit-types",
        permission: Some(Permission::EditSettings),
        why: "a visit type sets how long every later booking of it runs, a setting of the cabinet beside the slot length, so it asks what that asks; its read stays open (clinic plan C5b)",
    },
    Gate {
        method: "PUT",
        path: "/settings/visit-types/{id}",
        permission: Some(Permission::EditSettings),
        why: "changing a visit type's name or length is the same setting as adding it (clinic plan C5b)",
    },
    Gate {
        method: "POST",
        path: "/settings/visit-types/{id}/remove",
        permission: Some(Permission::EditSettings),
        why: "removing a visit type takes it off the list the desk books from; the same setting (clinic plan C5b)",
    },
];
