use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = crate::schema::products)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Product {
    pub id: i32,
    pub name: String,
    pub barcode: Option<String>,
    pub cost_price: f64,
    pub selling_price: f64,
    pub quantity_on_hand: i32,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Insertable)]
#[diesel(table_name = crate::schema::products)]
pub struct NewProduct {
    pub name: String,
    pub barcode: Option<String>,
    pub cost_price: f64,
    pub selling_price: f64,
    pub quantity_on_hand: i32,
}

impl Product {
    pub fn margin_pct(&self) -> f64 {
        if self.cost_price <= 0.0 {
            return 0.0;
        }
        ((self.selling_price - self.cost_price) / self.cost_price) * 100.0
    }
}
