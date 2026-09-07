use diesel::prelude::*;
use dzpos_core::db;
use dzpos_core::models::*;
use dzpos_core::repos::*;
use tempfile::NamedTempFile;

fn setup_db() -> (diesel::SqliteConnection, NamedTempFile) {
    let file = NamedTempFile::new().unwrap();
    let conn = db::open(file.path()).unwrap();
    (conn, file)
}

fn make_product(name: &str) -> NewProduct {
    NewProduct {
        name: name.to_string(),
        barcode: None,
        cost_price: 100.0,
        selling_price: 150.0,
        quantity_on_hand: 0,
    }
}

fn make_supplier(name: &str) -> NewSupplier {
    NewSupplier {
        name: name.to_string(),
        phone: None,
        notes: None,
    }
}

fn make_expense_category(name: &str) -> NewExpenseCategory {
    NewExpenseCategory {
        name: name.to_string(),
    }
}

fn set_stock(conn: &mut diesel::SqliteConnection, product_id: i32, quantity: i32) {
    use dzpos_core::schema::products;
    diesel::update(products::table.find(product_id))
        .set(products::quantity_on_hand.eq(quantity))
        .execute(conn)
        .unwrap();
}

fn make_expense(date: &str, category: &str, amount: f64) -> NewExpense {
    NewExpense {
        date: date.to_string(),
        category: category.to_string(),
        description: None,
        amount,
    }
}

// ─── Product Tests ─────────────────────────────────────────────────

#[test]
fn test_insert_and_find_product() {
    let (mut conn, _f) = setup_db();
    let p = product_repo::insert_product(&mut conn, &make_product("Test Product")).unwrap();
    assert!(p.id > 0);
    assert_eq!(p.name, "Test Product");
    assert_eq!(p.cost_price, 100.0);
    assert_eq!(p.selling_price, 150.0);
    assert_eq!(p.quantity_on_hand, 0);

    let found = product_repo::find_product_by_id(&mut conn, p.id).unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "Test Product");
}

#[test]
fn test_find_product_by_barcode() {
    let (mut conn, _f) = setup_db();
    let p = NewProduct {
        name: "Barcoded".to_string(),
        barcode: Some("123456789".to_string()),
        cost_price: 50.0,
        selling_price: 80.0,
        quantity_on_hand: 0,
    };
    product_repo::insert_product(&mut conn, &p).unwrap();

    let found = product_repo::find_product_by_barcode(&mut conn, "123456789").unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "Barcoded");

    let missing = product_repo::find_product_by_barcode(&mut conn, "000000000").unwrap();
    assert!(missing.is_none());
}

#[test]
fn test_update_product_does_not_change_cost_price() {
    let (mut conn, _f) = setup_db();
    let p = product_repo::insert_product(&mut conn, &make_product("Original")).unwrap();

    let update = NewProduct {
        name: "Renamed".to_string(),
        barcode: None,
        cost_price: 999.0,
        selling_price: 200.0,
        quantity_on_hand: 0,
    };
    product_repo::update_product(&mut conn, p.id, &update).unwrap();

    let updated = product_repo::find_product_by_id(&mut conn, p.id)
        .unwrap()
        .unwrap();
    assert_eq!(updated.name, "Renamed");
    assert_eq!(updated.cost_price, 999.0);
    assert_eq!(updated.selling_price, 200.0);
}

#[test]
fn test_delete_product_returns_bool() {
    let (mut conn, _f) = setup_db();
    let p = product_repo::insert_product(&mut conn, &make_product("ToDelete")).unwrap();
    assert!(product_repo::delete_product(&mut conn, p.id).unwrap());
    assert!(!product_repo::delete_product(&mut conn, 999).unwrap());
    assert!(product_repo::find_product_by_id(&mut conn, p.id)
        .unwrap()
        .is_none());
}

#[test]
fn test_unique_barcode_enforced() {
    let (mut conn, _f) = setup_db();
    product_repo::insert_product(
        &mut conn,
        &NewProduct {
            name: "A".to_string(),
            barcode: Some("same".to_string()),
            cost_price: 10.0,
            selling_price: 20.0,
            quantity_on_hand: 0,
        },
    )
    .unwrap();
    let dup = product_repo::insert_product(
        &mut conn,
        &NewProduct {
            name: "B".to_string(),
            barcode: Some("same".to_string()),
            cost_price: 10.0,
            selling_price: 20.0,
            quantity_on_hand: 0,
        },
    );
    assert!(dup.is_err());
}

// ─── Supplier Tests ─────────────────────────────────────────────────

#[test]
fn test_supplier_crud() {
    let (mut conn, _f) = setup_db();
    let s = supplier_repo::insert_supplier(&mut conn, &make_supplier("Fournisseur Alpha")).unwrap();
    assert!(s.id > 0);

    let found = supplier_repo::find_supplier_by_id(&mut conn, s.id).unwrap();
    assert!(found.is_some());

    let all = supplier_repo::find_all_suppliers(&mut conn).unwrap();
    assert_eq!(all.len(), 1);

    let update = NewSupplier {
        name: "Alpha Updated".to_string(),
        phone: Some("0555".to_string()),
        notes: None,
    };
    supplier_repo::update_supplier(&mut conn, s.id, &update).unwrap();
    let updated = supplier_repo::find_supplier_by_id(&mut conn, s.id)
        .unwrap()
        .unwrap();
    assert_eq!(updated.name, "Alpha Updated");
    assert_eq!(updated.phone, Some("0555".to_string()));

    assert!(supplier_repo::delete_supplier(&mut conn, s.id).unwrap());
    assert!(supplier_repo::find_all_suppliers(&mut conn)
        .unwrap()
        .is_empty());
}

#[test]
fn test_unique_supplier_name_enforced() {
    let (mut conn, _f) = setup_db();
    supplier_repo::insert_supplier(&mut conn, &make_supplier("Unique")).unwrap();
    let dup = supplier_repo::insert_supplier(&mut conn, &make_supplier("Unique"));
    assert!(dup.is_err());
}

// ─── Purchase Order Tests ───────────────────────────────────────────

fn setup_po(conn: &mut diesel::SqliteConnection) -> (PurchaseOrder, Vec<PurchaseOrderItem>) {
    let supplier = supplier_repo::insert_supplier(conn, &make_supplier("Supplier PO")).unwrap();
    let po = purchase_order_repo::insert_purchase_order(
        conn,
        &NewPurchaseOrder {
            purchase_order_number: "PO-TEST-001".to_string(),
            supplier_id: Some(supplier.id),
            status: "Draft".to_string(),
            notes: None,
            total: 0.0,
        },
    )
    .unwrap();

    let items = purchase_order_repo::insert_purchase_order_items(
        conn,
        &[NewPurchaseOrderItem {
            purchase_order_id: po.id,
            product_name: "PO Product".to_string(),
            quantity: 10,
            unit_cost: 50.0,
            line_total: 0.0,
        }],
    )
    .unwrap();

    (po, items)
}

#[test]
fn test_purchase_order_lifecycle() {
    let (mut conn, _f) = setup_db();
    let (po, items) = setup_po(&mut conn);

    assert!(po.id > 0);
    assert_eq!(po.status, "Draft");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].product_name, "PO Product");

    let found = purchase_order_repo::find_purchase_order_by_id(&mut conn, po.id).unwrap();
    assert!(found.is_some());

    let all = purchase_order_repo::find_all_purchase_orders(&mut conn).unwrap();
    assert_eq!(all.len(), 1);

    let found_items = purchase_order_repo::find_items_by_purchase_order(&mut conn, po.id).unwrap();
    assert_eq!(found_items.len(), 1);
}

#[test]
fn test_po_receive_creates_product_and_updates_stock() {
    let (mut conn, _f) = setup_db();
    let (po, _) = setup_po(&mut conn);

    purchase_order_repo::receive_purchase_order(&mut conn, po.id).unwrap();

    let received = purchase_order_repo::find_purchase_order_by_id(&mut conn, po.id)
        .unwrap()
        .unwrap();
    assert_eq!(received.status, "Received");

    let product = product_repo::find_all_products(&mut conn).unwrap();
    assert_eq!(product.len(), 1);
    assert_eq!(product[0].name, "PO Product");
    assert_eq!(product[0].cost_price, 50.0);
    assert_eq!(product[0].quantity_on_hand, 10);
    assert_eq!(product[0].selling_price, 0.0);
}

#[test]
fn test_po_receive_increments_existing_product_stock() {
    let (mut conn, _f) = setup_db();
    product_repo::insert_product(
        &mut conn,
        &NewProduct {
            name: "Existing Item".to_string(),
            barcode: None,
            cost_price: 30.0,
            selling_price: 60.0,
            quantity_on_hand: 0,
        },
    )
    .unwrap();

    let supplier = supplier_repo::insert_supplier(&mut conn, &make_supplier("S")).unwrap();
    let po = purchase_order_repo::insert_purchase_order(
        &mut conn,
        &NewPurchaseOrder {
            purchase_order_number: "PO-TEST-002".to_string(),
            supplier_id: Some(supplier.id),
            status: "Draft".to_string(),
            notes: None,
            total: 0.0,
        },
    )
    .unwrap();

    purchase_order_repo::insert_purchase_order_items(
        &mut conn,
        &[NewPurchaseOrderItem {
            purchase_order_id: po.id,
            product_name: "Existing Item".to_string(),
            quantity: 5,
            unit_cost: 35.0,
            line_total: 0.0,
        }],
    )
    .unwrap();

    purchase_order_repo::receive_purchase_order(&mut conn, po.id).unwrap();

    let product = product_repo::find_all_products(&mut conn).unwrap();
    assert_eq!(product.len(), 1);
    assert_eq!(product[0].quantity_on_hand, 5);
    assert_eq!(product[0].cost_price, 35.0);
}

#[test]
fn test_po_delete_returns_bool() {
    let (mut conn, _f) = setup_db();
    let (po, _) = setup_po(&mut conn);

    assert!(purchase_order_repo::delete_purchase_order(&mut conn, po.id).unwrap());
    assert!(!purchase_order_repo::delete_purchase_order(&mut conn, 999).unwrap());
    assert!(
        purchase_order_repo::find_purchase_order_by_id(&mut conn, po.id)
            .unwrap()
            .is_none()
    );
}

#[test]
fn test_po_count_today() {
    let (mut conn, _f) = setup_db();
    let prefix = "PO-TEST";
    let count = purchase_order_repo::purchase_order_count_today(&mut conn, prefix).unwrap();
    assert_eq!(count, 0);

    setup_po(&mut conn);
    let count = purchase_order_repo::purchase_order_count_today(&mut conn, prefix).unwrap();
    assert_eq!(count, 1);
}

// ─── Expense Category Tests ─────────────────────────────────────────

#[test]
fn test_expense_category_crud() {
    let (mut conn, _f) = setup_db();
    let cat = expense_category_repo::insert_category(
        &mut conn,
        &make_expense_category("Test Electricité"),
    )
    .unwrap();
    assert!(cat.id > 0);

    let all = expense_category_repo::find_all_categories(&mut conn).unwrap();
    assert!(
        all.len() >= 5,
        "expected at least 5 seed categories, got {}",
        all.len()
    );

    assert!(expense_category_repo::delete_category(&mut conn, cat.id).unwrap());
    let after_delete = expense_category_repo::find_all_categories(&mut conn).unwrap();
    assert_eq!(after_delete.len() as i64, all.len() as i64 - 1);
}

// ─── Expense Tests ──────────────────────────────────────────────────

#[test]
fn test_expense_crud() {
    let (mut conn, _f) = setup_db();
    let e =
        expense_repo::insert_expense(&mut conn, &make_expense("2026-06-25", "Livraison", 1500.0))
            .unwrap();
    assert!(e.id > 0);

    let range =
        expense_repo::find_expenses_by_date_range(&mut conn, "2026-06-01", "2026-06-30").unwrap();
    assert_eq!(range.len(), 1);

    let filtered =
        expense_repo::find_expenses_by_category(&mut conn, "Livraison", "2026-06-01", "2026-06-30")
            .unwrap();
    assert_eq!(filtered.len(), 1);

    let empty =
        expense_repo::find_expenses_by_category(&mut conn, "Loyer", "2026-06-01", "2026-06-30")
            .unwrap();
    assert!(empty.is_empty());

    assert!(expense_repo::delete_expense(&mut conn, e.id).unwrap());
    assert!(!expense_repo::delete_expense(&mut conn, 999).unwrap());
}

#[test]
fn test_expense_total_by_range() {
    let (mut conn, _f) = setup_db();
    expense_repo::insert_expense(&mut conn, &make_expense("2026-06-25", "Livraison", 500.0))
        .unwrap();
    expense_repo::insert_expense(&mut conn, &make_expense("2026-06-26", "Loyer", 300.0)).unwrap();

    let total =
        expense_repo::expense_total_by_range(&mut conn, "2026-06-25", "2026-06-26").unwrap();
    assert!((total - 800.0).abs() < f64::EPSILON);

    let partial =
        expense_repo::expense_total_by_range(&mut conn, "2026-06-25", "2026-06-25").unwrap();
    assert!((partial - 500.0).abs() < f64::EPSILON);

    let none = expense_repo::expense_total_by_range(&mut conn, "2026-01-01", "2026-01-01").unwrap();
    assert!((none - 0.0).abs() < f64::EPSILON);
}

// ─── Transaction Tests ──────────────────────────────────────────────

#[test]
fn test_transaction_lifecycle() {
    let (mut conn, _f) = setup_db();
    let product = product_repo::insert_product(&mut conn, &make_product("Sold Item")).unwrap();

    let tx = transaction_repo::insert_transaction(
        &mut conn,
        &NewTransaction {
            timestamp: "2026-06-25T10:00:00".to_string(),
            total: 300.0,
        },
    )
    .unwrap();
    assert!(tx.id > 0);

    set_stock(&mut conn, product.id, 10);

    let items = transaction_repo::insert_transaction_items(
        &mut conn,
        &[NewTransactionItem {
            transaction_id: tx.id,
            product_id: product.id,
            product_name: "Sold Item".to_string(),
            quantity: 2,
            selling_price: 150.0,
            cost_price: 100.0,
            line_total: 0.0,
        }],
    )
    .unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].product_name, "Sold Item");
    assert!((items[0].line_total - 300.0).abs() < f64::EPSILON);

    let found = transaction_repo::find_transactions_by_date(&mut conn, "2026-06-25").unwrap();
    assert_eq!(found.len(), 1);

    let found_items = transaction_repo::find_items_by_transaction(&mut conn, tx.id).unwrap();
    assert_eq!(found_items.len(), 1);
}

#[test]
fn test_daily_sales_total() {
    let (mut conn, _f) = setup_db();
    let p = product_repo::insert_product(&mut conn, &make_product("P")).unwrap();
    let tx = transaction_repo::insert_transaction(
        &mut conn,
        &NewTransaction {
            timestamp: "2026-06-25T10:00:00".to_string(),
            total: 500.0,
        },
    )
    .unwrap();
    set_stock(&mut conn, p.id, 10);
    transaction_repo::insert_transaction_items(
        &mut conn,
        &[NewTransactionItem {
            transaction_id: tx.id,
            product_id: p.id,
            product_name: "P".to_string(),
            quantity: 5,
            selling_price: 100.0,
            cost_price: 50.0,
            line_total: 0.0,
        }],
    )
    .unwrap();

    let total = transaction_repo::daily_sales_total(&mut conn, "2026-06-25").unwrap();
    assert!((total - 500.0).abs() < f64::EPSILON);

    let zero = transaction_repo::daily_sales_total(&mut conn, "2026-06-01").unwrap();
    assert!((zero - 0.0).abs() < f64::EPSILON);
}

#[test]
fn test_transaction_count_by_date() {
    let (mut conn, _f) = setup_db();
    let p = product_repo::insert_product(&mut conn, &make_product("P")).unwrap();
    for i in 0..3 {
        let tx = transaction_repo::insert_transaction(
            &mut conn,
            &NewTransaction {
                timestamp: format!("2026-06-25T{:02}:00:00", 10 + i),
                total: 100.0,
            },
        )
        .unwrap();
        set_stock(&mut conn, p.id, 10);
        transaction_repo::insert_transaction_items(
            &mut conn,
            &[NewTransactionItem {
                transaction_id: tx.id,
                product_id: p.id,
                product_name: "P".to_string(),
                quantity: 1,
                selling_price: 100.0,
                cost_price: 50.0,
                line_total: 0.0,
            }],
        )
        .unwrap();
    }

    let count = transaction_repo::transaction_count_by_date(&mut conn, "2026-06-25").unwrap();
    assert_eq!(count, 3);
}

// ─── Business Logic Tests ───────────────────────────────────────────

#[test]
fn test_product_margin_pct_zero_when_cost_zero() {
    let p = Product {
        id: 1,
        name: "Zero Cost".to_string(),
        barcode: None,
        cost_price: 0.0,
        selling_price: 100.0,
        quantity_on_hand: 0,
        created_at: String::new(),
    };
    assert!((p.margin_pct() - 0.0).abs() < f64::EPSILON);

    let p_neg = Product {
        cost_price: -5.0,
        ..p
    };
    assert!((p_neg.margin_pct() - 0.0).abs() < f64::EPSILON);
}

#[test]
fn test_product_margin_pct_correct() {
    let p = Product {
        id: 1,
        name: "Margin Test".to_string(),
        barcode: None,
        cost_price: 100.0,
        selling_price: 150.0,
        quantity_on_hand: 0,
        created_at: String::new(),
    };
    assert!((p.margin_pct() - 50.0).abs() < f64::EPSILON);
}

#[test]
fn test_line_total_computed_in_po_items() {
    let (mut conn, _f) = setup_db();
    let supplier = supplier_repo::insert_supplier(&mut conn, &make_supplier("S")).unwrap();
    let po = purchase_order_repo::insert_purchase_order(
        &mut conn,
        &NewPurchaseOrder {
            purchase_order_number: "LT-TEST".to_string(),
            supplier_id: Some(supplier.id),
            status: "Draft".to_string(),
            notes: None,
            total: 0.0,
        },
    )
    .unwrap();

    // line_total is ignored in the caller, computed server-side
    let items = purchase_order_repo::insert_purchase_order_items(
        &mut conn,
        &[NewPurchaseOrderItem {
            purchase_order_id: po.id,
            product_name: "Item".to_string(),
            quantity: 5,
            unit_cost: 20.0,
            line_total: 999.0,
        }],
    )
    .unwrap();

    assert!((items[0].line_total - 100.0).abs() < f64::EPSILON);
}

#[test]
fn test_line_total_computed_in_transaction_items() {
    let (mut conn, _f) = setup_db();
    let p = product_repo::insert_product(&mut conn, &make_product("P")).unwrap();
    let tx = transaction_repo::insert_transaction(
        &mut conn,
        &NewTransaction {
            timestamp: "2026-06-25T12:00:00".to_string(),
            total: 0.0,
        },
    )
    .unwrap();

    set_stock(&mut conn, p.id, 10);
    let items = transaction_repo::insert_transaction_items(
        &mut conn,
        &[NewTransactionItem {
            transaction_id: tx.id,
            product_id: p.id,
            product_name: "P".to_string(),
            quantity: 5,
            selling_price: 30.0,
            cost_price: 0.0,
            line_total: 0.0,
        }],
    )
    .unwrap();

    assert!((items[0].line_total - 150.0).abs() < f64::EPSILON);
}

#[test]
fn test_product_name_order() {
    let (mut conn, _f) = setup_db();
    product_repo::insert_product(&mut conn, &make_product("Zebra")).unwrap();
    product_repo::insert_product(&mut conn, &make_product("Apple")).unwrap();

    let all = product_repo::find_all_products(&mut conn).unwrap();
    assert_eq!(all[0].name, "Apple");
    assert_eq!(all[1].name, "Zebra");
}

#[test]
fn test_supplier_name_order() {
    let (mut conn, _f) = setup_db();
    supplier_repo::insert_supplier(&mut conn, &make_supplier("Z")).unwrap();
    supplier_repo::insert_supplier(&mut conn, &make_supplier("A")).unwrap();

    let all = supplier_repo::find_all_suppliers(&mut conn).unwrap();
    assert_eq!(all[0].name, "A");
    assert_eq!(all[1].name, "Z");
}

#[test]
fn test_stock_decrements_on_sale() {
    let (mut conn, _f) = setup_db();
    let product = product_repo::insert_product(&mut conn, &make_product("Widget")).unwrap();
    set_stock(&mut conn, product.id, 20);

    // Sell 7 units
    let tx = transaction_repo::insert_transaction(
        &mut conn,
        &NewTransaction {
            timestamp: "2026-06-25T10:00:00".to_string(),
            total: 0.0,
        },
    )
    .unwrap();
    transaction_repo::insert_transaction_items(
        &mut conn,
        &[NewTransactionItem {
            transaction_id: tx.id,
            product_id: product.id,
            product_name: "Widget".to_string(),
            quantity: 7,
            selling_price: 100.0,
            cost_price: 50.0,
            line_total: 0.0,
        }],
    )
    .unwrap();

    let p = product_repo::find_product_by_id(&mut conn, product.id)
        .unwrap()
        .unwrap();
    assert_eq!(p.quantity_on_hand, 13);
}

#[test]
fn test_insufficient_stock_rolls_back() {
    let (mut conn, _f) = setup_db();
    let product = product_repo::insert_product(&mut conn, &make_product("Fragile")).unwrap();
    set_stock(&mut conn, product.id, 3);

    let tx = transaction_repo::insert_transaction(
        &mut conn,
        &NewTransaction {
            timestamp: "2026-06-25T12:00:00".to_string(),
            total: 0.0,
        },
    )
    .unwrap();

    // Trying to sell 10 when stock is only 3
    let err = transaction_repo::insert_transaction_items(
        &mut conn,
        &[NewTransactionItem {
            transaction_id: tx.id,
            product_id: product.id,
            product_name: "Fragile".to_string(),
            quantity: 10,
            selling_price: 50.0,
            cost_price: 30.0,
            line_total: 0.0,
        }],
    )
    .unwrap_err();

    assert_eq!(err, diesel::result::Error::RollbackTransaction);

    // Stock should remain unchanged
    let p = product_repo::find_product_by_id(&mut conn, product.id)
        .unwrap()
        .unwrap();
    assert_eq!(p.quantity_on_hand, 3);

    // No transaction items should exist
    let items = transaction_repo::find_items_by_transaction(&mut conn, tx.id).unwrap();
    assert!(items.is_empty());
}
