import { chromium } from "/home/samir/observeone/projects/ObserveOne-frontend/node_modules/@playwright/test/index.mjs";

const base = process.argv[2] || "http://127.0.0.1:8766/desktop/";
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1366, height: 800 } });
const errors = [];
page.on("pageerror", (e) => errors.push("pageerror: " + e.message));
page.on("console", (m) => m.type() === "error" && errors.push("console: " + m.text()));
page.on("requestfailed", (r) => !r.url().includes("fonts.g") && errors.push("reqfail: " + r.url()));

const text = async (sel) => (await page.locator(sel).first().textContent())?.trim();
await page.goto(base + "#/till");
await page.waitForSelector(".ptile");
await page.click('[data-add="1"]'); // Huile 920 DA tva19
await page.click('[data-add="2"]'); // Sucre 110 DA tva9
await page.click('[data-add="2"]'); // qty 2
await page.click('[data-scan]'); // Eau 40 DA tva19
const cartCount = await text(".cart-head strong");
const net1 = await text(".cart-foot .grand .num");
await page.selectOption("[data-customer]", "1");
await page.click("[data-checkout]");
await page.waitForSelector(".modal");
const stampCash = await text(".modal .totals div:nth-last-child(2) .num");
await page.click('[data-mode="card"]');
const stampCard = await text(".modal .totals div:nth-last-child(2) .num");
await page.click('[data-mode="credit"]');
const creditBlocked = await page.locator("[data-confirm]").isDisabled();
await page.click('[data-mode="cash"]');
for (const k of ["2", "0", "0", "0"]) await page.click(`[data-key="${k}"]`);
const change = await text(".change-box .num");
await page.click("[data-confirm]");
await page.waitForSelector("article.doc-a4");
const number = await text(".inv-bar strong");
const words = await text(".doc-words strong");
await page.click('[data-doc="ticket"]');
await page.waitForSelector("article.doc-80");
await page.click('[data-lang="ar"]');
const dir = await page.evaluate(() => document.documentElement.dir);
const arTitle = await text(".top h1");
await page.screenshot({ path: process.argv[3] || "/tmp/dz-desktop.png" });
await page.click('[data-lang="fr"]');
await page.goto(base + "#/settings");
await page.waitForSelector("[data-stamp]");
await page.uncheck("[data-stamp]");
await page.goto(base + "#/till");
await page.click('[data-add="1"]');
await page.click("[data-checkout]");
const stampOff = await text(".modal .totals div:nth-last-child(2) .num");
await page.keyboard.press("Escape");
await page.goto(base + "#/customers");
await page.click('[data-open-customer="1"]');
const drawerRows = await page.locator(".drawer tbody tr").count();
await page.goto(base + "#/dashboard");
await page.waitForSelector(".spark");

console.log(JSON.stringify({ cartCount, net1, stampCash, stampCard, creditBlocked, change, number, words, dir, arTitle, stampOff, drawerRows, errors }, null, 1));
await browser.close();
process.exit(errors.length ? 1 : 0);
