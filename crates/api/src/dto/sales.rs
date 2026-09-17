//! A sale, its lines, its totals, an avoir and a cancellation.
//!
//! Re-exported by `super`, so every path outside this folder is unchanged.

use super::*;

/// How the till pays (features.md §3). cheque and transfer are parked, so
/// the wire does not offer them even though the column admits them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "PaymentModeDto.ts")]
#[serde(rename_all = "lowercase")]
pub enum PaymentModeDto {
    Cash,
    Card,
    Credit,
}

impl From<PaymentMode> for PaymentModeDto {
    fn from(m: PaymentMode) -> Self {
        match m {
            PaymentMode::Cash => PaymentModeDto::Cash,
            PaymentMode::Card => PaymentModeDto::Card,
            PaymentMode::Credit => PaymentModeDto::Credit,
        }
    }
}

impl From<PaymentModeDto> for PaymentMode {
    fn from(m: PaymentModeDto) -> Self {
        match m {
            PaymentModeDto::Cash => PaymentMode::Cash,
            PaymentModeDto::Card => PaymentMode::Card,
            PaymentModeDto::Credit => PaymentMode::Credit,
        }
    }
}

/// The document kinds of features.md §3. The till issues `ticket` and
/// `facture`; the union is whole so a later milestone adds a screen, not a
/// type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "DocumentKindDto.ts")]
#[serde(rename_all = "snake_case")]
pub enum DocumentKindDto {
    Ticket,
    Facture,
    Proforma,
    BonDeLivraison,
    Avoir,
    BonDeReception,
    Quittance,
}

impl From<DocumentKind> for DocumentKindDto {
    fn from(k: DocumentKind) -> Self {
        match k {
            DocumentKind::Ticket => DocumentKindDto::Ticket,
            DocumentKind::Facture => DocumentKindDto::Facture,
            DocumentKind::Proforma => DocumentKindDto::Proforma,
            DocumentKind::BonDeLivraison => DocumentKindDto::BonDeLivraison,
            DocumentKind::Avoir => DocumentKindDto::Avoir,
            DocumentKind::BonDeReception => DocumentKindDto::BonDeReception,
            DocumentKind::Quittance => DocumentKindDto::Quittance,
        }
    }
}

/// The paper the till rings a basket up on (features.md §3). Three values
/// and not `DocumentKindDto`: an avoir and a bon de livraison are their own
/// writes with their own rules, and a till that could name one on `POST
/// /sales` would be issuing a document nobody asked for.
///
/// A proforma is here because the till is where the basket is. It is the one
/// value that ends in no sale: the core hands it to `services::proforma`,
/// which writes the quotation and moves neither stock nor debt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export_to = "SaleKindDto.ts")]
#[serde(rename_all = "snake_case")]
pub enum SaleKindDto {
    #[default]
    Ticket,
    Facture,
    Proforma,
}

impl From<SaleKindDto> for SaleKind {
    fn from(k: SaleKindDto) -> Self {
        match k {
            SaleKindDto::Ticket => SaleKind::Ticket,
            SaleKindDto::Facture => SaleKind::Facture,
            SaleKindDto::Proforma => SaleKind::Proforma,
        }
    }
}

/// A cancelled document keeps its number and its row (features.md,
/// Numbering row), so the state is on the wire from the first version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "DocumentStatusDto.ts")]
#[serde(rename_all = "lowercase")]
pub enum DocumentStatusDto {
    Issued,
    Cancelled,
}

impl From<DocumentStatus> for DocumentStatusDto {
    fn from(s: DocumentStatus) -> Self {
        match s {
            DocumentStatus::Issued => DocumentStatusDto::Issued,
            DocumentStatus::Cancelled => DocumentStatusDto::Cancelled,
        }
    }
}

/// One sold line, snapshotted at issue: the product may be renamed or
/// deleted and a reprint still shows what the customer was handed.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "SaleLineDto.ts")]
pub struct SaleLineDto {
    pub id: i32,
    pub position: i32,
    pub product_id: Option<i32>,
    pub name: String,
    pub barcode: Option<String>,
    pub qty_milli: i64,
    pub unit_price_centimes: i64,
    pub line_discount_centimes: i64,
    pub rate_bps: u32,
    pub line_total_centimes: i64,
    /// The facture line this one credits, on an avoir line and nowhere else.
    /// The screen showing an avoir beside its facture lines the two up by it.
    pub ref_line_id: Option<i32>,
}

impl From<DocumentLine> for SaleLineDto {
    fn from(l: DocumentLine) -> Self {
        SaleLineDto {
            id: l.id,
            position: l.position,
            product_id: l.product_id,
            name: l.name,
            barcode: l.barcode,
            qty_milli: l.qty_milli,
            unit_price_centimes: l.unit_price.as_centimes(),
            line_discount_centimes: l.line_discount.as_centimes(),
            rate_bps: l.rate_bps.as_u32(),
            line_total_centimes: l.line_total.as_centimes(),
            ref_line_id: l.ref_line_id,
        }
    }
}

/// One row of the TVA recap, stored at issue so a reprint never recomputes
/// it. Empty under the IFU (`an_ifu_facture_names_no_tax_in_any_language`).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "SaleTvaDto.ts")]
pub struct SaleTvaDto {
    pub rate_bps: u32,
    pub base_centimes: i64,
    pub amount_centimes: i64,
}

impl From<TvaLine> for SaleTvaDto {
    fn from(t: TvaLine) -> Self {
        SaleTvaDto {
            rate_bps: t.rate.as_u32(),
            base_centimes: t.base.as_centimes(),
            amount_centimes: t.amount.as_centimes(),
        }
    }
}

/// The totals table of features.md §3, column for column. The amount in
/// words is not here: it is rendered at print time in the print language.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "SaleTotalsDto.ts")]
pub struct SaleTotalsDto {
    pub total_ht_centimes: i64,
    pub discount_centimes: i64,
    pub subtotal_ht_centimes: i64,
    pub tva_centimes: i64,
    pub total_ttc_centimes: i64,
    pub stamp_centimes: i64,
    pub net_to_pay_centimes: i64,
}

/// What the customer owed before this document, what it leaves unpaid, and
/// what they owe now (features.md §3, the balance triple). Stored on the
/// document at issue and never recomputed, so a screen and a reprint say the
/// same thing. `remaining_debt_centimes` is the one of the three that moves
/// afterwards: a payment settles part of a document and the column says how
/// much of it is left. Null on a document that names no customer.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "SaleBalanceDto.ts")]
pub struct SaleBalanceDto {
    pub old_balance_centimes: i64,
    pub remaining_debt_centimes: i64,
    pub total_debt_centimes: i64,
}

/// A sale as the till reads it back: the document, its lines and its TVA
/// recap in one answer, so the receipt view makes one call.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "SaleDto.ts")]
pub struct SaleDto {
    pub id: i32,
    pub shop_id: i32,
    pub kind: DocumentKindDto,
    pub series: String,
    pub number: i64,
    /// The number as it is printed and as a customer quotes it back,
    /// `FA-2026-000001`. Built by the core beside the templates that print it
    /// (`print::number`), so a screen naming a document and the paper in the
    /// customer's hand cannot spell it two ways.
    pub printed_number: String,
    /// `YYYY-MM-DD HH:MM:SS` on the shop's calendar (core, services::clock).
    pub issued_at: String,
    pub user_id: i32,
    pub regime: RegimeDto,
    pub payment_mode: PaymentModeDto,
    pub seller: StoreDto,
    pub customer_id: Option<i32>,
    /// The facture an avoir is written against, null on every other kind.
    /// The screen showing an avoir follows it to name the paper it credits.
    pub ref_document_id: Option<i32>,
    /// The buyer's name as this document printed it, snapshotted at issue.
    /// Null on a ticket sold to whoever walked in. A list naming the customer
    /// reads it from here and never from the fiche: the fiche is edited in
    /// place, and the paper says who it was made out to on the day.
    pub buyer_name: Option<String>,
    /// Null on a document with no customer, which is every cash ticket.
    pub balance: Option<SaleBalanceDto>,
    pub totals: SaleTotalsDto,
    pub tva: Vec<SaleTvaDto>,
    pub tendered_centimes: Option<i64>,
    pub change_centimes: Option<i64>,
    pub status: DocumentStatusDto,
    /// Filled exactly when `status` is `cancelled`: when it was annulled, by
    /// whom, why, and the avoir that carried the money back when one did.
    pub cancellation: Option<SaleCancellationDto>,
    pub lines: Vec<SaleLineDto>,
    /// What cancelling this document would do, so a screen can say it before
    /// it asks. Null on a list and on the answer to a sale: it is a question
    /// about one stored document and it costs a read of that document's credit
    /// notes, so only a read of one document carries it.
    pub cancel_effect: Option<SaleCancelEffectDto>,
    /// What the till should say while still handing over the ticket, null
    /// when there is nothing to say. A read of a stored document carries
    /// none: a warning is about the moment the sale was rung up, not about
    /// the paper.
    pub warning: Option<SaleWarningDto>,
}

/// What cancelling a document would do. A union rather than a word and a
/// nullable amount, so the amount cannot go missing on the one shape that has
/// one, and so the day a fourth effect exists the screens matching on these
/// three stop compiling.
///
/// The screen must not work this out from the document's own fields. A
/// facture whose goods have all come back on earlier credit notes carries
/// debt, was sold on credit and names a customer, and cancelling it does
/// nothing at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export_to = "SaleCancelEffectDto.ts")]
#[serde(tag = "effect", rename_all = "snake_case")]
pub enum SaleCancelEffectDto {
    /// Annulled and nothing moves: every line has already come back.
    NothingToReverse,
    /// The goods go back on the shelf. Nobody was owed anything.
    StockBack,
    /// The goods go back and this much comes off the customer's account. On a
    /// facture that is a numbered avoir; on a ticket it is a ledger row alone,
    /// because an avoir is written against a facture.
    StockBackAndAvoir { amount_centimes: i64 },
}

impl From<CancelEffect> for SaleCancelEffectDto {
    fn from(e: CancelEffect) -> Self {
        match e {
            CancelEffect::NothingToReverse => SaleCancelEffectDto::NothingToReverse,
            CancelEffect::StockBack => SaleCancelEffectDto::StockBack,
            CancelEffect::StockBackAndAvoir { amount } => SaleCancelEffectDto::StockBackAndAvoir {
                amount_centimes: amount.as_centimes(),
            },
        }
    }
}

/// What the till should say about a sale that went through anyway. A union
/// rather than a string, so the day a second warning exists the screens that
/// match on this one stop compiling instead of quietly ignoring it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "SaleWarningDto.ts")]
#[serde(rename_all = "snake_case")]
pub enum SaleWarningDto {
    /// The balance this sale leaves reached the customer's warn threshold.
    NearLimit,
}

impl From<Warning> for SaleWarningDto {
    fn from(w: Warning) -> Self {
        match w {
            Warning::NearLimit => SaleWarningDto::NearLimit,
        }
    }
}

impl From<Document> for SaleDto {
    fn from(d: Document) -> Self {
        SaleDto {
            id: d.id,
            shop_id: d.shop_id,
            printed_number: dzpos_core::print::number(&d),
            kind: d.kind.into(),
            series: d.series,
            number: d.number,
            issued_at: d.issued_at.format(DATE_TIME_FORMAT).to_string(),
            user_id: d.user_id,
            regime: d.regime.into(),
            payment_mode: d.payment_mode.into(),
            seller: StoreDto {
                name: d.seller.name,
                rc: d.seller.rc,
                nif: d.seller.nif,
                nis: d.seller.nis,
                ai: d.seller.ai,
                address: d.seller.address,
                phone: d.seller.phone,
            },
            customer_id: d.customer_id,
            ref_document_id: d.ref_document_id,
            buyer_name: d.buyer.as_ref().map(|b| b.name.clone()),
            balance: d.balance.map(|b| SaleBalanceDto {
                old_balance_centimes: b.old_balance.as_centimes(),
                remaining_debt_centimes: b.remaining_debt.as_centimes(),
                total_debt_centimes: b.total_debt.as_centimes(),
            }),
            totals: SaleTotalsDto {
                total_ht_centimes: d.totals.total_ht.as_centimes(),
                discount_centimes: d.totals.discount.as_centimes(),
                subtotal_ht_centimes: d.totals.subtotal_ht.as_centimes(),
                tva_centimes: d.totals.tva.as_centimes(),
                total_ttc_centimes: d.totals.total_ttc.as_centimes(),
                stamp_centimes: d.totals.stamp.as_centimes(),
                net_to_pay_centimes: d.totals.net_to_pay.as_centimes(),
            },
            tva: d.totals.tva_by_rate.into_iter().map(Into::into).collect(),
            tendered_centimes: d.tendered.map(Money::as_centimes),
            change_centimes: d.change.map(Money::as_centimes),
            status: d.status.into(),
            cancellation: d.cancellation.map(|c| SaleCancellationDto {
                cancelled_at: c.at.format(DATE_TIME_FORMAT).to_string(),
                cancelled_by: c.by,
                reason: c.reason,
                avoir_document_id: c.avoir_document_id,
            }),
            lines: d.lines.into_iter().map(Into::into).collect(),
            cancel_effect: None,
            warning: None,
        }
    }
}

impl From<Sale> for SaleDto {
    fn from(s: Sale) -> Self {
        SaleDto {
            warning: s.warning.map(Into::into),
            ..SaleDto::from(s.document)
        }
    }
}

/// One basket line. `unit_price_centimes` left out takes the product's
/// selling price, so a till that shows the price and one that overrides it
/// send the same shape.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "NewSaleLineDto.ts")]
#[serde(deny_unknown_fields)]
pub struct NewSaleLineDto {
    pub product_id: i32,
    pub qty_milli: i64,
    #[serde(default)]
    pub unit_price_centimes: Option<i64>,
    #[serde(default)]
    pub line_discount_centimes: i64,
}

/// The basket the till posts. `issued_at` is not on the wire: the moment a
/// sale happened is the server's to say, on the shop's calendar, and a till
/// with a wrong clock would otherwise date a fiscal document.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "NewSaleDto.ts")]
#[serde(deny_unknown_fields)]
pub struct NewSaleDto {
    pub lines: Vec<NewSaleLineDto>,
    #[serde(default)]
    pub global_discount_centimes: i64,
    pub payment_mode: PaymentModeDto,
    #[serde(default)]
    pub tendered_centimes: Option<i64>,
    /// Who the sale is made out to. Required on credit; on cash and card it
    /// names the buyer on the document and moves no debt.
    #[serde(default)]
    pub customer_id: Option<i32>,
    /// Sell past the customer's credit limit on purpose. `override` on the
    /// wire because that is what the button says; `override` is a Rust
    /// keyword, so the field is spelled out here and renamed on both sides.
    #[serde(default, rename = "override")]
    #[ts(rename = "override")]
    pub override_credit: bool,
    /// Ticket or facture, decided at the till before the sale is saved
    /// (features.md §3). Left out means a ticket: a sale to a consumer is
    /// the ordinary case and asks nothing of the buyer, so a caller written
    /// before this field existed keeps issuing what it always did.
    #[serde(default)]
    pub kind: SaleKindDto,
    /// Retry key (M7 T4). A caller that got no answer posts the same basket
    /// with the same key and gets the original sale back instead of ringing
    /// twice. Left out means no promise: today's desktop keeps ringing like
    /// it always did.
    #[serde(default)]
    pub idempotency_key: Option<String>,
}

impl TryFrom<NewSaleDto> for NewSale {
    type Error = ApiError;

    fn try_from(d: NewSaleDto) -> Result<Self, ApiError> {
        let mut lines = Vec::with_capacity(d.lines.len());
        for line in d.lines {
            lines.push(NewSaleLine {
                product_id: line.product_id,
                qty_milli: within_js_safe_range("qty_milli", line.qty_milli)?,
                unit_price: line
                    .unit_price_centimes
                    .map(|c| within_js_safe_range("unit_price_centimes", c))
                    .transpose()?
                    .map(Money::centimes),
                line_discount: Money::centimes(within_js_safe_range(
                    "line_discount_centimes",
                    line.line_discount_centimes,
                )?),
            });
        }
        Ok(NewSale {
            lines,
            global_discount: Money::centimes(within_js_safe_range(
                "global_discount_centimes",
                d.global_discount_centimes,
            )?),
            payment_mode: d.payment_mode.into(),
            tendered: d
                .tendered_centimes
                .map(|c| within_js_safe_range("tendered_centimes", c))
                .transpose()?
                .map(Money::centimes),
            customer_id: d.customer_id,
            override_credit: d.override_credit,
            kind: d.kind.into(),
            // The server dates the document (core, services::clock).
            issued_at: None,
        })
    }
}

/// Who the buyer is (features.md §2). Asked for on the fiche, never inferred
/// from whether an RC was typed in: loi 04-02 art. 10 decides ticket against
/// facture by the buyer, so an inference would flip the rule the moment
/// somebody cleared a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "PartyKindDto.ts")]
#[serde(rename_all = "lowercase")]
pub enum PartyKindDto {
    Company,
    Consumer,
}

impl From<PartyKind> for PartyKindDto {
    fn from(k: PartyKind) -> Self {
        match k {
            PartyKind::Company => PartyKindDto::Company,
            PartyKind::Consumer => PartyKindDto::Consumer,
        }
    }
}

impl From<PartyKindDto> for PartyKind {
    fn from(k: PartyKindDto) -> Self {
        match k {
            PartyKindDto::Company => PartyKind::Company,
            PartyKindDto::Consumer => PartyKind::Consumer,
        }
    }
}

/// Why the debt moved (features.md §2). The whole union crosses from the
/// first version: the ledger already holds the `sale` and `payment` rows the
/// till writes, and a screen that met an unknown kind could only refuse the
/// whole answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "DebtKindDto.ts")]
#[serde(rename_all = "lowercase")]
pub enum DebtKindDto {
    Opening,
    Sale,
    Payment,
    Avoir,
    Adjustment,
}

impl From<DebtKind> for DebtKindDto {
    fn from(k: DebtKind) -> Self {
        match k {
            DebtKind::Opening => DebtKindDto::Opening,
            DebtKind::Sale => DebtKindDto::Sale,
            DebtKind::Payment => DebtKindDto::Payment,
            DebtKind::Avoir => DebtKindDto::Avoir,
            DebtKind::Adjustment => DebtKindDto::Adjustment,
        }
    }
}

/// What a cancellation left on the document it annulled (features.md §3).
/// Whole or absent: a screen never has to ask whether the date is there
/// while the reason is not.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "SaleCancellationDto.ts")]
pub struct SaleCancellationDto {
    /// `YYYY-MM-DD HH:MM:SS` on the shop's calendar.
    pub cancelled_at: String,
    pub cancelled_by: i32,
    pub reason: String,
    /// The avoir the cancellation issued, null when there was nothing to
    /// carry back: a cash ticket owed nobody anything.
    pub avoir_document_id: Option<i32>,
}

/// One line of a facture and how much of it is coming back on an avoir. The
/// line is named by id and never by the product on it: a facture carries one
/// product on two lines as soon as a line discount is involved.
#[derive(Debug, Clone, Copy, Deserialize, TS)]
#[ts(export_to = "AvoirLineDto.ts")]
#[serde(deny_unknown_fields)]
pub struct AvoirLineDto {
    pub document_line_id: i32,
    pub qty_milli: i64,
}

/// What is coming back on a credit note. `lines` of null is the whole of what
/// is left on the facture, which is what the "avoir the lot" button sends and
/// what a cancellation uses.
///
/// Which is why a field this type does not know is refused rather than
/// dropped: `line` for `lines` would otherwise read as the whole facture
/// coming back, and a shop asking for one unit of three would have credited
/// all three without being told.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "NewAvoirDto.ts")]
#[serde(deny_unknown_fields)]
pub struct NewAvoirDto {
    pub lines: Option<Vec<AvoirLineDto>>,
    pub reason: Option<String>,
}

impl NewAvoirDto {
    pub fn lines(&self) -> Option<Vec<AvoirLine>> {
        self.lines.as_ref().map(|lines| {
            lines
                .iter()
                .map(|l| AvoirLine {
                    document_line_id: l.document_line_id,
                    qty_milli: l.qty_milli,
                })
                .collect()
        })
    }
}

/// Why a document is being annulled. Required: a document annulled for no
/// stated reason is what features.md §5 keeps a log against.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "CancelDocumentDto.ts")]
#[serde(deny_unknown_fields)]
pub struct CancelDocumentDto {
    pub reason: String,
}
