//! Product categories.
//!
//! Re-exported by `super`, so every path outside this folder is unchanged.

use super::*;

/// A category and the rate a product inherits from it. The add-product form
/// reads this list so the rate stops being hardcoded at 19 %.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "CategoryDto.ts")]
pub struct CategoryDto {
    pub id: i32,
    pub shop_id: i32,
    pub name: String,
    pub default_rate_bps: u32,
}

impl From<Category> for CategoryDto {
    fn from(c: Category) -> Self {
        CategoryDto {
            id: c.id,
            shop_id: c.shop_id,
            name: c.name,
            default_rate_bps: c.default_rate_bps.as_u32(),
        }
    }
}
