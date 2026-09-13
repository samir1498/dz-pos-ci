import { t, lang, setLang, applyDir, LANGS, LANG_LABEL } from "../shared/i18n.js";
import { MILLI_PER_UNIT, computeTotals, fmt, lineTotal, qtyLabel } from "../shared/money.js";
import { STORE, PRODUCTS, CATEGORIES, CUSTOMERS, LEDGER, USERS, DASH, nextNumber } from "../shared/data.js";
import { renderA4, renderTicket } from "../shared/doc.js";

const esc = (s) =>
  String(s ?? "").replace(/[&<>"]/g, (ch) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[ch]);
const pname = (p) => (lang() === "ar" ? p.ar : p.name);
const cname = (c) => (lang() === "ar" ? c.ar : c.name);
const catName = (c) => c[lang()] ?? c.fr;
const today = () => new Date().toISOString().slice(0, 10);

// ---- state ----
const products = PRODUCTS.map((p) => ({ ...p }));
const customers = CUSTOMERS.map((c) => ({ ...c }));
const store = { ...STORE };
const state = {
  cart: [], // {product, qtyMilli, unitPrice, lineDiscount, rateBps}
  customerId: null,
  globalDiscount: 0,
  search: "",
  cat: "all",
  modal: null, // "pay" | null
  pay: { mode: "cash", tendered: 0 },
  lastSale: null,
  docKind: "facture",
  drawer: null, // {type:"product"|"customer", id}
  firewall: true,
  prodSearch: "",
};

const app = document.getElementById("app");

// ---- routing ----
const ROUTES = ["dashboard", "till", "products", "customers", "invoice", "settings"];
function route() {
  const h = location.hash.replace(/^#\/?/, "").split("/")[0];
  return ROUTES.includes(h) ? h : "till";
}
function go(r) {
  location.hash = `/${r}`;
}

// ---- totals ----
// money.js refuses a discount above the basket instead of clamping it. The
// field must not be able to ask for one, and removing a line can shrink the
// basket under a discount already typed, so the clamp lives here.
function cartHt() {
  return computeTotals(state.cart, {
    globalDiscount: 0,
    paymentMode: "cash",
    stampEnabled: false,
    regime: store.regime,
  }).totalHt;
}

function totals(mode = state.pay.mode) {
  state.globalDiscount = Math.min(Math.max(state.globalDiscount, 0), cartHt());
  return computeTotals(state.cart, {
    globalDiscount: state.globalDiscount,
    paymentMode: mode,
    stampEnabled: store.stampEnabled,
    regime: store.regime,
  });
}
const customer = () => customers.find((c) => c.id === state.customerId) ?? null;

// ---- layout ----
function shell(inner, title) {
  const r = route();
  const nav = (id, key) =>
    `<a href="#/${id}" class="${r === id ? "is-active" : ""}"><span class="ico"></span>${t(key)}</a>`;
  return `
  <aside class="side">
    <div class="brand"><span class="brand-mark">D</span>${t("app_name")}</div>
    <nav class="nav">
      ${nav("till", "till")}
      ${nav("dashboard", "dashboard")}
      <div class="nav-group">${t("stock")}</div>
      ${nav("products", "products")}
      <a href="#/products" class="muted"><span class="ico"></span>${t("purchases")}</a>
      <a href="#/products" class="muted"><span class="ico"></span>${t("suppliers")}</a>
      <div class="nav-group">${t("customers")}</div>
      ${nav("customers", "customers")}
      <div class="nav-group">${t("settings")}</div>
      ${nav("settings", "settings")}
    </nav>
    <div class="side-foot">${esc(store.name)}<br><span class="tiny">${t("network_lan")} · ${esc(store.server)}</span></div>
  </aside>
  <header class="top">
    <h1>${title}</h1>
    <div class="top-right">
      <span class="tag tag-ok">● ${t("connected")} · 2</span>
      <div class="seg" data-langs>${LANGS.map((l) => `<button data-lang="${l}" class="${lang() === l ? "is-active" : ""}">${LANG_LABEL[l]}</button>`).join("")}</div>
      <span class="tag">Yacine · ${t("role_cashier")}</span>
    </div>
  </header>
  <main class="main ${r === "till" ? "is-till" : ""}">${inner}</main>
  ${state.modal === "pay" ? payModal() : ""}
  ${state.drawer ? drawer() : ""}`;
}

// ---- screens ----
function dashboard() {
  const top = DASH.topProducts.map((id) => products.find((p) => p.id === id));
  const debt = customers.reduce((s, c) => s + c.debt, 0);
  const low = products.filter((p) => p.qty <= p.min);
  const max = Math.max(...DASH.sparkline);
  return `<span class="static-note">${t("static")}</span>
  <div class="grid-4">
    <div class="card card-pad"><div class="card-title">${t("sales_today")}</div><div class="card-value num">${fmt(DASH.salesToday, lang())}</div><div class="tiny">${t("month")} · <span class="num">${fmt(DASH.salesMonth, lang())}</span></div></div>
    <div class="card card-pad"><div class="card-title">${t("margin_today")}</div><div class="card-value num hl">${fmt(DASH.marginToday, lang())}</div><div class="tiny">15.3 %</div></div>
    <div class="card card-pad"><div class="card-title">${t("customer_debt")}</div><div class="card-value num warn-text">${fmt(debt, lang())}</div><div class="tiny">${customers.filter((c) => c.debt > 0).length} ${t("customers").toLowerCase()}</div></div>
    <div class="card card-pad"><div class="card-title">${t("cash_position")}</div><div class="card-value num">${fmt(DASH.cashPosition, lang())}</div><div class="tiny">${t("expenses")} ${t("month").toLowerCase()} · <span class="num">${fmt(DASH.expensesMonth, lang())}</span></div></div>
  </div>
  <div class="grid-2" style="margin-block-start:var(--space-4)">
    <div class="card card-pad"><div class="card-title" style="margin-block-end:var(--space-3)">${t("sales_today")} — 7 j</div>
      <div class="spark">${DASH.sparkline.map((v) => `<span style="height:${(v / max) * 100}%"></span>`).join("")}</div></div>
    <div class="card"><div class="card-pad card-title">${t("low_stock")} (${low.length})</div>
      <table class="table"><tbody>${low.map((p) => `<tr><td>${esc(pname(p))}</td><td class="n num">${p.qty} / ${p.min}</td><td class="n">${stockTag(p)}</td></tr>`).join("")}</tbody></table></div>
  </div>
  <div class="card" style="margin-block-start:var(--space-4)"><div class="card-pad card-title">${t("top_products")}</div>
    <table class="table"><thead><tr><th>${t("designation")}</th><th class="n">${t("qty")}</th><th class="n">${t("total")}</th></tr></thead>
    <tbody>${top.map((p, i) => `<tr><td>${esc(pname(p))}</td><td class="n num">${[184, 122, 97, 88, 41][i]}</td><td class="n num">${fmt(p.price * [184, 122, 97, 88, 41][i], lang())}</td></tr>`).join("")}</tbody></table></div>`;
}

function stockTag(p) {
  if (p.qty === 0) return `<span class="tag tag-danger">${t("out")}</span>`;
  if (p.qty <= p.min) return `<span class="tag tag-warn">${t("low")}</span>`;
  return `<span class="tag tag-ok">${t("ok")}</span>`;
}

function till() {
  const q = state.search.trim().toLowerCase();
  const list = products.filter(
    (p) =>
      (state.cat === "all" || p.cat === state.cat) &&
      (!q || p.name.toLowerCase().includes(q) || p.ar.includes(q) || p.barcode.includes(q)),
  );
  const tot = totals();
  const cust = customer();
  return `<div class="till">
    <section class="till-left">
      <div class="till-search">
        <input class="input input-lg" data-search placeholder="${t("search_product")}" value="${esc(state.search)}" autofocus />
        <button class="btn btn-secondary btn-lg" data-scan title="6130002000017">${t("scan")}</button>
      </div>
      <div class="chips">
        <button class="chip ${state.cat === "all" ? "is-active" : ""}" data-cat="all">${t("all")}</button>
        ${CATEGORIES.map((c) => `<button class="chip ${state.cat === c.id ? "is-active" : ""}" data-cat="${c.id}">${esc(catName(c))}</button>`).join("")}
      </div>
      <div class="pgrid">
        ${list
          .map(
            (p) => `<button class="ptile" data-add="${p.id}" ${p.qty === 0 ? "disabled" : ""}>
            <span class="pname">${esc(pname(p))}</span>
            <span style="display:flex;justify-content:space-between;align-items:center"><span class="pprice num">${fmt(p.price, lang())}</span>${stockTag(p)}</span>
          </button>`,
          )
          .join("")}
      </div>
    </section>
    <aside class="till-right">
      <div class="cart-head">
        <div style="display:flex;justify-content:space-between;align-items:center"><strong>${t("cart")} · ${state.cart.length}</strong>
          <button class="btn btn-ghost btn-sm" data-clear ${state.cart.length ? "" : "disabled"}>${t("clear")}</button></div>
        <select class="select" data-customer>
          <option value="">${t("walk_in")}</option>
          ${customers.map((c) => `<option value="${c.id}" ${state.customerId === c.id ? "selected" : ""}>${esc(cname(c))}${c.debt ? ` — ${fmt(c.debt, lang())}` : ""}</option>`).join("")}
        </select>
        ${cust && cust.debt >= cust.warn ? `<span class="tag ${cust.debt > cust.limit ? "tag-danger" : "tag-warn"}">${t(cust.debt > cust.limit ? "over_limit" : "near_limit")} · ${fmt(cust.debt, lang())} / ${fmt(cust.limit, lang())}</span>` : ""}
      </div>
      <div class="cart-lines">
        ${
          state.cart.length
            ? state.cart
                .map(
                  (l, i) => `<div class="cline">
              <span class="cname">${esc(pname(l.product))}</span>
              <span class="ctotal num">${fmt(lineTotal(l.unitPrice, l.qtyMilli) - l.lineDiscount, lang())}</span>
              <span class="cmeta">
                <span class="qty"><button data-dec="${i}">−</button><span class="num">${qtyLabel(l.qtyMilli)}</span><button data-inc="${i}">+</button></span>
                <span class="num">× ${fmt(l.unitPrice, lang())}</span>
                <span class="tiny">${t("tva")} ${l.rateBps / 100}%</span>
              </span>
              <span style="text-align:end"><button class="btn btn-ghost btn-sm" data-del="${i}">✕</button></span>
            </div>`,
                )
                .join("")
            : `<div class="cart-empty">${t("cart_empty")}</div>`
        }
      </div>
      <div class="cart-foot">
        <div class="field"><label>${t("global_discount")} (DA)</label><input class="input" type="number" min="0" step="1" data-gdisc value="${state.globalDiscount / 100 || ""}" /></div>
        <div class="totals">
          <div><span class="muted">${t("total_ht")}</span><span class="num">${fmt(tot.totalHt, lang())}</span></div>
          ${tot.discount ? `<div><span class="muted">${t("discount")}</span><span class="num">−${fmt(tot.discount, lang())}</span></div>` : ""}
          ${tot.tvaByRate.map((g) => `<div><span class="muted">${t("tva")} ${g.rateBps / 100}%</span><span class="num">${fmt(g.amount, lang())}</span></div>`).join("")}
          ${tot.stamp ? `<div><span class="muted">${t("stamp")}</span><span class="num">${fmt(tot.stamp, lang())}</span></div>` : ""}
          <div class="grand"><span>${t("net_to_pay")}</span><span class="num">${fmt(tot.netToPay, lang())}</span></div>
        </div>
        <button class="btn btn-primary btn-lg btn-block" data-checkout ${state.cart.length ? "" : "disabled"}>${t("checkout")} · <span class="num">${fmt(tot.netToPay, lang())}</span></button>
      </div>
    </aside>
  </div>`;
}

function payModal() {
  const mode = state.pay.mode;
  const tot = totals(mode);
  const cust = customer();
  const tendered = state.pay.tendered;
  const change = tendered - tot.netToPay;
  let block = "";
  if (mode === "credit" && !cust) block = t("credit_needs_customer");
  else if (mode === "credit" && cust && cust.debt + tot.netToPay > cust.limit) block = t("credit_blocked");
  const canConfirm = !block;
  return `<div class="modal-scrim" data-scrim>
    <div class="modal" role="dialog">
      <div class="modal-head"><span>${t("payment")}</span><button class="btn btn-ghost btn-sm" data-close>✕</button></div>
      <div class="modal-body">
        <div class="modes">${["cash", "card", "cheque", "transfer", "credit"].map((m) => `<button data-mode="${m}" class="${mode === m ? "is-active" : ""}">${t(m)}</button>`).join("")}</div>
        <div class="pay-grid" style="margin-block-start:var(--space-5)">
          <div>
            <div class="totals">
              <div><span class="muted">${t("subtotal_ht")}</span><span class="num">${fmt(tot.subtotalHt, lang())}</span></div>
              ${tot.tvaByRate.map((g) => `<div><span class="muted">${t("tva")} ${g.rateBps / 100}%</span><span class="num">${fmt(g.amount, lang())}</span></div>`).join("")}
              <div><span class="muted">${t("total_ttc")}</span><span class="num">${fmt(tot.totalTtc, lang())}</span></div>
              <div><span class="muted">${t("stamp")}</span><span class="num">${tot.stamp ? fmt(tot.stamp, lang()) : "—"}</span></div>
              <div class="grand"><span>${t("net_to_pay")}</span><span class="num">${fmt(tot.netToPay, lang())}</span></div>
            </div>
            ${cust ? `<div style="margin-block-start:var(--space-4)" class="tiny">${esc(cname(cust))} · ${t("debt")} <span class="num">${fmt(cust.debt, lang())}</span> / <span class="num">${fmt(cust.limit, lang())}</span></div>` : ""}
            ${block ? `<div style="margin-block-start:var(--space-3)"><span class="tag tag-danger">${block}</span></div>` : ""}
          </div>
          <div>
            ${
              mode === "cash"
                ? `<div class="field"><label>${t("tendered")}</label><div class="display num">${fmt(tendered, lang())}</div></div>
              <div class="quick">${[500, 1000, 2000].map((v) => `<button class="btn btn-secondary" data-quick="${v * 100}">${v}</button>`).join("")}<button class="btn btn-secondary" data-quick="exact">=</button></div>
              <div class="keypad">${["1", "2", "3", "4", "5", "6", "7", "8", "9", "00", "0", "⌫"].map((k) => `<button data-key="${k}">${k}</button>`).join("")}</div>
              <div class="change-box ${change < 0 ? "is-short" : ""}"><span>${change < 0 ? t("remaining") : t("change")}</span><span class="num">${fmt(Math.abs(change), lang())}</span></div>`
                : `<div class="card card-pad muted">${t(mode)} — ${t("net_to_pay")}: <strong class="num">${fmt(tot.netToPay, lang())}</strong></div>`
            }
          </div>
        </div>
      </div>
      <div class="modal-foot">
        <button class="btn btn-secondary" data-close>${t("cancel")}</button>
        <button class="btn btn-primary btn-lg" data-confirm ${canConfirm ? "" : "disabled"}>${t("confirm_sale")}</button>
      </div>
    </div>
  </div>`;
}

function invoice() {
  const s = state.lastSale;
  if (!s) return `<div class="card card-pad">${t("cart_empty")}</div>`;
  return `<div class="inv-bar">
    <div class="stat-row"><span class="tag tag-ok">${t("sale_done")}</span><strong class="num">${esc(s.number)}</strong></div>
    <div style="display:flex;gap:var(--space-2);align-items:center">
      <div class="seg"><button data-doc="facture" class="${state.docKind === "facture" ? "is-active" : ""}">${t("print_facture")}</button><button data-doc="ticket" class="${state.docKind === "ticket" ? "is-active" : ""}">${t("print_ticket")}</button></div>
      <button class="btn btn-secondary" data-print>${t("print")}</button>
      <button class="btn btn-primary" data-newsale>${t("new_sale")}</button>
    </div>
  </div>
  <div class="inv-wrap">${state.docKind === "facture" ? renderA4(s, lang()) : renderTicket(s, lang())}</div>`;
}

function productsScreen() {
  const q = state.prodSearch.toLowerCase();
  const list = products.filter((p) => !q || p.name.toLowerCase().includes(q) || p.barcode.includes(q) || p.ar.includes(q));
  return `<span class="static-note">${t("static")}</span>
  <div class="page-head"><input class="input" style="max-width:360px" data-psearch placeholder="${t("search_product")}" value="${esc(state.prodSearch)}" /><button class="btn btn-primary" data-open-product="new">+ ${t("add_product")}</button></div>
  <div class="card"><table class="table"><thead><tr><th>${t("designation")}</th><th>${t("barcode")}</th><th>${t("category")}</th><th class="n">${t("cost")}</th><th class="n">${t("price")}</th><th class="n">${t("tva_rate")}</th><th class="n">${t("stock")}</th><th>${t("status")}</th></tr></thead>
  <tbody>${list
    .map(
      (p) => `<tr class="is-clickable" data-open-product="${p.id}"><td><strong>${esc(pname(p))}</strong></td><td class="num muted">${p.barcode}</td><td>${esc(catName(CATEGORIES.find((c) => c.id === p.cat)))}</td>
      <td class="n num">${fmt(p.cost, lang())}</td><td class="n num">${fmt(p.price, lang())}</td><td class="n num">${p.tvaBps / 100} %</td><td class="n num">${p.qty}</td><td>${stockTag(p)}</td></tr>`,
    )
    .join("")}</tbody></table></div>`;
}

function customersScreen() {
  return `<span class="static-note">${t("static")}</span>
  <div class="page-head"><h2 style="font-weight:600">${t("customers")} · ${customers.length}</h2><button class="btn btn-primary" data-open-customer="new">+ ${t("add_customer")}</button></div>
  <div class="card"><table class="table"><thead><tr><th>${t("name")}</th><th>${t("phone")}</th><th>NIF</th><th class="n">${t("debt")}</th><th class="n">${t("credit_limit")}</th><th>${t("status")}</th></tr></thead>
  <tbody>${customers
    .map(
      (c) => `<tr class="is-clickable" data-open-customer="${c.id}"><td><strong>${esc(cname(c))}</strong></td><td class="num muted">${esc(c.phone)}</td><td class="num muted">${esc(c.nif || "—")}</td>
      <td class="n num ${c.debt ? "warn-text" : ""}">${fmt(c.debt, lang())}</td><td class="n num">${fmt(c.limit, lang())}</td>
      <td>${c.debt > c.limit ? `<span class="tag tag-danger">${t("over_limit")}</span>` : c.debt >= c.warn ? `<span class="tag tag-warn">${t("near_limit")}</span>` : `<span class="tag tag-ok">${t("ok")}</span>`}</td></tr>`,
    )
    .join("")}</tbody></table></div>`;
}

function settings() {
  const f = (label, val, w = "") => `<div class="field"><label>${label}</label><input class="input ${w}" value="${esc(val)}" /></div>`;
  return `<div class="settings">
    <span class="static-note">${t("static")} · ${t("tax_settings")} & ${t("network")} interactive</span>
    <div class="card card-pad"><h2>${t("store_info")}</h2>
      <div class="field-row">${f(t("name"), store.name)}${f(t("phone"), store.phone)}${f(t("address"), store.address)}</div>
      <h2 style="margin-block-start:var(--space-4)">${t("fiscal")}</h2>
      <div class="field-row">${f("RC", store.rc, "num")}${f("NIF", store.nif, "num")}${f("NIS", store.nis, "num")}${f("AI", store.ai, "num")}</div>
    </div>
    <div class="card card-pad"><h2>${t("tax_settings")}</h2>
      <div class="field-row">
        <div class="field"><label>${t("default_tva")}</label><select class="select" data-tva>${[1900, 900, 0].map((r) => `<option ${store.defaultTvaBps === r ? "selected" : ""}>${r / 100} %</option>`).join("")}</select></div>
        <label class="switch" style="align-self:end"><input type="checkbox" data-stamp ${store.stampEnabled ? "checked" : ""} /> ${t("stamp_enabled")}</label>
      </div>
    </div>
    <div class="card card-pad"><h2>${t("network")}</h2>
      <div class="netmodes">
        ${[
          ["single", "network_single"],
          ["lan", "network_lan"],
          ["cloud", "network_cloud"],
        ]
          .map(([id, k]) => `<button data-net="${id}" class="${store.network === id ? "is-active" : ""}"><strong>${t(k)}</strong><span class="tiny">${id === "single" ? "SQLite local" : id === "lan" ? `${t("lan_note")}` : `dz-pos cloud — ${t("static")}`}</span></button>`)
          .join("")}
      </div>
      ${
        store.network === "lan"
          ? `<div style="display:flex;gap:var(--space-6);margin-block-start:var(--space-5);align-items:center">
          <div class="qr">QR</div>
          <div><p>${t("pair_hint")}</p><p class="num muted" style="margin-block-start:var(--space-2)">${store.server} · ${store.lanHost}:${store.lanPort}</p>
          <p class="tiny">mDNS _dzpos._tcp · token 4f9c… (60 s)</p></div>
        </div>
        ${state.firewall ? `<div class="banner" style="margin-block-start:var(--space-4)"><span>⚠ ${t("firewall_warn")}</span><button class="btn btn-primary btn-sm" data-allow>${t("allow")}</button></div>` : ""}`
          : ""
      }
    </div>
    <div class="card card-pad"><h2>${t("users")}</h2>
      <table class="table"><tbody>${USERS.map((u) => `<tr><td>${u.name}</td><td>${t(u.role)}</td><td class="n"><button class="btn btn-ghost btn-sm">${t("edit")}</button></td></tr>`).join("")}</tbody></table>
    </div>
  </div>`;
}

function drawer() {
  const d = state.drawer;
  let body = "";
  let title = "";
  if (d.type === "product") {
    const p = products.find((x) => x.id === d.id) ?? { name: "", ar: "", barcode: "", cost: 0, price: 0, tvaBps: store.defaultTvaBps, qty: 0, min: 0, cat: "epicerie" };
    title = d.id === "new" ? t("add_product") : pname(p);
    const f = (label, val, extra = "") => `<div class="field"><label>${label}</label><input class="input" value="${esc(val)}" ${extra} /></div>`;
    body = `${f(t("name"), p.name)}${f("الاسم", p.ar, 'dir="rtl"')}${f(t("barcode"), p.barcode)}
      <div class="field"><label>${t("category")}</label><select class="select">${CATEGORIES.map((c) => `<option ${p.cat === c.id ? "selected" : ""}>${esc(catName(c))}</option>`).join("")}</select></div>
      <div class="field-row">${f(t("cost"), p.cost / 100)}${f(t("price"), p.price / 100)}</div>
      <div class="field-row"><div class="field"><label>${t("tva_rate")}</label><select class="select">${[1900, 900, 0].map((r) => `<option ${p.tvaBps === r ? "selected" : ""}>${r / 100} %</option>`).join("")}</select></div>${f(t("stock"), p.qty)}${f(t("low_stock"), p.min)}</div>`;
  } else {
    const c = customers.find((x) => x.id === d.id);
    title = c ? cname(c) : t("add_customer");
    const led = c ? LEDGER[c.id] ?? [] : [];
    let bal = 0;
    body = c
      ? `<div class="grid-2"><div class="card card-pad"><div class="card-title">${t("debt")}</div><div class="card-value num warn-text">${fmt(c.debt, lang())}</div></div>
        <div class="card card-pad"><div class="card-title">${t("credit_limit")}</div><div class="card-value num">${fmt(c.limit, lang())}</div></div></div>
        <p class="tiny" style="margin-block:var(--space-3)">${esc(c.phone)} ${c.nif ? `· NIF <span class="num">${c.nif}</span>` : ""} ${c.address ? `· ${esc(c.address)}` : ""}</p>
        <h2 style="font-size:var(--text-md);font-weight:600;margin-block:var(--space-3)">${t("ledger")}</h2>
        <table class="table"><thead><tr><th>${t("date")}</th><th>${t("document")}</th><th class="n">${t("amount")}</th><th class="n">${t("balance")}</th></tr></thead>
        <tbody>${led
          .map((r) => {
            bal += r.debit - r.credit;
            return `<tr><td class="num">${r.date}</td><td class="num">${r.doc}</td><td class="n num ${r.credit ? "hl" : ""}">${r.credit ? "−" + fmt(r.credit, lang()) : fmt(r.debit, lang())}</td><td class="n num">${fmt(bal, lang())}</td></tr>`;
          })
          .join("")}</tbody></table>
        <button class="btn btn-secondary btn-block" style="margin-block-start:var(--space-4)">${t("receive_payment")}</button>`
      : `<div class="field-row"><div class="field"><label>${t("name")}</label><input class="input" /></div><div class="field"><label>${t("phone")}</label><input class="input" /></div></div>
        <div class="field-row" style="margin-block-start:var(--space-3)"><div class="field"><label>RC</label><input class="input" /></div><div class="field"><label>NIF</label><input class="input" /></div><div class="field"><label>NIS</label><input class="input" /></div><div class="field"><label>AI</label><input class="input" /></div></div>
        <div class="field-row" style="margin-block-start:var(--space-3)"><div class="field"><label>${t("credit_limit")}</label><input class="input" value="10000" /></div></div>`;
  }
  return `<div class="modal-scrim" data-scrim></div>
  <div class="drawer">
    <div class="modal-head"><span>${esc(title)}</span><button class="btn btn-ghost btn-sm" data-close>✕</button></div>
    <div class="modal-body" style="display:grid;gap:var(--space-3)">${body}</div>
    <div class="modal-foot"><button class="btn btn-secondary" data-close>${t("cancel")}</button><button class="btn btn-primary" data-close>${t("save")}</button></div>
  </div>`;
}

// ---- render ----
const TITLES = { dashboard: "dashboard", till: "till", products: "products", customers: "customers", invoice: "invoice", settings: "settings" };
function render() {
  applyDir();
  const r = route();
  const inner = { dashboard, till, products: productsScreen, customers: customersScreen, invoice, settings }[r]();
  const focus = document.activeElement?.dataset?.search !== undefined;
  app.innerHTML = shell(inner, t(TITLES[r]));
  if (focus) app.querySelector("[data-search]")?.focus();
}

// ---- events ----
function addToCart(id) {
  const p = products.find((x) => x.id === id);
  if (!p || p.qty === 0) return;
  const line = state.cart.find((l) => l.product.id === id);
  if (line) line.qtyMilli += MILLI_PER_UNIT;
  else
    state.cart.push({
      product: p,
      qtyMilli: MILLI_PER_UNIT,
      unitPrice: p.price,
      lineDiscount: 0,
      rateBps: p.tvaBps,
    });
}

function toast(msg) {
  const el = document.createElement("div");
  el.className = "toast";
  el.textContent = msg;
  document.body.appendChild(el);
  setTimeout(() => el.remove(), 1800);
}

function confirmSale() {
  const tot = totals();
  const sale = {
    number: nextNumber(state.customerId ? "facture" : "ticket"),
    date: today(),
    lines: state.cart.map((l) => ({ ...l })),
    totals: tot,
    customer: customer(),
    paymentMode: state.pay.mode,
    tendered: state.pay.mode === "cash" ? state.pay.tendered : 0,
  };
  for (const l of sale.lines) l.product.qty -= l.qtyMilli / MILLI_PER_UNIT;
  if (sale.paymentMode === "credit" && sale.customer) sale.customer.debt += tot.netToPay;
  state.lastSale = sale;
  state.docKind = sale.customer ? "facture" : "ticket";
  state.cart = [];
  state.globalDiscount = 0;
  state.customerId = null;
  state.modal = null;
  state.pay = { mode: "cash", tendered: 0 };
  go("invoice");
  toast(`${t("sale_done")} · ${sale.number}`);
}

app.addEventListener("click", (e) => {
  const el = e.target.closest("[data-lang],[data-add],[data-inc],[data-dec],[data-del],[data-clear],[data-checkout],[data-close],[data-mode],[data-key],[data-quick],[data-confirm],[data-doc],[data-print],[data-newsale],[data-cat],[data-scan],[data-open-product],[data-open-customer],[data-net],[data-allow],[data-scrim]");
  if (!el) return;
  const d = el.dataset;
  if (d.lang) setLang(d.lang);
  else if (d.add) addToCart(Number(d.add));
  else if (d.inc !== undefined) state.cart[d.inc].qtyMilli += MILLI_PER_UNIT;
  else if (d.dec !== undefined) {
    state.cart[d.dec].qtyMilli -= MILLI_PER_UNIT;
    if (state.cart[d.dec].qtyMilli <= 0) state.cart.splice(d.dec, 1);
  } else if (d.del !== undefined) state.cart.splice(d.del, 1);
  else if (d.clear !== undefined) {
    state.cart = [];
    state.globalDiscount = 0;
  } else if (d.checkout !== undefined) {
    state.modal = "pay";
    state.pay = { mode: "cash", tendered: 0 };
  } else if (d.close !== undefined || (d.scrim !== undefined && e.target === el)) {
    state.modal = null;
    state.drawer = null;
  } else if (d.mode) state.pay.mode = d.mode;
  else if (d.key) {
    const cur = String(state.pay.tendered / 100 || "");
    if (d.key === "⌫") state.pay.tendered = Number(cur.slice(0, -1) || 0) * 100;
    else state.pay.tendered = Number((cur + d.key).slice(0, 9)) * 100;
  } else if (d.quick) state.pay.tendered = d.quick === "exact" ? totals().netToPay : state.pay.tendered + Number(d.quick);
  else if (d.confirm !== undefined) return confirmSale();
  else if (d.doc) state.docKind = d.doc;
  else if (d.print !== undefined) return window.print();
  else if (d.newsale !== undefined) return go("till");
  else if (d.cat) state.cat = d.cat;
  else if (d.scan !== undefined) {
    addToCart(7);
    toast("scan · 6130002000017");
  } else if (d.openProduct) state.drawer = { type: "product", id: d.openProduct === "new" ? "new" : Number(d.openProduct) };
  else if (d.openCustomer) state.drawer = { type: "customer", id: d.openCustomer === "new" ? "new" : Number(d.openCustomer) };
  else if (d.net) store.network = d.net;
  else if (d.allow !== undefined) {
    state.firewall = false;
    toast("netsh advfirewall … OK");
  } else return;
  render();
});

app.addEventListener("input", (e) => {
  const el = e.target;
  if (el.dataset.search !== undefined) {
    state.search = el.value;
    // scanner: 13 digits + enter behaves like a scan; here match immediately
    const hit = products.find((p) => p.barcode === el.value.trim());
    if (hit) {
      addToCart(hit.id);
      state.search = "";
      el.value = "";
    }
    render();
  } else if (el.dataset.gdisc !== undefined) {
    state.globalDiscount = Math.min(Math.max(0, Math.round(Number(el.value || 0) * 100)), cartHt());
    render();
    app.querySelector("[data-gdisc]")?.focus();
  } else if (el.dataset.psearch !== undefined) {
    state.prodSearch = el.value;
    render();
    const i = app.querySelector("[data-psearch]");
    i?.focus();
    i?.setSelectionRange(i.value.length, i.value.length);
  }
});

app.addEventListener("change", (e) => {
  const el = e.target;
  if (el.dataset.customer !== undefined) state.customerId = el.value ? Number(el.value) : null;
  else if (el.dataset.stamp !== undefined) store.stampEnabled = el.checked;
  else if (el.dataset.tva !== undefined) store.defaultTvaBps = parseInt(el.value, 10) * 100;
  else return;
  render();
});

document.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && (state.modal || state.drawer)) {
    state.modal = null;
    state.drawer = null;
    render();
  }
  if (e.key === "F9" && route() === "till" && state.cart.length) {
    state.modal = "pay";
    render();
  }
});

window.addEventListener("hashchange", render);
if (!location.hash) location.hash = "/till";
render();
