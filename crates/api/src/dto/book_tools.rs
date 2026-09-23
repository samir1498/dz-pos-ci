//! The book tools on the wire (C5b of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`):
//! the working week, the absence blocks, the visit types, the next free
//! slot and the day list. Behind the
//! `clinic` feature
//! like the routes that use it; `just types` builds with the feature on.
//!
//! Re-exported by `super`, so every path outside this folder is unchanged.

use super::*;
use chrono::NaiveDateTime;
use dzpos_core::services::absence_blocks::{AbsenceBlock, BlockMade};
use dzpos_core::services::day_list::DayList;
use dzpos_core::services::free_slot::FreeSlot;
use dzpos_core::services::visit_types::VisitType;
use dzpos_core::services::working_hours::{OpenRange, WorkingWeek};

/// One open range of a day, `HH:MM` to `HH:MM` on the shop's clock. A
/// visit may end on `closes` and may not start on it; `24:00` closes at
/// midnight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "OpenRangeDto.ts")]
#[serde(deny_unknown_fields)]
pub struct OpenRangeDto {
    pub opens: String,
    pub closes: String,
}

/// The cabinet's week, Sunday first: seven lists of ranges, a lunch break
/// being two ranges and a closed day none. `null` while the cabinet has
/// never set its hours, when the book refuses no time of any day.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "WorkingHoursDto.ts")]
pub struct WorkingHoursDto {
    pub days: Option<Vec<Vec<OpenRangeDto>>>,
}

/// A new week, written whole: seven lists, Sunday first, at least one range
/// among them.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "WorkingHoursWriteDto.ts")]
#[serde(deny_unknown_fields)]
pub struct WorkingHoursWriteDto {
    pub days: Vec<Vec<OpenRangeDto>>,
}

impl From<Option<WorkingWeek>> for WorkingHoursDto {
    fn from(week: Option<WorkingWeek>) -> Self {
        WorkingHoursDto {
            days: week.map(|w| {
                w.days
                    .iter()
                    .map(|d| d.iter().map(range_dto).collect())
                    .collect()
            }),
        }
    }
}

fn range_dto(r: &OpenRange) -> OpenRangeDto {
    OpenRangeDto {
        opens: clock_text(r.opens_minute),
        closes: clock_text(r.closes_minute),
    }
}

/// Minutes from midnight as `HH:MM`; 1440 is `24:00`.
fn clock_text(minutes: i32) -> String {
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

/// The week a write names, each `HH:MM` read strictly: two digits each,
/// `00:00` to `23:59`, or `24:00` for a close at midnight. Anything else is
/// a 422 on `days`; the service checks the shape of the week itself.
pub fn parse_week(dto: WorkingHoursWriteDto) -> Result<Vec<Vec<OpenRange>>, ApiError> {
    dto.days
        .iter()
        .map(|day| {
            day.iter()
                .map(|r| {
                    Ok(OpenRange {
                        opens_minute: parse_clock(&r.opens)?,
                        closes_minute: parse_clock(&r.closes)?,
                    })
                })
                .collect()
        })
        .collect()
}

fn parse_clock(text: &str) -> Result<i32, ApiError> {
    let refused = || {
        ApiError::Request(CoreError::validation(
            "days",
            "a time of day is written HH:MM, from 00:00 to 24:00",
        ))
    };
    let (hh, mm) = text.split_once(':').ok_or_else(refused)?;
    let two_digits = |s: &str| s.len() == 2 && s.bytes().all(|b| b.is_ascii_digit());
    if !two_digits(hh) || !two_digits(mm) {
        return Err(refused());
    }
    let hours: i32 = hh.parse().map_err(|_| refused())?;
    let minutes: i32 = mm.parse().map_err(|_| refused())?;
    if minutes > 59 || hours > 24 || (hours == 24 && minutes != 0) {
        return Err(refused());
    }
    Ok(hours * 60 + minutes)
}

/// A period the doctor is away, half-open: an appointment may end on
/// `starts_at` or start on `ends_at`. Both `YYYY-MM-DD HH:MM:SS` on the
/// shop's clock.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "AbsenceBlockDto.ts")]
pub struct AbsenceBlockDto {
    pub id: String,
    pub starts_at: String,
    pub ends_at: String,
    /// A short word for the desk, never a reason about a patient.
    pub label: Option<String>,
}

impl From<AbsenceBlock> for AbsenceBlockDto {
    fn from(b: AbsenceBlock) -> Self {
        AbsenceBlockDto {
            id: b.id,
            starts_at: b.starts_at.format(DATE_TIME_FORMAT).to_string(),
            ends_at: b.ends_at.format(DATE_TIME_FORMAT).to_string(),
            label: b.label,
        }
    }
}

/// The blocks not yet over, in time order.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "AbsenceBlocksDto.ts")]
pub struct AbsenceBlocksDto {
    pub blocks: Vec<AbsenceBlockDto>,
}

/// A new block. The label is optional, 60 characters at most.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "AbsenceBlockWriteDto.ts")]
#[serde(deny_unknown_fields)]
pub struct AbsenceBlockWriteDto {
    pub starts_at: String,
    pub ends_at: String,
    pub label: Option<String>,
}

/// A block as made, and every live appointment it lands on in time order:
/// the desk moves or cancels each one; the block itself changed none.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "BlockMadeDto.ts")]
pub struct BlockMadeDto {
    pub block: AbsenceBlockDto,
    pub hits: Vec<AppointmentDto>,
}

impl From<BlockMade> for BlockMadeDto {
    fn from(m: BlockMade) -> Self {
        BlockMadeDto {
            block: AbsenceBlockDto::from(m.block),
            hits: m.hits.into_iter().map(AppointmentDto::from).collect(),
        }
    }
}

/// `YYYY-MM-DD HH:MM:SS` and nothing else, compared back after parsing the
/// way `parse_starts_at` does, with the refusal naming `field`.
pub fn parse_moment(field: &str, text: &str) -> Result<NaiveDateTime, ApiError> {
    NaiveDateTime::parse_from_str(text, DATE_TIME_FORMAT)
        .ok()
        .filter(|t| t.format(DATE_TIME_FORMAT).to_string() == text)
        .ok_or_else(|| {
            ApiError::Request(CoreError::validation(
                field,
                "a moment is written YYYY-MM-DD HH:MM:SS",
            ))
        })
}

/// A kind of visit and how long it runs, in minutes.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "VisitTypeDto.ts")]
pub struct VisitTypeDto {
    pub id: String,
    pub name: String,
    pub minutes: i32,
}

impl From<VisitType> for VisitTypeDto {
    fn from(t: VisitType) -> Self {
        VisitTypeDto {
            id: t.id,
            name: t.name,
            minutes: t.minutes,
        }
    }
}

/// Every visit type of the cabinet, by name.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "VisitTypesDto.ts")]
pub struct VisitTypesDto {
    pub visit_types: Vec<VisitTypeDto>,
}

/// A new visit type or a changed one: a name of at most 60 characters and
/// 5 to 240 minutes, a multiple of the slot length in force.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "VisitTypeWriteDto.ts")]
#[serde(deny_unknown_fields)]
pub struct VisitTypeWriteDto {
    pub name: String,
    pub minutes: i32,
}

/// The next free slot: the days searched, first and last (`YYYY-MM-DD`,
/// both read), the length searched for, and the earliest start that a
/// booking of it would be taken at, `null` when those days have no room.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "FreeSlotDto.ts")]
pub struct FreeSlotDto {
    pub first_day: String,
    pub last_day: String,
    pub slot_minutes: i32,
    pub starts_at: Option<String>,
}

impl From<FreeSlot> for FreeSlotDto {
    fn from(f: FreeSlot) -> Self {
        FreeSlotDto {
            first_day: f.first_day.format(DATE_FORMAT).to_string(),
            last_day: f.last_day.format(DATE_FORMAT).to_string(),
            slot_minutes: f.minutes,
            starts_at: f.starts_at.map(|t| t.format(DATE_TIME_FORMAT).to_string()),
        }
    }
}

/// One day as the desk reads it out: the live appointments in time order,
/// a missed one with its mark, and that day's walk-ins in arrival order,
/// whatever became of each. Data only; printing it is the screen's.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "DayListDto.ts")]
pub struct DayListDto {
    /// `YYYY-MM-DD`.
    pub day: String,
    pub appointments: Vec<AppointmentDto>,
    pub walk_ins: Vec<QueueEntryDto>,
}

impl From<DayList> for DayListDto {
    fn from(d: DayList) -> Self {
        DayListDto {
            day: d.day.format(DATE_FORMAT).to_string(),
            appointments: d
                .appointments
                .into_iter()
                .map(AppointmentDto::from)
                .collect(),
            walk_ins: d.walk_ins.into_iter().map(QueueEntryDto::from).collect(),
        }
    }
}
