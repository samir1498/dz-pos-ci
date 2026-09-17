//! The dashboard figures and the series behind them.
//!
//! Re-exported by `super`, so every path outside this folder is unchanged.

use super::*;

/// What one stretch of days came to (features.md §1, Dashboard). Every field
/// is derived from the ledgers when the screen asks; no column stores any of
/// it.
///
/// `sales_ttc_centimes` is what the tickets and factures of the period asked
/// for over the counter, credit notes not taken off it. The margin is the
/// other question: `lines_ht_centimes` less `discounts_centimes` is the
/// revenue, `cost_of_goods_centimes` is what those goods cost at the cost
/// they left on, and `margin_centimes` is the difference. A credit note
/// lowers the revenue and the cost together, so what it leaves is the margin
/// of what the customer kept.
#[derive(Debug, Clone, Copy, Serialize, TS)]
#[ts(export_to = "DashboardFiguresDto.ts")]
pub struct DashboardFiguresDto {
    pub sales_ttc_centimes: i64,
    pub sales_count: i64,
    pub lines_ht_centimes: i64,
    pub discounts_centimes: i64,
    pub sales_ht_centimes: i64,
    pub cost_of_goods_centimes: i64,
    pub margin_centimes: i64,
    pub expenses_centimes: i64,
}

impl From<Figures> for DashboardFiguresDto {
    fn from(f: Figures) -> Self {
        DashboardFiguresDto {
            sales_ttc_centimes: f.sales_ttc.as_centimes(),
            sales_count: f.sales_count,
            lines_ht_centimes: f.lines_ht.as_centimes(),
            discounts_centimes: f.discounts.as_centimes(),
            sales_ht_centimes: f.sales_ht.as_centimes(),
            cost_of_goods_centimes: f.cost_of_goods.as_centimes(),
            margin_centimes: f.margin.as_centimes(),
            expenses_centimes: f.expenses.as_centimes(),
        }
    }
}

/// A product the shop is short of: what the count says it has, and the
/// threshold somebody set on the fiche. Quantities are thousandths of the
/// unit, the way every quantity on the wire is.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "LowStockDto.ts")]
pub struct LowStockDto {
    pub product_id: i32,
    pub name: String,
    pub qty_on_hand_milli: i64,
    pub low_stock_at_milli: i64,
}

impl From<LowStock> for LowStockDto {
    fn from(l: LowStock) -> Self {
        LowStockDto {
            product_id: l.product_id,
            name: l.name,
            qty_on_hand_milli: l.qty_on_hand_milli,
            low_stock_at_milli: l.low_stock_at_milli,
        }
    }
}

/// One product's month. `qty_milli` is net of what came back, so a product
/// sold and credited in the same month reads as nothing moved.
///
/// `lines_ht_centimes` is this product's lines and not its share of a
/// remise given off a whole document, which belongs to no line. It is
/// therefore not the same figure as `DashboardFiguresDto::sales_ht_centimes`,
/// and the margins of the products on a month that carried a remise do not
/// add up to that month's margin. The ranking is what these are for.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "TopProductDto.ts")]
pub struct TopProductDto {
    pub product_id: i32,
    pub name: String,
    pub qty_milli: i64,
    pub lines_ht_centimes: i64,
    pub cost_of_goods_centimes: i64,
    pub margin_centimes: i64,
}

impl From<TopProduct> for TopProductDto {
    fn from(p: TopProduct) -> Self {
        TopProductDto {
            product_id: p.product_id,
            name: p.name,
            qty_milli: p.qty_milli,
            lines_ht_centimes: p.lines_ht.as_centimes(),
            cost_of_goods_centimes: p.cost_of_goods.as_centimes(),
            margin_centimes: p.margin.as_centimes(),
        }
    }
}

/// One side of the outstanding money. `parties` counts only those in the red:
/// a customer holding credit is left out rather than netted off, because
/// money the shop owes one of them does not reduce what another one owes.
#[derive(Debug, Clone, Copy, Serialize, TS)]
#[ts(export_to = "OwedDto.ts")]
pub struct OwedDto {
    pub total_centimes: i64,
    pub parties: i64,
}

impl From<Owed> for OwedDto {
    fn from(o: Owed) -> Self {
        OwedDto {
            total_centimes: o.total.as_centimes(),
            parties: o.parties,
        }
    }
}

/// The whole dashboard for one day and the month it falls in on the shop's
/// calendar. The two top lists are the month's, not the day's: a day names
/// too few products for a ranking to say anything.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "DashboardDto.ts")]
pub struct DashboardDto {
    pub day: String,
    pub month: String,
    pub today: DashboardFiguresDto,
    pub this_month: DashboardFiguresDto,
    pub cash_today: CashPositionDto,
    pub cash_this_month: CashPositionDto,
    pub low_stock: Vec<LowStockDto>,
    pub top_by_quantity: Vec<TopProductDto>,
    pub top_by_margin: Vec<TopProductDto>,
    pub customer_debt: OwedDto,
    pub supplier_debt: OwedDto,
    pub open_purchases: i64,
}

impl TryFrom<Dashboard> for DashboardDto {
    type Error = ApiError;

    fn try_from(d: Dashboard) -> Result<Self, ApiError> {
        Ok(DashboardDto {
            day: d.day.format(DATE_FORMAT).to_string(),
            month: d.month.as_text(),
            today: DashboardFiguresDto::from(d.today),
            this_month: DashboardFiguresDto::from(d.this_month),
            cash_today: CashPositionDto::try_from(d.cash_today)?,
            cash_this_month: CashPositionDto::try_from(d.cash_this_month)?,
            low_stock: d.low_stock.into_iter().map(LowStockDto::from).collect(),
            top_by_quantity: d
                .top_by_quantity
                .into_iter()
                .map(TopProductDto::from)
                .collect(),
            top_by_margin: d
                .top_by_margin
                .into_iter()
                .map(TopProductDto::from)
                .collect(),
            customer_debt: OwedDto::from(d.customer_debt),
            supplier_debt: OwedDto::from(d.supplier_debt),
            open_purchases: d.open_purchases,
        })
    }
}

/// One bucket of the dashboard's chart: a day, or the week its days were
/// folded into. `from` and `to` are both included and they are equal on a
/// day, so a tooltip names the range the figure covers rather than the one
/// the screen asked for.
///
/// `cash_in_centimes` is the drawer's side alone, sales paid on the spot plus
/// money handed over against a debt. The card takings are a movement of the
/// bank and not of the till, so they are not in this line; the day's own
/// `/cash` answer is where they are.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "DashboardSeriesPointDto.ts")]
pub struct DashboardSeriesPointDto {
    pub from: String,
    pub to: String,
    pub figures: DashboardFiguresDto,
    pub cash_in_centimes: i64,
}

impl From<SeriesPoint> for DashboardSeriesPointDto {
    fn from(p: SeriesPoint) -> Self {
        DashboardSeriesPointDto {
            from: p.from.format(DATE_FORMAT).to_string(),
            to: p.to.format(DATE_FORMAT).to_string(),
            figures: DashboardFiguresDto::from(p.figures),
            cash_in_centimes: p.cash_in.as_centimes(),
        }
    }
}

/// The dashboard's chart: a stretch of days ending on the day the screen
/// asked about, each on its own and folded into weeks. Both lists run oldest
/// first, and `days` skips nothing: a day the shop sold nothing is a row of
/// zeros, because a gap in a chart reads as a day it was shut.
///
/// The weeks are cut back from `to`, so the last bucket is a whole week of
/// the days the shop is in and the odd ones fall at the far end. Thirty days
/// is four weeks and two days.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "DashboardSeriesDto.ts")]
pub struct DashboardSeriesDto {
    pub from: String,
    pub to: String,
    pub days: Vec<DashboardSeriesPointDto>,
    pub weeks: Vec<DashboardSeriesPointDto>,
}

impl From<Series> for DashboardSeriesDto {
    fn from(s: Series) -> Self {
        DashboardSeriesDto {
            from: s.from.format(DATE_FORMAT).to_string(),
            to: s.to.format(DATE_FORMAT).to_string(),
            days: s
                .days
                .into_iter()
                .map(DashboardSeriesPointDto::from)
                .collect(),
            weeks: s
                .weeks
                .into_iter()
                .map(DashboardSeriesPointDto::from)
                .collect(),
        }
    }
}
