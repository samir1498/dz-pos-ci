//! A product's pack size, optional (plan shop-manual-test-findings T13): 1,5 L
//! of oil, 250 g of coffee. Shops type it into the name; this gives it a
//! place of its own, appended to the name wherever the name is shown.
//!
//! The quantity is thousandths of the unit, the way every quantity on the
//! file is (migration 000030), so `1,5 L` is `1500` with `L` and no float is
//! ever made from it.

use crate::error::CoreError;
use dzpos_kernel::money::format::format_qty;

/// The four units a pack size is written in. Mass and volume only: a pack of
/// six is a product of its own, and its count is not a contenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContenanceUnit {
    G,
    Kg,
    Ml,
    L,
}

impl ContenanceUnit {
    /// The column value, which is also the symbol a label prints (`L` aside,
    /// whose symbol is upper case so it does not read as the digit one).
    pub const fn as_str(self) -> &'static str {
        match self {
            ContenanceUnit::G => "g",
            ContenanceUnit::Kg => "kg",
            ContenanceUnit::Ml => "ml",
            ContenanceUnit::L => "l",
        }
    }

    /// The symbol shown after the quantity: SI symbols, the same in the three
    /// languages, the way a price's digits are.
    pub const fn symbol(self) -> &'static str {
        match self {
            ContenanceUnit::G => "g",
            ContenanceUnit::Kg => "kg",
            ContenanceUnit::Ml => "mL",
            ContenanceUnit::L => "L",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "g" => Ok(ContenanceUnit::G),
            "kg" => Ok(ContenanceUnit::Kg),
            "ml" => Ok(ContenanceUnit::Ml),
            "l" => Ok(ContenanceUnit::L),
            _ => Err(CoreError::validation(
                "contenance_unit",
                "a pack size is in g, kg, ml or l",
            )),
        }
    }
}

/// How much one unit of the product holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Contenance {
    qty_milli: i64,
    unit: ContenanceUnit,
}

impl Contenance {
    /// Refuses a pack of nothing, which the file refuses too.
    pub fn new(qty_milli: i64, unit: ContenanceUnit) -> Result<Self, CoreError> {
        if qty_milli <= 0 {
            return Err(CoreError::validation(
                "contenance_milli",
                "a pack size is more than nothing",
            ));
        }
        Ok(Contenance { qty_milli, unit })
    }

    /// The two columns read back together. Both set or both empty is what
    /// migration 000030's CHECK holds; one without the other reads as none.
    pub fn from_columns(
        qty_milli: Option<i64>,
        unit: Option<&str>,
    ) -> Result<Option<Self>, CoreError> {
        match (qty_milli, unit) {
            (Some(qty), Some(unit)) => {
                Ok(Some(Contenance::new(qty, ContenanceUnit::parse(unit)?)?))
            }
            _ => Ok(None),
        }
    }

    pub const fn qty_milli(self) -> i64 {
        self.qty_milli
    }

    pub const fn unit(self) -> ContenanceUnit {
        self.unit
    }

    /// `1,5 L`, `250 g`: the quantity the way a person writes it and the
    /// unit's symbol.
    pub fn label(self) -> String {
        format!("{} {}", format_qty(self.qty_milli), self.unit.symbol())
    }
}

/// The name as the screens show it: the name, then the pack size when there
/// is one. The stored name is left as it is, so a product whose name already
/// says `1,5 L` and has no contenance reads the same as before.
pub fn display_name(name: &str, contenance: Option<Contenance>) -> String {
    match contenance {
        Some(c) => format!("{name} {}", c.label()),
        None => name.to_owned(),
    }
}
