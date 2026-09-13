// Contract: the shop's own block, the régime it prints under, and the copies
// of its file. A day is `YYYY-MM-DD` and nothing else, because the régime is
// dated and a statement asks the server what today is; a byte count and a
// restore's counts are exact integers.

import { describe, expect, test } from "vitest";

import type { BackupDto } from "../generated/BackupDto";
import type { BackupsDto } from "../generated/BackupsDto";
import type { RestoreDto } from "../generated/RestoreDto";
import type { SettingsDto } from "../generated/SettingsDto";
import type { StoreDto } from "../generated/StoreDto";
import {
  backupSchema,
  backupsSchema,
  clockSchema,
  datedRegimeSchema,
  healthSchema,
  regimeSchema,
  restoreSchema,
  settingsSchema,
  storeSchema,
} from "./settings";

const store: StoreDto = {
  name: "Alimentation El Baraka",
  rc: "16/00-1234567B25",
  nif: "000216001234567",
  nis: null,
  ai: null,
  address: "12 rue Didouche Mourad, Alger",
  phone: "0550112233",
};

const settings: SettingsDto = {
  store,
  regime: { regime: "reel", valid_from: "2026-01-01" },
  regime_planned: null,
  theme: null,
  discount_threshold_bps: 0,
};

const backup: BackupDto = {
  name: "dzpos-20260908-093000.sqlite",
  taken_at: "2026-09-08T09:30:00",
  bytes: 143_360,
};

const backups: BackupsDto = { backups: [backup], safety_copies: [], upgrade_copies: [] };

const restore: RestoreDto = {
  restored_from: backup.name,
  safety_copy: "dzpos-safety-20260910-064500.sqlite",
  products: 42,
  documents: 17,
};

describe("healthSchema", () => {
  test("takes the answer the unauthenticated route gives", () => {
    expect(
      healthSchema.parse({ status: "ok", shop_id: 1, needs_first_setup: true }),
    ).toEqual({ status: "ok", shop_id: 1, needs_first_setup: true });
  });

  test("refuses an answer with no shop", () => {
    expect(healthSchema.safeParse({ status: "ok" }).success).toBe(false);
  });
});

describe("clockSchema", () => {
  test("takes the day the shop is on", () => {
    expect(clockSchema.parse({ today: "2026-09-10" })).toEqual({ today: "2026-09-10" });
  });

  test("refuses a day it cannot read as a calendar day", () => {
    expect(clockSchema.safeParse({ today: "10/09/2026" }).success).toBe(false);
  });
});

describe("regimeSchema", () => {
  test("takes one of the two the migration's CHECK allows", () => {
    expect(regimeSchema.parse("ifu")).toBe("ifu");
  });

  test("refuses a régime that is not one of them", () => {
    expect(regimeSchema.safeParse("forfait").success).toBe(false);
  });

  test("carries the two the migration's CHECK allows and no third", () => {
    expect(regimeSchema.options).toEqual(["ifu", "reel"]);
  });
});

describe("datedRegimeSchema", () => {
  test("takes a régime and the day it took effect", () => {
    expect(datedRegimeSchema.parse(settings.regime)).toEqual(settings.regime);
  });

  test("refuses a change dated with a time of day", () => {
    const withTime = { regime: "reel", valid_from: "2026-01-01T00:00:00" };
    expect(datedRegimeSchema.safeParse(withTime).success).toBe(false);
  });
});

describe("storeSchema", () => {
  test("takes the seller block with the identifiers it has and nulls elsewhere", () => {
    expect(storeSchema.parse(store)).toEqual(store);
  });

  test("refuses a block whose identifier was left out rather than nulled", () => {
    const { nis: _nis, ...withoutNis } = store;
    expect(storeSchema.safeParse(withoutNis).success).toBe(false);
  });
});

describe("settingsSchema", () => {
  test("takes the page with no change dated ahead", () => {
    expect(settingsSchema.parse(settings)).toEqual(settings);
  });

  test("refuses a planned change that is neither a régime nor null", () => {
    expect(settingsSchema.safeParse({ ...settings, regime_planned: "reel" }).success).toBe(false);
  });
});

describe("backupSchema", () => {
  test("takes one copy of the shop file", () => {
    expect(backupSchema.parse(backup)).toEqual(backup);
  });

  test("refuses a fractional byte count", () => {
    expect(backupSchema.safeParse({ ...backup, bytes: 143_360.5 }).success).toBe(false);
  });
});

describe("backupsSchema", () => {
  test("takes the two lists apart, the safety one empty", () => {
    expect(backupsSchema.parse(backups)).toEqual(backups);
  });

  test("refuses an answer carrying only the daily list", () => {
    expect(backupsSchema.safeParse({ backups: [backup] }).success).toBe(false);
  });
});

describe("restoreSchema", () => {
  test("takes the counts read out of the file that landed", () => {
    expect(restoreSchema.parse(restore)).toEqual(restore);
  });

  test("takes a copy from before the documents table existed", () => {
    expect(restoreSchema.parse({ ...restore, documents: null })).toMatchObject({ documents: null });
  });

  test("refuses a count JSON.parse had to round", () => {
    expect(restoreSchema.safeParse({ ...restore, products: 42.5 }).success).toBe(false);
  });
});
