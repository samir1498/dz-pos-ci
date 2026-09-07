// Printed-document renderers: A4 facture and 80 mm ticket.
// Pure function of (sale, lang) — same contract as crates/core::documents.
import { STORE } from "./data.js";
import { amountInWords, fmt } from "./money.js";
import { t } from "./i18n.js";

const esc = (s) =>
  String(s ?? "").replace(/[&<>"]/g, (ch) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[ch]);

function partyBlock(title, p) {
  const rows = [
    ["RC", p.rc],
    ["NIF", p.nif],
    ["NIS", p.nis],
    ["AI", p.ai],
  ].filter(([, v]) => v);
  return `<div class="doc-block"><h3>${title}</h3>
    <div><strong>${esc(p.name)}</strong></div>
    ${p.address ? `<div>${esc(p.address)}</div>` : ""}
    ${p.phone ? `<div>${esc(p.phone)}</div>` : ""}
    ${rows.map(([k, v]) => `<div><span class="muted">${k}</span> <span class="num">${esc(v)}</span></div>`).join("")}
  </div>`;
}

function totalsRows(tot, lang, showTva = true) {
  const r = [];
  r.push(`<div><span>${t("total_ht")}</span><span class="num">${fmt(tot.totalHt, lang)}</span></div>`);
  if (tot.discount) r.push(`<div><span>${t("discount")}</span><span class="num">−${fmt(tot.discount, lang)}</span></div>`);
  if (showTva)
    for (const g of tot.tvaByRate)
      r.push(`<div><span>${t("tva")} ${g.rate}%</span><span class="num">${fmt(g.amount, lang)}</span></div>`);
  else r.push(`<div><span>${t("tva")}</span><span class="num">${fmt(tot.tva, lang)}</span></div>`);
  r.push(`<div><span>${t("total_ttc")}</span><span class="num">${fmt(tot.totalTtc, lang)}</span></div>`);
  if (tot.stamp) r.push(`<div><span>${t("stamp")}</span><span class="num">${fmt(tot.stamp, lang)}</span></div>`);
  r.push(`<div class="grand"><span>${t("net_to_pay")}</span><span class="num">${fmt(tot.netToPay, lang)}</span></div>`);
  return r.join("");
}

export function renderA4(sale, lang) {
  const { lines, totals, customer, number, date, paymentMode } = sale;
  const buyer = customer ?? { name: t("walk_in") };
  const oldBalance = customer?.debt ?? 0;
  const remaining = paymentMode === "credit" ? totals.netToPay : 0;
  return `<article class="doc doc-a4" dir="${lang === "ar" ? "rtl" : "ltr"}">
    <header style="display:flex;justify-content:space-between;align-items:flex-start">
      <div><h1>${t("invoice").toUpperCase()}</h1>
        <div class="num">N° ${esc(number)}</div>
        <div class="num">${esc(date)}</div></div>
      <div style="text-align:end"><strong>${esc(STORE.name)}</strong><div>${esc(STORE.address)}</div><div class="num">${esc(STORE.phone)}</div></div>
    </header>
    <div class="doc-grid" style="margin-block-start:8mm">
      ${partyBlock(t("seller"), STORE)}
      ${partyBlock(t("buyer"), buyer)}
    </div>
    <table>
      <thead><tr><th>${t("designation")}</th><th class="n">${t("qty")}</th><th class="n">${t("unit_price")}</th><th class="n">${t("tva")}</th><th class="n">${t("line_total")}</th></tr></thead>
      <tbody>${lines
        .map(
          (l) => `<tr><td>${esc(lang === "ar" ? l.product.ar : l.product.name)}</td>
          <td class="n num">${l.qty}</td><td class="n num">${fmt(l.unitPrice, lang)}</td>
          <td class="n num">${l.tvaRate}%</td><td class="n num">${fmt(l.qty * l.unitPrice - (l.lineDiscount || 0), lang)}</td></tr>`,
        )
        .join("")}</tbody>
    </table>
    <div class="doc-totals">${totalsRows(totals, lang, true)}</div>
    <p class="doc-words">${t("in_words")} : <strong>${amountInWords(totals.netToPay, lang)}</strong></p>
    <div class="doc-grid" style="margin-block-start:5mm">
      <div class="doc-block"><h3>${t("payment_mode")}</h3><div>${t(paymentMode)}</div></div>
      <div class="doc-block"><h3>${t("balance")}</h3>
        <div style="display:flex;justify-content:space-between"><span>${t("old_balance")}</span><span class="num">${fmt(oldBalance, lang)}</span></div>
        <div style="display:flex;justify-content:space-between"><span>${t("remaining")}</span><span class="num">${fmt(remaining, lang)}</span></div>
        <div style="display:flex;justify-content:space-between"><strong>${t("total_debt")}</strong><strong class="num">${fmt(oldBalance + remaining, lang)}</strong></div>
      </div>
    </div>
    <div class="doc-sign"><div>${t("seller")} — ${t("signature")}</div><div>${t("buyer")} — ${t("signature")}</div></div>
  </article>`;
}

export function renderTicket(sale, lang) {
  const { lines, totals, number, date, paymentMode, tendered } = sale;
  const change = paymentMode === "cash" && tendered ? Math.max(0, tendered - totals.netToPay) : 0;
  return `<article class="doc doc-80" dir="${lang === "ar" ? "rtl" : "ltr"}">
    <div class="center"><strong style="font-size:15px">${esc(STORE.name)}</strong><br>${esc(STORE.address)}<br><span class="num">${esc(STORE.phone)}</span>
      <br><span class="num">NIF ${esc(STORE.nif)}</span></div>
    <div class="dashed"></div>
    <div style="display:flex;justify-content:space-between"><span class="num">${esc(number)}</span><span class="num">${esc(date)}</span></div>
    <div class="dashed"></div>
    <table style="margin-block:2mm">
      ${lines
        .map(
          (l) => `<tr><td colspan="2">${esc(lang === "ar" ? l.product.ar : l.product.name)}</td></tr>
        <tr><td class="num muted">${l.qty} × ${fmt(l.unitPrice, lang)}</td><td class="n num">${fmt(l.qty * l.unitPrice - (l.lineDiscount || 0), lang)}</td></tr>`,
        )
        .join("")}
    </table>
    <div class="dashed"></div>
    <div class="doc-totals">${totalsRows(totals, lang, false)}</div>
    <div class="dashed"></div>
    <div style="display:flex;justify-content:space-between"><span>${t("payment_mode")}</span><span>${t(paymentMode)}</span></div>
    ${
      paymentMode === "cash" && tendered
        ? `<div style="display:flex;justify-content:space-between"><span>${t("tendered")}</span><span class="num">${fmt(tendered, lang)}</span></div>
           <div style="display:flex;justify-content:space-between"><span>${t("change")}</span><span class="num">${fmt(change, lang)}</span></div>`
        : ""
    }
    <div class="dashed"></div>
    <div class="center">${t("thank_you")}</div>
  </article>`;
}
