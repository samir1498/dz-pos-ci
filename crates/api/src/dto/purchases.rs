//! Purchase orders, what was received against them, and what was paid.
//!
//! Re-exported by `super`, so every path outside this folder is unchanged.

use super::*;

/// Where an order stands (features.md §1, Purchase). The whole union crosses
/// from the first version: a screen that met an unknown state could only
/// refuse the whole answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "PurchaseStatusDto.ts")]
#[serde(rename_all = "snake_case")]
pub enum PurchaseStatusDto {
    Ordered,
    PartiallyReceived,
    Received,
    Cancelled,
    ClosedShort,
}

impl From<PurchaseStatus> for PurchaseStatusDto {
    fn from(s: PurchaseStatus) -> Self {
        match s {
            PurchaseStatus::Ordered => PurchaseStatusDto::Ordered,
            PurchaseStatus::PartiallyReceived => PurchaseStatusDto::PartiallyReceived,
            PurchaseStatus::Received => PurchaseStatusDto::Received,
            PurchaseStatus::Cancelled => PurchaseStatusDto::Cancelled,
            PurchaseStatus::ClosedShort => PurchaseStatusDto::ClosedShort,
        }
    }
}

impl From<PurchaseStatusDto> for PurchaseStatus {
    fn from(s: PurchaseStatusDto) -> Self {
        match s {
            PurchaseStatusDto::Ordered => PurchaseStatus::Ordered,
            PurchaseStatusDto::PartiallyReceived => PurchaseStatus::PartiallyReceived,
            PurchaseStatusDto::Received => PurchaseStatus::Received,
            PurchaseStatusDto::Cancelled => PurchaseStatus::Cancelled,
            PurchaseStatusDto::ClosedShort => PurchaseStatus::ClosedShort,
        }
    }
}

/// An order as the list reads it: the paper and nothing of its lines. The
/// list shows a row per order, and the lines are what `/purchases/{id}`
/// answers.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "PurchaseDto.ts")]
pub struct PurchaseDto {
    pub id: i32,
    pub shop_id: i32,
    pub supplier_id: i32,
    /// The number written on the paper the supplier sent, when it carried
    /// one.
    pub supplier_document_number: Option<String>,
    /// `YYYY-MM-DD` on the shop's calendar.
    pub purchase_date: String,
    pub due_date: Option<String>,
    pub transport_centimes: i64,
    pub extra_costs_centimes: i64,
    /// The two columns above as one amount, added in the core. The screens
    /// show what the goods cost to get here under one heading, and a screen
    /// that added the two itself would be a second answer to a question the
    /// core already answers — and an unchecked one.
    pub extras_centimes: i64,
    pub status: PurchaseStatusDto,
    pub user_id: i32,
    pub note: Option<String>,
    /// `YYYY-MM-DD HH:MM:SS`, the moment the row was written.
    pub created_at: String,
}

impl TryFrom<Purchase> for PurchaseDto {
    type Error = ApiError;

    fn try_from(p: Purchase) -> Result<Self, ApiError> {
        let extras = p.extras().map_err(ApiError::from)?;
        Ok(PurchaseDto {
            id: p.id,
            shop_id: p.shop_id,
            supplier_id: p.supplier_id,
            supplier_document_number: p.supplier_document_number,
            purchase_date: p.purchase_date,
            due_date: p.due_date,
            transport_centimes: p.transport.as_centimes(),
            extra_costs_centimes: p.extra_costs.as_centimes(),
            extras_centimes: extras.as_centimes(),
            status: p.status.into(),
            user_id: p.user_id,
            note: p.note,
            created_at: p.created_at.format(DATE_TIME_FORMAT).to_string(),
        })
    }
}

/// One product on an order, with what has arrived and what has gone back.
/// Both totals are the file's running columns, so a screen counting the
/// receipts itself would be a second answer.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "PurchaseLineDto.ts")]
pub struct PurchaseLineDto {
    pub id: i32,
    pub product_id: i32,
    pub qty_ordered_milli: i64,
    /// What the supplier charges for the unit.
    pub unit_cost_centimes: i64,
    /// That plus this line's share of the transport and the extra costs,
    /// fixed when the order was saved.
    pub landed_unit_cost_centimes: i64,
    pub qty_received_milli: i64,
    pub qty_returned_milli: i64,
}

impl From<PurchaseLine> for PurchaseLineDto {
    fn from(l: PurchaseLine) -> Self {
        PurchaseLineDto {
            id: l.id,
            product_id: l.product_id,
            qty_ordered_milli: l.qty_ordered_milli,
            unit_cost_centimes: l.unit_cost.as_centimes(),
            landed_unit_cost_centimes: l.landed_unit_cost.as_centimes(),
            qty_received_milli: l.qty_received_milli,
            qty_returned_milli: l.qty_returned_milli,
        }
    }
}

/// What arrived on one delivery, line by line.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "PurchaseReceiptLineDto.ts")]
pub struct PurchaseReceiptLineDto {
    pub purchase_line_id: i32,
    pub qty_milli: i64,
}

/// One bon de réception: the delivery, its number and what came on it. It is
/// not a document and takes no document number; the series is
/// `reception:<year>` and resets on 1 January like every other one.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "PurchaseReceiptDto.ts")]
pub struct PurchaseReceiptDto {
    pub id: i32,
    pub series: String,
    pub number: i64,
    /// `YYYY-MM-DD HH:MM:SS` on the shop's calendar.
    pub received_at: String,
    pub user_id: i32,
    pub note: Option<String>,
    pub lines: Vec<PurchaseReceiptLineDto>,
}

/// A whole order: the paper, its lines with what has arrived and gone back,
/// and every delivery against it, newest first. Every route that changes an
/// order answers this, so the screen never has a change without the state it
/// left behind.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "PurchaseDetailDto.ts")]
pub struct PurchaseDetailDto {
    pub purchase: PurchaseDto,
    pub lines: Vec<PurchaseLineDto>,
    pub receipts: Vec<PurchaseReceiptDto>,
}

impl TryFrom<PurchaseView> for PurchaseDetailDto {
    type Error = ApiError;

    fn try_from(v: PurchaseView) -> Result<Self, ApiError> {
        Ok(PurchaseDetailDto {
            purchase: PurchaseDto::try_from(v.purchase)?,
            lines: v.lines.into_iter().map(PurchaseLineDto::from).collect(),
            receipts: v
                .receipts
                .into_iter()
                .map(|r| PurchaseReceiptDto {
                    id: r.receipt.id,
                    series: r.receipt.series,
                    number: r.receipt.number,
                    received_at: r.receipt.received_at.format(DATE_TIME_FORMAT).to_string(),
                    user_id: r.receipt.user_id,
                    note: r.receipt.note,
                    lines: r
                        .lines
                        .into_iter()
                        .map(|l| PurchaseReceiptLineDto {
                            purchase_line_id: l.purchase_line_id,
                            qty_milli: l.qty_milli,
                        })
                        .collect(),
                })
                .collect(),
        })
    }
}

/// One line of an order as the form sends it.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "NewPurchaseLineDto.ts")]
#[serde(deny_unknown_fields)]
pub struct NewPurchaseLineDto {
    pub product_id: i32,
    pub qty_ordered_milli: i64,
    pub unit_cost_centimes: i64,
}

/// Money handed to the supplier as the order is written. The mode is on it
/// because the ledger's file refuses a payment that does not say how it was
/// taken.
#[derive(Debug, Clone, Copy, Deserialize, TS)]
#[ts(export_to = "PaidNowDto.ts")]
#[serde(deny_unknown_fields)]
pub struct PaidNowDto {
    pub amount_centimes: i64,
    pub payment_mode: PaymentMethodDto,
}

/// An order as the screen sends it. `receive_now` is the common case of
/// features.md §1: the goods came with the paper, so the whole receipt is
/// written in the same transaction.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "NewPurchaseDto.ts")]
#[serde(deny_unknown_fields)]
pub struct NewPurchaseDto {
    pub supplier_id: i32,
    #[serde(default)]
    pub supplier_document_number: Option<String>,
    pub purchase_date: String,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub transport_centimes: i64,
    #[serde(default)]
    pub extra_costs_centimes: i64,
    #[serde(default)]
    pub note: Option<String>,
    pub lines: Vec<NewPurchaseLineDto>,
    #[serde(default)]
    pub paid_now: Option<PaidNowDto>,
    #[serde(default)]
    pub receive_now: bool,
}

impl NewPurchaseDto {
    /// The order as the core takes it. Every amount is checked against the
    /// safe-integer bound here, at the edge, like every other one on the
    /// wire; what the amounts mean is the core's business.
    pub fn into_core(self) -> Result<NewPurchase, ApiError> {
        let mut lines = Vec::with_capacity(self.lines.len());
        for line in self.lines {
            lines.push(NewLine {
                product_id: line.product_id,
                qty_ordered_milli: line.qty_ordered_milli,
                unit_cost: Money::centimes(within_js_safe_range(
                    "unit_cost_centimes",
                    line.unit_cost_centimes,
                )?),
            });
        }
        let paid_now = match self.paid_now {
            None => None,
            Some(paid) => Some(Paid {
                amount: Money::centimes(within_js_safe_range(
                    "paid_now_centimes",
                    paid.amount_centimes,
                )?),
                mode: paid.payment_mode.into(),
            }),
        };
        Ok(NewPurchase {
            supplier_id: self.supplier_id,
            supplier_document_number: self.supplier_document_number,
            purchase_date: self.purchase_date,
            due_date: self.due_date,
            transport: Money::centimes(within_js_safe_range(
                "transport_centimes",
                self.transport_centimes,
            )?),
            extra_costs: Money::centimes(within_js_safe_range(
                "extra_costs_centimes",
                self.extra_costs_centimes,
            )?),
            note: self.note,
            lines,
            paid_now,
            receive_now: self.receive_now,
        })
    }
}

/// How much of one ordered line a delivery took in, or a return sent back.
#[derive(Debug, Clone, Copy, Deserialize, TS)]
#[ts(export_to = "ReceiveLineDto.ts")]
#[serde(deny_unknown_fields)]
pub struct ReceiveLineDto {
    pub purchase_line_id: i32,
    pub qty_milli: i64,
}

impl From<ReceiveLineDto> for ReceiveLine {
    fn from(l: ReceiveLineDto) -> Self {
        ReceiveLine {
            purchase_line_id: l.purchase_line_id,
            qty_milli: l.qty_milli,
        }
    }
}

/// A delivery, or a return: the lines it names and a note if the shop wants
/// to say why. The two carry the same fields because they are the same
/// question asked in two directions, and the route is what says which.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "NewReceiptDto.ts")]
#[serde(deny_unknown_fields)]
pub struct NewReceiptDto {
    pub lines: Vec<ReceiveLineDto>,
    #[serde(default)]
    pub note: Option<String>,
}

/// Why an order was cancelled or closed short. Required, because writing off
/// goods that never came is a decision and the audit log is where it is
/// written down.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "CloseOrderDto.ts")]
#[serde(deny_unknown_fields)]
pub struct CloseOrderDto {
    pub reason: String,
}
