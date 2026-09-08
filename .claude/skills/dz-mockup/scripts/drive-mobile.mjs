import { chromium } from "/home/samir/observeone/projects/ObserveOne-frontend/node_modules/@playwright/test/index.mjs";

const base = process.argv[2] || "http://127.0.0.1:8766/mobile/";
const errors = [];
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 430, height: 900 } });
page.on("console", (m) => m.type() === "error" && errors.push(m.text()));
page.on("pageerror", (e) => errors.push(String(e)));
page.on("requestfailed", (r) => !r.url().includes("fonts.g") && errors.push(`REQFAIL ${r.url()}`));

const screen = () => page.getAttribute("#screen", "data-screen");
const step = async (name, fn) => {
  await fn();
  console.log(`ok  ${name} → ${await screen()}`);
};

await page.goto(base + "#/pair");
await step("pair", async () => {
  await page.click("[data-act=pair]");
  await page.waitForFunction(() => document.querySelector("#screen").dataset.screen === "till");
});
await step("add 2 products", async () => {
  await page.click("[data-add='1']");
  await page.click("[data-add='7']");
  await page.waitForSelector("[data-role=stickybar]");
});
await step("search filters", async () => {
  await page.fill("[data-role=search]", "eau ifri");
  const n = await page.locator("[data-role=products] .row").count();
  if (n !== 1) throw new Error(`search expected 1 row, got ${n}`);
  await page.fill("[data-role=search]", "");
});
await step("open cart", async () => {
  await page.click("[data-role=stickybar] [data-go=cart]");
  await page.waitForFunction(() => document.querySelector("#screen").dataset.screen === "cart");
});
await step("qty +", async () => {
  await page.click("[data-inc='7']");
  const q = await page.locator("[data-inc='7']").locator("xpath=preceding-sibling::span").textContent();
  if (q.trim() !== "2") throw new Error(`qty expected 2 got ${q}`);
});
await step("go pay", async () => {
  await page.click("[data-go=pay]");
  await page.waitForFunction(() => document.querySelector("#screen").dataset.screen === "pay");
});
const stampVisible = async () => (await page.locator("[data-role=totals]").textContent()).includes("timbre");
await step("cash shows stamp", async () => {
  if (!(await stampVisible())) throw new Error("stamp missing on cash");
});
await step("card hides stamp", async () => {
  await page.click("[data-mode=card]");
  if (await stampVisible()) throw new Error("stamp still shown on card");
  const kp = await page.locator("[data-role=keypad]").count();
  if (kp !== 0) throw new Error("keypad shown on card");
});
await step("back to cash, keypad 2000", async () => {
  await page.click("[data-mode=cash]");
  await page.click("[data-key='2']");
  await page.click("[data-key='0']");
  await page.click("[data-key='00']");
  const shown = await page.locator("[data-role=tendered]").textContent();
  if (!shown.replace(/\s/g, "").startsWith("2000,00")) throw new Error(`tendered shows ${shown}`);
  const change = await page.locator("[data-role=change]").textContent();
  console.log("    tendered:", shown.trim(), "| change:", change.trim());
});
await step("credit without customer blocks", async () => {
  await page.click("[data-mode=credit]");
  const dis = await page.locator("[data-act=confirm]").isDisabled();
  if (!dis) throw new Error("confirm not disabled for credit w/o customer");
  await page.click("[data-mode=cash]");
});
await step("confirm → ticket", async () => {
  await page.click("[data-act=confirm]");
  await page.waitForFunction(() => document.querySelector("#screen").dataset.screen === "ticket");
  await page.waitForSelector("article.doc-80");
  const txt = await page.locator("article.doc-80").textContent();
  console.log("    ticket has number:", /TK-2026-\d{4}/.test(txt), "| thank_you:", txt.includes("Merci"));
});
await step("print toast", async () => {
  await page.click("[data-act=print]");
  await page.waitForSelector(".toast");
});
await step("arabic flips dir", async () => {
  await page.click("[data-lang=ar]");
  const dir = await page.getAttribute("html", "dir");
  if (dir !== "rtl") throw new Error(`dir=${dir}`);
  await page.click("[data-lang=fr]");
});
await step("new sale → till, stock decremented", async () => {
  await page.click("[data-go=till]");
  await page.waitForFunction(() => document.querySelector("#screen").dataset.screen === "till");
  const row = await page.locator("[data-add='7'] .tiny").textContent();
  console.log("    Eau Ifri row:", row.trim(), "(was 120)");
});
await step("customers sheet", async () => {
  await page.click("[data-go=customers]");
  await page.click("[data-sheet=customer][data-id='1']");
  await page.waitForSelector("[data-role=sheet] table");
  await page.click("[data-role=sheet] [data-act=close-sheet]");
  if (await page.locator("[data-role=sheet]").count()) throw new Error("sheet did not close");
});

await browser.close();
console.log(errors.length ? `CONSOLE ERRORS:\n${errors.join("\n")}` : "console errors: 0");
process.exit(errors.length ? 1 : 0);
