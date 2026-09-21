# Candidate 7: the Odoo shape as Odoo actually does it

D7 brainstorm page, judged against the constraints block in
`context/plans/20260921-whether-dinar-becomes-a-core-and-modules.md`. Written
2026-09-21 from `main` at 97cc5c9, on top of candidates 1, 3, 4 and 5. Every
Odoo quote was fetched 2026-09-21 from `odoo.com/documentation/18.0/`, pages
`developer/reference/backend/{orm,module,testing,security,orm/changelog}.html`,
`developer/reference/user_interface/view_records.html`,
`developer/tutorials/server_framework_101/12_inheritance.html` and
`administration/on_premise/{source,packages}.html`, or from the raw files at
`raw.githubusercontent.com/odoo/odoo/18.0/` with their line numbers; the
other pages are `peps.python.org/pep-0011/`, `pep-0619/` and
`sqlite.org/lang_altertable.html`. Each repo count says its method.

### 1. What Odoo does, in its own words

**A model is a Python class assembled at runtime, per database.**
`odoo/models.py:527`: "The system automatically instantiates every model
once per database. Those instances represent the available models on each
database, and depend on which modules are installed on that database. The
actual class of each instance is built from the Python classes that create
and inherit from the corresponding model." `:410`: the registry class "is
created dynamically when the registry is built."

**`_inherit` without `_name` edits another module's model in place.** ORM
reference, Extension: "When using `_inherit` but leaving out `_name`, the new
model replaces the existing one, essentially extending it in-place. This is
useful to add new fields or methods to existing models (created in other
modules), or to customize or reconfigure them." Methods the same way;
tutorial chapter 12 has an override "call super to execute the parent
method", under a box marked Danger: "It is very important to always call
super() to avoid breaking the flow."

**`_inherits` is the other mechanism, and it is a side table.** ORM
reference, Delegation: "using the `_inherits` a model delegates the lookup
of any field not found on the current model to 'children' models", a
second table with a required `many2one`.

**A field becomes a column at install, by reconciliation.**
`odoo/modules/registry.py:571`, `init_models`: "Call methods `_auto_init` and
`init` on each model to create or update the database tables supporting the
models." `models.py:3429`,
`_auto_init`: "create the corresponding table, create/update the necessary
columns/tables for fields, initialize new columns on existing rows, add
the SQL constraints given on the model". Per field, `odoo/fields.py:1124`:
"the column does not exist, create it", calling `odoo/tools/sql.py:346`
`create_column`, `"ALTER TABLE %s ADD COLUMN %s %s %s"`; `sql.py:507`
adds the foreign key with `ADD FOREIGN KEY`. A diff of the class against
the live schema, not a numbered migration.

**The registry orders modules by `depends`.** Module Manifests reference:
`depends` is "Odoo modules which must be loaded before this one, either
because this module uses features they create or because it alters resources
they define. When a module is installed, all of its dependencies are
installed before it." The same page names the pattern for two modules that
must interact: `auto_install` is for "'link modules' implementing synergetic
integration between two otherwise independent modules", such as `sale_crm`,
"without either sale or crm being aware of one another."

**Views are data and are extended by XML.** View records reference: views
"are specified in XML and stored as records themselves"; "Inheritance
allows for customizing delivered views. It makes it possible, for example,
to add content as modules are installed", through `inherit_id`, `xpath`
and `position` (`inside`, `after`, `before`, `replace`, `attributes`).

**PostgreSQL, Python 3.10, Windows, money and access.** Source install
page: "Odoo uses PostgreSQL as its database management system"; "Odoo
requires Python 3.10 or later to run." Packaged installers page, Windows:
"Windows packaging is offered for the convenience of testing or running
single-user local instances but production deployment is discouraged due
to a number of limitations and risks associated with deploying Odoo on
a Windows platform." `fields.py:1705`: `class Monetary(Field[float])`,
`:1717`: `_column_type = ('numeric', 'numeric')`. Security reference: access
is rows of `ir.model.access`, and "Access rights are additive, a user's
accesses are the union of the accesses they get through all their groups".

**Does adding a module break another?** A sentence saying so in words was not
found on the pages fetched; these were. `models.py:460`, on a field defined
in several modules: "the field's final definition depends on the presence
or absence of each definition class, which itself depends on the modules
loaded in the registry." ORM reference, Models: "you cannot define a field
and a method with the same name, the last one will silently overwrite the
former ones." Testing reference: `at_install` runs "right after the module
installation and before other modules are installed"; `post_install` runs
"after all the modules are installed". A framework that needs both tags is
saying a module that passes alone can fail in company.

### 2. The honest translation to this repo, piece by piece

**The registry and `depends`.** Candidate 4's `fn modules() -> Vec<Box<dyn
Module>>` is a registry with an explicit order fixed at build. Odoo's is
install-time on a live database; with runtime loading ruled out (plan page,
"What Samir decided") it collapses to "which compiled-in modules are on for
this shop file", one table candidate 4 could add at almost no cost. What
does not translate is `_auto_init`. Diesel has
versioned embedded folders, 19 at `crates/core/src/db.rs:7`, run once at
`:102`, one source per call keyed by version (candidate 4 section 5, on
`diesel_migrations` 2.3.2). A reconciler that diffs a Rust struct against
`sqlite_master` and emits `ALTER TABLE` is new infrastructure; Odoo's is
additive with no down, where `crates/core/tests/migration.rs` (55 tests,
4 122 lines by candidate 6) reverts at `:2053` and tests the versioned
shape. The plan page's own line applies: "at that point the dynamic ORM we
were avoiding has been rebuilt by hand".

**A module adds a column to another module's table.** Against a Diesel
`table!` and a hand-written `crates/core/src/schema.rs` (header:
"Hand-written to match that SQL"; 31 `table!` blocks by `grep -c`), four
facts. First, the SQL is fine:
SQLite's `ALTER TABLE` page, section 4, allows `ADD COLUMN` provided a `NOT
NULL` column "must have a default value other than NULL" and, with foreign
keys on (`db.rs:93`), a `REFERENCES` column "must have a default value of
NULL". Second, kernel reads survive: `table!` emits an explicit column tuple,
shipped code has 0 `SELECT *` and 49 `as_select()` calls (`grep -rn` over
`crates/core/src` and `crates/api/src`), so an extra column is invisible to
kernel reads. Third, kernel writes survive under a rule: the 26 `Insertable`
structs under `crates/core/src/models` name their columns, so the module
column is nullable or defaulted. Fourth, the cost: the module cannot read
its own column through the kernel's `Document` (`models/document.rs:132`)
or its `table!` block. It writes a second `table!` for the same SQL table
naming the kernel's columns plus its own, or drops to `sql_query` (29 in
shipped code, `grep -rn`). Two Rust truths for one table means a kernel
column rename compiles in the kernel and fails in the module at runtime,
the opposite of the safety asked for.

**A module overrides another module's method.** Odoo lets an override wrap
any method of any model and choose whether to call `super()`. Rust has
no runtime MRO; the rendering is every service entry point behind a trait
object with a chain of implementers, 35 service files and 16 667 lines by
candidate 5's count. Candidate 4's rendering is the fixed-point version:
`inside(&self, conn, issued: &Issued)` and `after` (its section 4), where
`Issued` is a shared reference so a module cannot change the total, cannot
open its own transaction and cannot skip the kernel's work; Odoo's can.

**Views by XML inheritance.** Dinar's screens are code: 45 route files under
`apps/desktop/src/routes` (`find`, tests excluded), 122 generated DTOs
(`ls packages/shared/src/generated | wc -l`), 818 keys per language (leaf
count of `i18n/en.json`). A column a module adds to `documents` shows on no
screen until somebody edits the retail React file, so the in-place column
only pays when views are data too: the desktop rewritten as a form and list
renderer driven by descriptions. Candidate 4's `tiles()` is the first step,
priced at 0 until two modules share a screen; the full step has no count.

**Permissions.** Odoo's rows of `ir.model.access` are the runtime registry
Samir turned down: one enum at `crates/core/src/services/permissions.rs:39`
(15 variants) and the exhaustive `can` match whose comment says a sixteenth
variant "fails to compile here until it is placed". Decided; cost not paid.

### 3. What candidate 4 already gives, and what it does not

Gives: a registry (`modules()`, explicit order), module-contributed
routes (`routes()` as data, folded into the guarded router), gate rows
(`gates()` concatenated at startup), per-module migrations (`migrations()`,
in registration order), module tables with foreign keys to kernel tables
(which is `_inherits` delegation in SQL), audit actions and settings keys
at 0 cost, chores, numbering, a module error arm, and the `modules_compose`
test that runs the retail suite beside a `fixtures` module. Does not give:
a module adding a column to another module's table, overriding another
module's method, changing another module's screen, or (decided) a permission.

### 4. Does a clinic or a pharmacy need the missing parts

Judged against Anouar's answer: "it does not have to be a doctor, it can
be anything", and the goal is that "adding new modules will not break the
logic". No second product is named, so no column on `documents` has a name
either. A clinic's patient is a `customers` row plus a clinic table keyed by
`customer_id`, which is `_inherits`, and candidate 4 gives it; a pharmacy's
batches are a table keyed by `product_id`. What either would want in place is
a second `kind`
on a document, and that is D3's tear list on `documents`, candidate 1 and
4's shared fork, not a column. The method override is the one piece a trade
might miss: a pharmacy refusing a sale without a prescription is a rule
inside the sale, and candidate 4's `inside` hook returning `Err` is that
rule with the total out of reach. Nobody has asked for the rest.

### 5. The money write as one transaction under a dynamic ORM

Odoo keeps one transaction: every override runs inside the caller's, and its
`TransactionCase` runs a whole class "in a single transaction" (testing
reference). What the dynamic ORM adds is a write-behind cache: ORM changelog
17.1, "The flushing of fields is now done by `execute_query()`", so a write
sits in memory until a query needs it. `repos/counters.rs:25` is three raw
statements inside the caller's transaction (candidate 5) and `sales.rs:228`
one closure on one connection behind one mutex (`crates/api/src/lib.rs:68`);
a hook running raw SQL beside a cached row is a hazard diesel does not have,
because diesel runs the statement when asked. A dynamic ORM rebuilt here
skips the cache, which is candidate 4's hook on a raw connection, or carries
that hazard with it.

### 6. Windows 7

Odoo 18 needs Python 3.10 (quoted above). PEP 11, Microsoft Windows note:
"A new feature release X.Y.0 will support all Windows versions whose
extended support phase has not yet expired." Windows 7's extended support
ended 2020-01-14 (candidate 5 section (a), quoting `dotnet/docs`); PEP 619:
"3.10.0 final: Monday, 2021-10-04". So no Python that runs Odoo 18 targets
Windows 7. PostgreSQL is candidate 3 section 7: the EDB installer page
lists Windows Server 2016 and later and no Windows 7 row. "One process" is
generous: `odoo-bin` plus a PostgreSQL service is the server the constraints
block's first line rules out, on a platform Odoo's own page discourages. A
Rust rebuild on SQLite inherits candidate 1 section 3: no Windows 7 either
way.

### 7. Which of today's guarantees survive

Checked centimes: `Money(i64)` at `crates/core/src/money/mod.rs:27` with
`checked_add` (`:73`), `checked_sub` (`:80`), `checked_mul` (`:88`),
and the file's own line 3, "No `f64` on any path that reaches a total",
against `Monetary(Field[float])`. Survives only if the module boundary keeps
`Money` as a type; a field added at runtime has a column type, not a Rust
type. The exhaustive permission match dies under rows and survives under
Samir's decision. The five source walks fare as under candidate 4 section 7.
Gapless numbering survives only without a write-behind cache (section 5).
Foreign keys: candidate 3 case (a), unchanged, and its zero core-to-trade
edges still hold.

### 8. Pros, cons, the one thing that kills it

Pros. The reference Anouar named, "over about twenty years" (plan page).
`_inherits` and the registry translate to candidate 4 almost for free. Views
as data would let a module change a screen without a React edit; nothing
else does.

Cons. The two mechanisms that make it "the Odoo way", in-place extension
of another module's table and method, are the two that turn compile-time
facts into runtime facts here: two `table!` truths per shared table, every
service behind a `dyn` chain, `Money` reduced to a column type, the desktop
rewritten as a renderer. PostgreSQL and Python are a server on a till.

What kills it. Odoo's safety story is the inverse of Anouar's. Odoo makes
adding a module easy and finding what it broke a matter of running the suite
with everything installed; Dinar makes adding a module a compile error until
it is placed. The one thing Anouar asked for, "adding new modules will not
break the logic", is the one thing the in-place mechanism is built to permit.

### 9. The paragraph for Anouar

What the Odoo way gives that candidate 4 does not: a clinic module could
add a column to the sales table and a button to the sales screen without
anyone touching the retail code, and could wrap the sale itself. What it
would cost him: a database server on every till, or the same machinery
written by hand in Rust; the money type demoted from a checked integer to
a column; the permission list turned into rows that cannot fail to compile;
and Odoo's own habit of running every test twice, alone and with everything
installed, because a module that passes alone can break another. The safety
he asked for is the thing Dinar has today by compile error and source walk,
and candidate 4 keeps it while giving a module its own tables, routes,
gate rows, migrations and a hook inside the sale that can refuse it but
cannot change the total. Take candidate 4; keep this page as the list of
what to add the day a second trade names a column.
