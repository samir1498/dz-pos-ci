import { t, lang, setLang, applyDir, LANGS, LANG_LABEL } from "../shared/i18n.js";
import { MILLI_PER_UNIT, computeTotals, fmt, lineTotal, qtyLabel } from "../shared/money.js";
import { STORE, PRODUCTS, CATEGORIES, CUSTOMERS, LEDGER, nextNumber } from "../shared/data.js";
import { renderTicket } from "../shared/doc.js";

const esc = (s) =>
  String(s ?? "").replace(/[&<>"]/g, (ch) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[ch]);

// ---- state ----
const state = {
  paired: false,
  products: PRODUCTS.map((p) => ({ ...p })),
  cart: [], // { product, qtyMilli, unitPrice, lineDiscount, rateBps }
  customerId: null,
  globalDiscount: 0,
  paymentMode: "cash",
  tendered: 0,
  search: "",
  cat: "all",
  productSearch: "",
  sheet: null, // { kind: "product"|"customer", id }
  lastSale: null,
};

const screenEl = document.getElementById("screen");
const chipEl = document.getElementById("conn-chip");

function route() {
  const h = location.hash.replace(/^#\/?/, "") || "pair";
  if (!state.paired && h !== "pair") return "pair";
  return h;
}

function go(h) {
  location.hash = `#/${h}`;
}

function pname(p) {
  return lang() === "ar" ? p.ar : p.name;
}

function cname(c) {
  return lang() === "ar" ? c.ar : c.name;
}

function catName(c) {
  return c[lang()] ?? c.fr;
}

function customer() {
  return CUSTOMERS.find((c) => c.id === state.customerId) ?? null;
}

// money.js refuses a discount above the basket instead of clamping it. The
// field must not be able to ask for one, and removing a line can shrink the
// basket under a discount already typed, so the clamp lives here.
function cartHt() {
  return computeTotals(state.cart, {
    globalDiscount: 0,
    paymentMode: "cash",
    stampEnabled: false,
    regime: STORE.regime,
  }).totalHt;
}

function totals(mode = state.paymentMode) {
  state.globalDiscount = Math.min(Math.max(state.globalDiscount, 0), cartHt());
  return computeTotals(state.cart, {
    globalDiscount: state.globalDiscount,
    paymentMode: mode,
    stampEnabled: STORE.stampEnabled,
    regime: STORE.regime,
  });
}

function cartCount() {
  return state.cart.reduce((n, l) => n + l.qtyMilli / MILLI_PER_UNIT, 0);
}

function addToCart(p) {
  const line = state.cart.find((l) => l.product.id === p.id);
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
  document.getElementById("phone").appendChild(el);
  setTimeout(() => el.remove(), 1800);
}

function stockTag(p) {
  if (p.qty === 0) return `<span class="tag tag-danger">${t("out")}</span>`;
  if (p.qty <= p.min) return `<span class="tag tag-warn">${t("low")}</span>`;
  return "";
}

function debtTag(c) {
  if (c.debt > c.limit) return `<span class="tag tag-danger">${t("over_limit")}</span>`;
  if (c.debt >= c.warn) return `<span class="tag tag-warn">${t("near_limit")}</span>`;
  return `<span class="tag tag-ok">${t("ok")}</span>`;
}

// ---- chrome ----
function langSwitch() {
  return `<div class="langs" data-role="langs">${LANGS.map(
    (l) => `<button type="button" data-lang="${l}" class="${l === lang() ? "is-active" : ""}">${LANG_LABEL[l]}</button>`,
  ).join("")}</div>`;
}

function appbar(title, leading = "") {
  return `<div class="appbar">
    <div style="display:flex;align-items:center;gap:var(--space-2)">${leading}<h1>${esc(title)}</h1></div>
    ${langSwitch()}
  </div>`;
}

function backBtn(target) {
  return `<button type="button" class="btn btn-ghost btn-sm" data-go="${target}" aria-label="${t("back")}">←</button>`;
}

const TABS = [
  ["till", "till", "🧾"],
  ["products", "products", "📦"],
  ["customers", "customers", "👥"],
  ["more", "more", "⋯"],
];

function tabbar(active) {
  return `<nav class="tabbar">${TABS.map(
    ([id, key, ico]) =>
      `<button type="button" data-go="${id}" class="${active === id ? "is-active" : ""}"><span class="ico">${ico}</span>${t(key)}</button>`,
  ).join("")}</nav>`;
}

function totalsBlock(tot) {
  const rows = [];
  rows.push(`<div><span>${t("total_ht")}</span><span class="num">${fmt(tot.totalHt, lang())}</span></div>`);
  if (tot.discount) rows.push(`<div><span>${t("discount")}</span><span class="num">−${fmt(tot.discount, lang())}</span></div>`);
  for (const g of tot.tvaByRate)
    rows.push(`<div><span>${t("tva")} ${g.rateBps / 100}%</span><span class="num">${fmt(g.amount, lang())}</span></div>`);
  rows.push(`<div><span>${t("total_ttc")}</span><span class="num">${fmt(tot.totalTtc, lang())}</span></div>`);
  if (tot.stamp) rows.push(`<div><span>${t("stamp")}</span><span class="num">${fmt(tot.stamp, lang())}</span></div>`);
  rows.push(`<div class="grand"><span>${t("net_to_pay")}</span><span class="num">${fmt(tot.netToPay, lang())}</span></div>`);
  return `<div class="totals card card-pad" data-role="totals">${rows.join("")}</div>`;
}

// ---- screens ----
function pairScreen() {
  return `${appbar(t("pair_title"))}
  <div class="body">
    <div class="qr">▦</div>
    <p class="muted" style="text-align:center">${t("pair_scan")}</p>
    <div class="pairinfo" data-role="pairinfo" hidden>CAISSE-1 · <span class="num">192.168.1.20:8080</span></div>
    <button type="button" class="btn btn-primary btn-lg btn-block" data-act="pair">${t("pair_demo")}</button>
  </div>`;
}

function tillScreen() {
  const q = state.search.trim().toLowerCase();
  const list = state.products.filter((p) => {
    if (state.cat !== "all" && p.cat !== state.cat) return false;
    if (!q) return true;
    return p.name.toLowerCase().includes(q) || p.ar.includes(q) || p.barcode.includes(q);
  });
  const n = cartCount();
  const tot = totals("cash");
  return `${appbar(t("till"))}
  <div class="body" style="padding-block-end:${n ? "88px" : "var(--space-4)"}">
    <div class="searchrow">
      <input class="input" type="search" data-role="search" placeholder="${t("search_product")}" value="${esc(state.search)}" />
      <button type="button" class="btn btn-secondary" data-act="scan" aria-label="${t("scan")}">▤ ${t("scan")}</button>
    </div>
    <div class="chips">
      <button type="button" data-cat="all" class="${state.cat === "all" ? "is-active" : ""}">${t("all")}</button>
      ${CATEGORIES.map((c) => `<button type="button" data-cat="${c.id}" class="${state.cat === c.id ? "is-active" : ""}">${esc(catName(c))}</button>`).join("")}
    </div>
    <div class="list" data-role="products">
      ${list
        .map(
          (p) => `<button type="button" class="row" data-add="${p.id}">
            <div class="main"><span class="title">${esc(pname(p))}</span><span class="tiny num">${p.barcode} · ${t("stock")} ${p.qty}</span></div>
            ${stockTag(p)}
            <span class="price num">${fmt(p.price, lang())}</span>
          </button>`,
        )
        .join("")}
    </div>
  </div>
  ${
    n
      ? `<div class="stickybar" data-role="stickybar"><span><strong class="num">${n}</strong> ${t("items")} · <strong class="num">${fmt(tot.netToPay, lang())}</strong></span>
         <button type="button" class="btn btn-primary" data-go="cart">${t("view_cart")}</button></div>`
      : ""
  }
  ${tabbar("till")}`;
}

function cartScreen() {
  const tot = totals("cash");
  return `${appbar(t("cart"), backBtn("till"))}
  <div class="body">
    ${
      state.cart.length
        ? `<div class="list">${state.cart
            .map(
              (l) => `<div class="cartline">
              <div class="main"><div class="title">${esc(pname(l.product))}</div>
                <div class="tiny num">${qtyLabel(l.qtyMilli)} × ${fmt(l.unitPrice, lang())} · ${t("tva")} ${l.rateBps / 100}%</div>
                <div class="price num">${fmt(lineTotal(l.unitPrice, l.qtyMilli) - l.lineDiscount, lang())}</div></div>
              <div class="stepper">
                <button type="button" data-dec="${l.product.id}" aria-label="−">−</button>
                <span class="q num">${qtyLabel(l.qtyMilli)}</span>
                <button type="button" data-inc="${l.product.id}" aria-label="+">+</button>
                <button type="button" class="rm" data-rm="${l.product.id}" aria-label="×">×</button>
              </div>
            </div>`,
            )
            .join("")}</div>`
        : `<p class="muted" style="text-align:center;padding:var(--space-8)">${t("cart_empty")}</p>`
    }
    <div class="field"><label>${t("buyer")}</label>
      <select class="select" data-role="customer">
        <option value="">${t("walk_in")}</option>
        ${CUSTOMERS.map((c) => `<option value="${c.id}" ${c.id === state.customerId ? "selected" : ""}>${esc(cname(c))}</option>`).join("")}
      </select></div>
    <div class="field"><label>${t("global_discount")} (DA)</label>
      <input class="input num" type="number" min="0" step="1" data-role="discount" value="${state.globalDiscount / 100}" /></div>
    ${totalsBlock(tot)}
    <button type="button" class="btn btn-primary btn-lg btn-block" data-go="pay" ${state.cart.length ? "" : "disabled"}>${t("checkout")}</button>
    <button type="button" class="btn btn-ghost btn-block" data-act="clear">${t("clear")}</button>
  </div>
  ${tabbar("till")}`;
}

const MODES = ["cash", "card", "cheque", "transfer", "credit"];

function payScreen() {
  const tot = totals();
  const cust = customer();
  let block = "";
  let disabled = false;
  if (state.paymentMode === "credit") {
    if (!cust) {
      block = `<span class="tag tag-warn">${t("credit_needs_customer")}</span>`;
      disabled = true;
    } else if (cust.debt + tot.netToPay > cust.limit) {
      block = `<span class="tag tag-danger">${t("credit_blocked")}</span>`;
      disabled = true;
    }
  }
  const change = Math.max(0, state.tendered - tot.netToPay);
  const remaining = Math.max(0, tot.netToPay - state.tendered);
  return `${appbar(t("payment"), backBtn("cart"))}
  <div class="body">
    <div class="field"><label>${t("payment_mode")}</label>
      <div class="chips" data-role="modes">${MODES.map(
        (m) => `<button type="button" data-mode="${m}" class="${m === state.paymentMode ? "is-active" : ""}">${t(m)}</button>`,
      ).join("")}</div></div>
    ${totalsBlock(tot)}
    ${block ? `<div>${block}</div>` : ""}
    ${
      state.paymentMode === "cash"
        ? `<div class="field"><label>${t("tendered")}</label>
            <div class="bigamount num" data-role="tendered">${fmt(state.tendered, lang())}</div></div>
          <div class="quick">${[500, 1000, 2000]
            .map((d) => `<button type="button" class="btn btn-secondary" data-quick="${d}">${d} DA</button>`)
            .join("")}</div>
          <div class="keypad" data-role="keypad">
            ${["1", "2", "3", "4", "5", "6", "7", "8", "9", "00", "0", "⌫"]
              .map((k) => `<button type="button" data-key="${k}">${k}</button>`)
              .join("")}
          </div>
          <div class="totals card card-pad">
            <div><span>${t("change")}</span><span class="num" data-role="change">${fmt(change, lang())}</span></div>
            ${remaining ? `<div class="muted"><span>${t("remaining")}</span><span class="num">${fmt(remaining, lang())}</span></div>` : ""}
          </div>`
        : ""
    }
    <button type="button" class="btn btn-primary btn-lg btn-block" data-act="confirm" ${disabled ? "disabled" : ""}>${t("confirm_sale")}</button>
  </div>`;
}

function ticketScreen() {
  const sale = state.lastSale;
  if (!sale) return tillScreen();
  return `${appbar(t("ticket"))}
  <div class="body">
    <p class="tag tag-ok" style="justify-self:center">${t("sale_done")} · <span class="num">${esc(sale.number)}</span></p>
    <div class="ticketwrap">${renderTicket(sale, lang())}</div>
    <button type="button" class="btn btn-secondary btn-lg btn-block" data-act="print">${t("print_bt")}</button>
    <button type="button" class="btn btn-primary btn-lg btn-block" data-go="till">${t("new_sale")}</button>
  </div>`;
}

function productsScreen() {
  const q = state.productSearch.trim().toLowerCase();
  const list = state.products.filter(
    (p) => !q || p.name.toLowerCase().includes(q) || p.ar.includes(q) || p.barcode.includes(q),
  );
  return `${appbar(t("products"))}
  <span class="static-note">${t("static")}</span>
  <div class="body">
    <input class="input" type="search" data-role="psearch" placeholder="${t("search_product")}" value="${esc(state.productSearch)}" />
    <div class="list">${list
      .map(
        (p) => `<button type="button" class="row" data-sheet="product" data-id="${p.id}">
          <div class="main"><span class="title">${esc(pname(p))}</span><span class="tiny num">${p.barcode}</span></div>
          ${stockTag(p)}<span class="tiny num">${t("stock")} ${p.qty}</span>
          <span class="price num">${fmt(p.price, lang())}</span></button>`,
      )
      .join("")}</div>
  </div>
  ${sheet()}
  ${tabbar("products")}`;
}

function customersScreen() {
  return `${appbar(t("customers"))}
  <span class="static-note">${t("static")}</span>
  <div class="body">
    <div class="list">${CUSTOMERS.map(
      (c) => `<button type="button" class="row" data-sheet="customer" data-id="${c.id}">
        <div class="main"><span class="title">${esc(cname(c))}</span><span class="tiny num">${esc(c.phone)}</span></div>
        ${debtTag(c)}<span class="price num">${fmt(c.debt, lang())}</span></button>`,
    ).join("")}</div>
  </div>
  ${sheet()}
  ${tabbar("customers")}`;
}

function moreScreen() {
  return `${appbar(t("more"))}
  <span class="static-note">${t("static")}</span>
  <div class="body">
    <div class="card card-pad">
      <div class="kv"><span>${t("language")}</span>${langSwitch()}</div>
      <div class="kv"><span>${t("network")}</span><span class="num">${esc(STORE.server)} · ${esc(STORE.lanHost)}:${STORE.lanPort}</span></div>
      <div class="kv"><span>${t("users")}</span><span>Yacine · ${t("role_cashier")}</span></div>
      <div class="kv"><span>${t("sync_queue")}</span><span class="num">0</span></div>
      <div class="kv"><span>${t("version")}</span><span>mockup</span></div>
    </div>
    <button type="button" class="btn btn-danger btn-block">${t("logout")}</button>
  </div>
  ${tabbar("more")}`;
}

function sheet() {
  if (!state.sheet) return "";
  let body = "";
  if (state.sheet.kind === "product") {
    const p = state.products.find((x) => x.id === state.sheet.id);
    if (!p) return "";
    body = `<h2>${esc(pname(p))}</h2>
      <div class="kv"><span>${t("barcode")}</span><span class="num">${p.barcode}</span></div>
      <div class="kv"><span>${t("category")}</span><span>${esc(catName(CATEGORIES.find((c) => c.id === p.cat)))}</span></div>
      <div class="kv"><span>${t("price")}</span><span class="num">${fmt(p.price, lang())}</span></div>
      <div class="kv"><span>${t("stock")}</span><span class="num">${p.qty} ${stockTag(p)}</span></div>
      <div class="kv"><span>${t("tva_rate")}</span><span class="num">${p.tvaBps / 100}%</span></div>`;
  } else {
    const c = CUSTOMERS.find((x) => x.id === state.sheet.id);
    if (!c) return "";
    const rows = LEDGER[c.id] ?? [];
    let bal = 0;
    body = `<h2>${esc(cname(c))}</h2>
      <div class="kv"><span>${t("debt")}</span><span class="num">${fmt(c.debt, lang())} ${debtTag(c)}</span></div>
      <div class="kv"><span>${t("credit_limit")}</span><span class="num">${fmt(c.limit, lang())}</span></div>
      <h3 class="card-title">${t("ledger")}</h3>
      <table class="table"><thead><tr><th>${t("date")}</th><th>${t("document")}</th><th class="n">${t("amount")}</th><th class="n">${t("balance")}</th></tr></thead>
      <tbody>${rows
        .map((r) => {
          bal += r.debit - r.credit;
          return `<tr><td class="num">${r.date}</td><td class="num">${r.doc}</td><td class="n num">${r.debit ? fmt(r.debit, lang()) : "−" + fmt(r.credit, lang())}</td><td class="n num">${fmt(bal, lang())}</td></tr>`;
        })
        .join("")}</tbody></table>`;
  }
  return `<div class="sheet-scrim" data-act="close-sheet"><div class="sheet" data-role="sheet"><div class="handle"></div>${body}
    <button type="button" class="btn btn-secondary btn-block" data-act="close-sheet">${t("done")}</button></div></div>`;
}

const SCREENS = {
  pair: pairScreen,
  till: tillScreen,
  cart: cartScreen,
  pay: payScreen,
  ticket: ticketScreen,
  products: productsScreen,
  customers: customersScreen,
  more: moreScreen,
};

function render() {
  applyDir();
  const r = route();
  if (r !== location.hash.replace(/^#\/?/, "")) {
    location.hash = `#/${r}`;
    return;
  }
  chipEl.textContent = state.paired ? `${t("connected")} · ${STORE.server}` : "—";
  chipEl.classList.toggle("is-on", state.paired);
  screenEl.innerHTML = (SCREENS[r] ?? tillScreen)();
  screenEl.dataset.screen = r;
}

// ---- events (delegated) ----
screenEl.addEventListener("click", (e) => {
  const el = e.target.closest("[data-lang],[data-go],[data-act],[data-add],[data-cat],[data-inc],[data-dec],[data-rm],[data-mode],[data-key],[data-quick],[data-sheet]");
  if (!el) return;
  const d = el.dataset;
  if (d.lang) {
    setLang(d.lang);
    return render();
  }
  if (d.go) return go(d.go);
  if (d.cat) {
    state.cat = d.cat;
    return render();
  }
  if (d.add) {
    addToCart(state.products.find((p) => p.id === Number(d.add)));
    return render();
  }
  if (d.inc || d.dec || d.rm) {
    const id = Number(d.inc || d.dec || d.rm);
    const i = state.cart.findIndex((l) => l.product.id === id);
    if (i < 0) return;
    if (d.inc) state.cart[i].qtyMilli += MILLI_PER_UNIT;
    else if (d.dec) state.cart[i].qtyMilli -= MILLI_PER_UNIT;
    if (d.rm || state.cart[i].qtyMilli <= 0) state.cart.splice(i, 1);
    return render();
  }
  if (d.mode) {
    state.paymentMode = d.mode;
    return render();
  }
  if (d.key) {
    const cur = String(Math.round(state.tendered / 100));
    let next;
    if (d.key === "⌫") next = cur.slice(0, -1);
    else next = (cur === "0" ? "" : cur) + d.key;
    state.tendered = Number(next || 0) * 100;
    return render();
  }
  if (d.quick) {
    state.tendered += Number(d.quick) * 100;
    return render();
  }
  if (d.sheet) {
    state.sheet = { kind: d.sheet, id: Number(d.id) };
    return render();
  }
  if (d.act === "close-sheet") {
    if (e.target.closest("[data-role=sheet]") && !e.target.closest("button")) return;
    state.sheet = null;
    return render();
  }
  if (d.act === "pair") {
    const info = screenEl.querySelector("[data-role=pairinfo]");
    if (info) info.hidden = false;
    state.paired = true;
    setTimeout(() => go("till"), 500);
    return;
  }
  if (d.act === "scan") {
    const p = state.products.find((x) => x.barcode === "6130002000017");
    if (p) addToCart(p);
    return render();
  }
  if (d.act === "clear") {
    state.cart = [];
    state.globalDiscount = 0;
    return render();
  }
  if (d.act === "confirm") {
    const tot = totals();
    const sale = {
      number: nextNumber("ticket"),
      date: new Date().toISOString().slice(0, 10),
      lines: state.cart.map((l) => ({ ...l })),
      totals: tot,
      customer: customer(),
      paymentMode: state.paymentMode,
      tendered: state.paymentMode === "cash" ? state.tendered : 0,
    };
    for (const l of state.cart) {
      const p = state.products.find((x) => x.id === l.product.id);
      if (p) p.qty = Math.max(0, p.qty - l.qtyMilli / MILLI_PER_UNIT);
    }
    state.lastSale = sale;
    state.cart = [];
    state.globalDiscount = 0;
    state.tendered = 0;
    state.paymentMode = "cash";
    return go("ticket");
  }
  if (d.act === "print") {
    toast("Imprimante BT: OK");
  }
});

screenEl.addEventListener("input", (e) => {
  const role = e.target.dataset.role;
  if (role === "search") {
    state.search = e.target.value;
    const list = screenEl.querySelector("[data-role=products]");
    const tmp = document.createElement("div");
    tmp.innerHTML = tillScreen();
    list.innerHTML = tmp.querySelector("[data-role=products]").innerHTML;
  } else if (role === "psearch") {
    state.productSearch = e.target.value;
    const tmp = document.createElement("div");
    tmp.innerHTML = productsScreen();
    screenEl.querySelector(".list").innerHTML = tmp.querySelector(".list").innerHTML;
  } else if (role === "discount") {
    const asked = Math.max(0, Math.round(Number(e.target.value || 0) * 100));
    state.globalDiscount = Math.min(asked, cartHt());
    // Show what is applied, so the field never claims more than the basket.
    if (state.globalDiscount !== asked) e.target.value = state.globalDiscount / 100;
    screenEl.querySelector("[data-role=totals]").outerHTML = totalsBlock(totals("cash"));
  }
});

screenEl.addEventListener("change", (e) => {
  if (e.target.dataset.role === "customer") {
    state.customerId = e.target.value ? Number(e.target.value) : null;
  }
});

window.addEventListener("hashchange", render);
render();
