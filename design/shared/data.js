// Invented shop. Every identifier is fake by construction.
export const STORE = {
  name: "Supérette El Baraka",
  address: "12 rue des Frères Boudjemaa, Constantine",
  phone: "031 00 00 00",
  email: "contact@elbaraka.example",
  rc: "25/00-0000000 A 00",
  nif: "000000000000000",
  nis: "000000000000000",
  ai: "25010000000",
  defaultTvaBps: 1900,
  regime: "reel",
  stampEnabled: true,
  network: "lan",
  server: "CAISSE-1",
  lanHost: "192.168.1.20",
  lanPort: 8080,
};

const c = (dinars) => Math.round(dinars * 100);

export const CATEGORIES = [
  { id: "epicerie", fr: "Épicerie", ar: "بقالة", en: "Grocery" },
  { id: "boissons", fr: "Boissons", ar: "مشروبات", en: "Drinks" },
  { id: "laitier", fr: "Laitier", ar: "ألبان", en: "Dairy" },
  { id: "hygiene", fr: "Hygiène", ar: "نظافة", en: "Hygiene" },
  { id: "menage", fr: "Ménage", ar: "منزل", en: "Household" },
];

export const PRODUCTS = [
  { id: 1, name: "Huile Elio 5L", ar: "زيت إليو 5ل", barcode: "6130001000018", cat: "epicerie", cost: c(820), price: c(920), tvaBps: 1900, qty: 24, min: 10 },
  { id: 2, name: "Sucre Cristal 1kg", ar: "سكر 1كغ", barcode: "6130001000025", cat: "epicerie", cost: c(95), price: c(110), tvaBps: 900, qty: 60, min: 20 },
  { id: 3, name: "Farine Sim 1kg", ar: "فرينة سيم 1كغ", barcode: "6130001000032", cat: "epicerie", cost: c(58), price: c(70), tvaBps: 900, qty: 8, min: 15 },
  { id: 4, name: "Couscous Moyen 1kg", ar: "كسكس متوسط 1كغ", barcode: "6130001000049", cat: "epicerie", cost: c(120), price: c(140), tvaBps: 900, qty: 30, min: 10 },
  { id: 5, name: "Pâtes Spaghetti 500g", ar: "معكرونة 500غ", barcode: "6130001000056", cat: "epicerie", cost: c(62), price: c(75), tvaBps: 900, qty: 45, min: 15 },
  { id: 6, name: "Tomate concentrée 400g", ar: "طماطم مركزة 400غ", barcode: "6130001000063", cat: "epicerie", cost: c(115), price: c(135), tvaBps: 1900, qty: 0, min: 12 },
  { id: 7, name: "Eau Ifri 1.5L", ar: "ماء إفري 1.5ل", barcode: "6130002000017", cat: "boissons", cost: c(30), price: c(40), tvaBps: 1900, qty: 120, min: 24 },
  { id: 8, name: "Hamoud Boualem 1L", ar: "حمود بوعلام 1ل", barcode: "6130002000024", cat: "boissons", cost: c(80), price: c(100), tvaBps: 1900, qty: 36, min: 12 },
  { id: 9, name: "Jus Ramy Orange 1L", ar: "عصير رامي برتقال 1ل", barcode: "6130002000031", cat: "boissons", cost: c(120), price: c(150), tvaBps: 1900, qty: 18, min: 12 },
  { id: 10, name: "Lait Candia 1L", ar: "حليب كانديا 1ل", barcode: "6130003000016", cat: "laitier", cost: c(115), price: c(130), tvaBps: 900, qty: 40, min: 20 },
  { id: 11, name: "Yaourt Soummam x4", ar: "ياغورت صومام ×4", barcode: "6130003000023", cat: "laitier", cost: c(140), price: c(170), tvaBps: 900, qty: 22, min: 10 },
  { id: 12, name: "Fromage La Vache 8p", ar: "جبن البقرة 8ق", barcode: "6130003000030", cat: "laitier", cost: c(250), price: c(290), tvaBps: 1900, qty: 14, min: 8 },
  { id: 13, name: "Savon Le Chat 4x", ar: "صابون لوشا ×4", barcode: "6130004000015", cat: "hygiene", cost: c(280), price: c(340), tvaBps: 1900, qty: 9, min: 10 },
  { id: 14, name: "Dentifrice Signal", ar: "معجون أسنان سيغنال", barcode: "6130004000022", cat: "hygiene", cost: c(190), price: c(240), tvaBps: 1900, qty: 25, min: 8 },
  { id: 15, name: "Isis Détergent 3kg", ar: "إيزيس منظف 3كغ", barcode: "6130005000014", cat: "menage", cost: c(690), price: c(790), tvaBps: 1900, qty: 11, min: 6 },
  { id: 16, name: "Eau de Javel 1L", ar: "ماء جافيل 1ل", barcode: "6130005000021", cat: "menage", cost: c(55), price: c(70), tvaBps: 1900, qty: 33, min: 10 },
];

export const CUSTOMERS = [
  { id: 1, name: "Restaurant El Kahina", ar: "مطعم الكاهنة", phone: "0550 00 00 01", rc: "25/00-0000001 B 25", nif: "000000000000001", nis: "000000000000001", ai: "25010000001", address: "Cité Daksi, Constantine", limit: c(80000), warn: c(60000), debt: c(52400) },
  { id: 2, name: "Boulangerie Nour", ar: "مخبزة نور", phone: "0550 00 00 02", rc: "25/00-0000002 B 25", nif: "000000000000002", nis: "000000000000002", ai: "25010000002", address: "Rue Larbi Ben M'hidi", limit: c(50000), warn: c(40000), debt: c(48100) },
  { id: 3, name: "Amine Benali", ar: "أمين بن علي", phone: "0660 00 00 03", rc: "", nif: "", nis: "", ai: "", address: "", limit: c(15000), warn: c(10000), debt: c(3200) },
  { id: 4, name: "Café des Arcades", ar: "مقهى الأروقة", phone: "0770 00 00 04", rc: "25/00-0000004 B 25", nif: "000000000000004", nis: "000000000000004", ai: "25010000004", address: "Place des Martyrs", limit: c(30000), warn: c(25000), debt: c(0) },
  { id: 5, name: "Samia Kaci", ar: "سامية قاسي", phone: "0550 00 00 05", rc: "", nif: "", nis: "", ai: "", address: "", limit: c(10000), warn: c(8000), debt: c(9450) },
];

export const LEDGER = {
  1: [
    { date: "2026-09-01", doc: "FA-2026-0142", debit: c(31200), credit: 0 },
    { date: "2026-09-03", doc: "PAY-0089", debit: 0, credit: c(20000) },
    { date: "2026-09-05", doc: "FA-2026-0151", debit: c(41200), credit: 0 },
  ],
  2: [
    { date: "2026-08-28", doc: "FA-2026-0130", debit: c(28100), credit: 0 },
    { date: "2026-09-04", doc: "FA-2026-0148", debit: c(20000), credit: 0 },
  ],
  3: [{ date: "2026-09-06", doc: "TK-2026-0912", debit: c(3200), credit: 0 }],
  4: [],
  5: [{ date: "2026-09-02", doc: "TK-2026-0877", debit: c(9450), credit: 0 }],
};

export const USERS = [
  { name: "Karim", role: "role_owner" },
  { name: "Nadia", role: "role_manager" },
  { name: "Yacine", role: "role_cashier" },
];

export const DASH = {
  salesToday: c(64350),
  marginToday: c(9870),
  expensesMonth: c(84000),
  cashPosition: c(212400),
  salesMonth: c(1184300),
  sparkline: [42, 55, 38, 61, 70, 48, 64],
  topProducts: [7, 2, 10, 8, 1],
};

export function nextNumber(kind) {
  const n = { ticket: 913, facture: 152 };
  return `${kind === "facture" ? "FA" : "TK"}-2026-${String(n[kind] + 1).padStart(4, "0")}`;
}
