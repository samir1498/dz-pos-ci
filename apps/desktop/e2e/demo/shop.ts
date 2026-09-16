// The shop every scene performs in, put in place through the API the way
// the till specs do it. Each helper is safe to call twice: the second scene
// on the same shop file finds the row already there and carries on.

import type { APIRequestContext } from "@playwright/test";

import { apiHeaders, apiUrl } from "../api";
import { expect } from "./scene";

export type Product = { name: string; barcode: string; price: number; rate: number };

export const CAFE: Product = { name: "Café moulu 250g", barcode: "6130000000017", price: 42_000, rate: 1900 };
export const LAIT: Product = { name: "Lait 1L", barcode: "6130000000024", price: 14_000, rate: 900 };
export const PAIN: Product = { name: "Pain", barcode: "6130000000031", price: 1_500, rate: 0 };
export const CIMENT: Product = { name: "Ciment 50kg", barcode: "6130000000048", price: 95_000, rate: 1900 };

export type Customer = {
  name: string;
  rc: string | null;
  nif: string | null;
  nis: string | null;
  credit_limit_centimes: number | null;
  warn_threshold_centimes: number | null;
};

/** A company that asks for a facture: its own identifiers go on the paper. */
export const ROUIBA: Customer = {
  name: "SARL Bâtiment Rouiba",
  rc: "16/00-7654321 B 22",
  nif: "000216007654321",
  nis: "098216007654321",
  credit_limit_centimes: null,
  warn_threshold_centimes: null,
};

/** A regular who buys on the book, within a limit the shop set. */
export const MEZIANE: Customer = {
  name: "Épicerie Meziane",
  rc: null,
  nif: null,
  nis: null,
  credit_limit_centimes: 1_000_000,
  warn_threshold_centimes: 700_000,
};

export async function seedProduct(request: APIRequestContext, p: Product): Promise<void> {
  const res = await request.post(`${apiUrl()}/products`, {
    headers: apiHeaders(),
    data: {
      name: p.name,
      barcode: p.barcode,
      category_id: 1,
      unit: "piece",
      cost_centimes: 0,
      selling_centimes: p.price,
      wholesale_centimes: null,
      qty_on_hand_milli: 50_000,
      low_stock_at_milli: 0,
      rate_bps: p.rate,
      active: true,
    },
  });
  // 201 on an empty shop, 409 when the previous scene already put it there.
  expect([201, 409]).toContain(res.status());
}

export async function seedCustomer(request: APIRequestContext, c: Customer): Promise<void> {
  const listed = await request.get(`${apiUrl()}/customers`, { headers: apiHeaders() });
  expect(listed.ok()).toBe(true);
  const rows: { name: string }[] = await listed.json();
  if (rows.some((row) => row.name === c.name)) return;
  const res = await request.post(`${apiUrl()}/customers`, {
    headers: apiHeaders(),
    data: {
      name: c.name,
      party_kind: "company",
      phone: "0555 44 33 22",
      address: c.rc === null ? null : "Zone industrielle, Rouiba",
      rc: c.rc,
      nif: c.nif,
      nis: c.nis,
      ai: null,
      credit_limit_centimes: c.credit_limit_centimes,
      warn_threshold_centimes: c.warn_threshold_centimes,
      notes: null,
      active: true,
      opening_debt_centimes: null,
    },
  });
  expect(res.status()).toBe(201);
}

/** Under the réel a ticket shows its TVA; the IFU shows none. Stated here
 * rather than inherited from whatever ran before. */
export async function reel(request: APIRequestContext): Promise<void> {
  const current = await request.get(`${apiUrl()}/settings`, { headers: apiHeaders() });
  const settings: { regime: { regime: string } } = await current.json();
  if (settings.regime.regime === "reel") return;
  const today = new Date(Date.now() + 3_600_000).toISOString().slice(0, 10);
  const res = await request.post(`${apiUrl()}/settings/regime`, {
    headers: apiHeaders(),
    data: { regime: "reel", valid_from: today },
  });
  expect(res.ok()).toBe(true);
}

/** The seller's block a facture must carry (décret 05-468 art. 3). */
export async function storeBlock(request: APIRequestContext): Promise<void> {
  const res = await request.put(`${apiUrl()}/settings/store`, {
    headers: apiHeaders(),
    data: {
      name: "Supérette El Bahdja",
      rc: "16/00-1234567 B 21",
      nif: "000216001234567",
      nis: "098216001234567",
      ai: "16123456789",
      address: "Rue Didouche Mourad, Alger",
      phone: "021 00 00 00",
    },
  });
  expect(res.status()).toBe(200);
}
