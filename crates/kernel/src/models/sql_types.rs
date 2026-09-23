//! Storage-side types, and the macro that builds one from a `column => "text"`
//! list.
//!
//! Each enum built here is one column with a CHECK behind it. The macro
//! writes the same four impls every time; hand-written, they drifted from
//! the CHECK the moment a variant was added to one and not the other.
//!
//! `Role` is the one enum that stays in this crate (S4 of
//! `a-kernel-crate-and-retail-as-the-first-module`): every other
//! `text_enum!` invocation names a shop concept and lives in
//! `dzpos_retail::models::sql_types` now, built with this macro. The macro
//! itself is exported rather than duplicated there, because a second,
//! hand-copied `text_enum!` is exactly the drift the macro exists to avoid,
//! and `crates/retail` already depends on `crates/kernel`, so the shared
//! machinery has one home in the direction the dependency already points.
//!
//! Every path the macro body writes is absolute (`::diesel::...`,
//! `::serde::...`) rather than a name pulled in by a `use` above, because a
//! `macro_rules!` path written as a bare name resolves at the macro's
//! *definition* site: a bare `deserialize::Result` here would need
//! `dzpos_retail::models::sql_types` to carry the same `use
//! diesel::deserialize` this file used to, which is the same drift risk one
//! level up. Absolute paths need nothing from the caller but the crate
//! itself on its dependency list, which every caller already has.

/// An enum stored as the exact text its column's CHECK allows.
#[macro_export]
macro_rules! text_enum {
    (
        $(#[$meta:meta])*
        $name:ident { $($variant:ident => $stored:literal),+ $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Hash,
            ::serde::Serialize,
            ::serde::Deserialize,
            ::diesel::expression::AsExpression,
            ::diesel::deserialize::FromSqlRow,
        )]
        #[diesel(sql_type = ::diesel::sql_types::Text)]
        #[serde(rename_all = "snake_case")]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            pub const fn as_str(self) -> &'static str {
                match self {
                    $($name::$variant => $stored),+
                }
            }

            pub fn parse(s: &str) -> Option<Self> {
                match s {
                    $($stored => Some($name::$variant),)+
                    _ => None,
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl ::diesel::deserialize::FromSql<::diesel::sql_types::Text, ::diesel::sqlite::Sqlite> for $name {
            fn from_sql(
                value: <::diesel::sqlite::Sqlite as ::diesel::backend::Backend>::RawValue<'_>,
            ) -> ::diesel::deserialize::Result<Self> {
                let raw = <String as ::diesel::deserialize::FromSql<::diesel::sql_types::Text, ::diesel::sqlite::Sqlite>>::from_sql(value)?;
                $name::parse(&raw)
                    .ok_or_else(|| format!(concat!("unknown ", stringify!($name), ": {}"), raw).into())
            }
        }

        impl ::diesel::serialize::ToSql<::diesel::sql_types::Text, ::diesel::sqlite::Sqlite> for $name {
            fn to_sql<'b>(&'b self, out: &mut ::diesel::serialize::Output<'b, '_, ::diesel::sqlite::Sqlite>) -> ::diesel::serialize::Result {
                out.set_value(self.as_str().to_string());
                Ok(::diesel::serialize::IsNull::No)
            }
        }
    };
}

text_enum! {
    /// What a user is allowed to be (features.md §5). The `users.role` CHECK
    /// has allowed exactly these three since the first migration, and this is
    /// the same list on the Rust side.
    ///
    /// The variants are an identity and never a comparison a screen makes: T1
    /// puts a `can(role, permission)` table over them and every caller asks
    /// that, so a rule is written once rather than restated wherever it bites.
    Role {
        Owner => "owner",
        Manager => "manager",
        Cashier => "cashier",
    }
}
