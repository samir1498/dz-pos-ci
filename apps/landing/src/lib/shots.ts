// Turns the desktop e2e's committed screenshots
// (apps/desktop/e2e/screenshots/*.png) into the product shots this page
// shows. Every pixel a visitor sees is one of those PNGs, cropped and
// framed; nothing here draws UI (context/plans/20260911-landing-page.md, L1).
//
// Pure planning and composition. `src/shots.test.ts` is the only thing that
// touches disk: it writes when DZPOS_SHOTS_WRITE=1 (`pnpm run shots`, the
// same shape as @dzpos/design's `gen:theme`), and otherwise asserts the
// committed files under public/shots/ and src/lib/shots.json still match
// what this module produces, so a changed screenshot that nobody reran the
// script for fails the gates instead of shipping stale art.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import sharp, { type Sharp } from "sharp";

import { theme } from "@dzpos/design";

const SCREENSHOTS_DIR = fileURLToPath(
  new URL("../../../desktop/e2e/screenshots/", import.meta.url),
);

const screenshotPath = (name: string): string => `${SCREENSHOTS_DIR}${name}.png`;

/** Reads a committed screenshot's own bytes, so a test comparing hashes has one place to look. */
export const readScreenshot = (name: string): Buffer => readFileSync(screenshotPath(name));

/**
 * The frame: `design/index.html`'s `.frame` class, unchanged, wrapped around
 * a real screenshot instead of standing in for one. That rule is
 *   border-radius: var(--radius-md); background: var(--surface-raised);
 *   border: 1px solid var(--border-default);
 * No titlebar, no traffic lights: neither `design/` mockup draws one, and
 * the brief says not to invent a frame the mockups do not already show. The
 * screenshot itself already carries the app's own top bar.
 *
 * The soft blur (`--shadow-lg`) is left for the page's CSS: rasterizing a
 * gaussian shadow bakes in one background colour forever, and the collage
 * sits on the dark band while the single shots sit on the light one. Only
 * the crisp ring below is baked in, the way `design/mobile`'s `.phone`
 * bezel is a flat colour, not a shadow.
 */
const FRAME = {
  ringColor: theme.colors.surface.raised,
  borderColor: theme.colors.border.default,
  radiusPx: theme.radius.md,
  ringPx: theme.space[4],
} as const;

/** A crop of one committed screenshot, in the screenshot's own pixels. Always the full 1280 width. */
export interface Crop {
  readonly source: string;
  readonly top: number;
  readonly height: number;
}

const roundedRectFill = (width: number, height: number, radius: number, fill: string): Buffer =>
  Buffer.from(
    `<svg width="${width}" height="${height}" xmlns="http://www.w3.org/2000/svg">` +
      `<rect x="0" y="0" width="${width}" height="${height}" rx="${radius}" ry="${radius}" fill="${fill}"/>` +
      `</svg>`,
  );

const roundedRectStroke = (width: number, height: number, radius: number, stroke: string): Buffer =>
  Buffer.from(
    `<svg width="${width}" height="${height}" xmlns="http://www.w3.org/2000/svg">` +
      `<rect x="0.5" y="0.5" width="${width - 1}" height="${height - 1}" ` +
      `rx="${radius}" ry="${radius}" fill="none" stroke="${stroke}" stroke-width="1"/>` +
      `</svg>`,
  );

/**
 * One screenshot, cropped and scaled to `width`, set into the `.frame` ring
 * at `scale` (1 for the @1x output, 2 for @2x) so the ring and the radius
 * stay the same physical size at either density. The screenshot itself
 * keeps square corners: at `ring > radius` the bezel's curvature never
 * reaches it, the same way a photograph sits flat inside a rounded frame.
 */
const frameTile = async (
  crop: Crop,
  width: number,
  scale: 1 | 2,
): Promise<{ image: Sharp; width: number; height: number }> => {
  const shot = await sharp(screenshotPath(crop.source))
    .extract({ left: 0, top: crop.top, width: 1280, height: crop.height })
    .resize({ width })
    .png()
    .toBuffer();
  const shotMeta = await sharp(shot).metadata();
  if (shotMeta.width === undefined || shotMeta.height === undefined) {
    throw new Error(`${crop.source}: sharp returned no pixel dimensions after resize`);
  }
  const { width: shotWidth, height: shotHeight } = shotMeta;

  const ring = FRAME.ringPx * scale;
  const radius = FRAME.radiusPx * scale;
  const outerWidth = shotWidth + ring * 2;
  const outerHeight = shotHeight + ring * 2;

  const image = sharp(roundedRectFill(outerWidth, outerHeight, radius, FRAME.ringColor)).composite([
    { input: shot, left: ring, top: ring },
    { input: roundedRectStroke(outerWidth, outerHeight, radius, FRAME.borderColor), left: 0, top: 0 },
  ]);

  return { image, width: outerWidth, height: outerHeight };
};

/** A single framed screenshot: the hero and the four named pieces. */
export interface SingleSpec {
  readonly key: string;
  readonly crop: Crop;
  /** CSS width the page displays this at; the @2x output doubles it. */
  readonly cssWidth: number;
}

// The till, cropped above the (empty, in this fixture) ticket-details box so
// the cart, the totals and the payment keypad all stay in frame.
export const HERO: SingleSpec = {
  key: "hero",
  crop: { source: "till-ar", top: 0, height: 1450 },
  cssWidth: 640,
};

// A different till moment from the hero: a credit sale past the customer's
// limit, warned and then overridden. No crop: the whole screen is the point.
export const SELLING: SingleSpec = {
  key: "selling",
  crop: { source: "till-credit-ar", top: 0, height: 1060 },
  cssWidth: 480,
};

// The documents list, cropped above the opened facture panel: one screen,
// every paper kind the shop issues (facture, avoir, ticket).
export const INVOICING: SingleSpec = {
  key: "invoicing",
  crop: { source: "documents-avoir-ar", top: 0, height: 950 },
  cssWidth: 480,
};

// The product catalogue, not the recount panel buried inside Settings: the
// one screen that answers "how much do I have and what's running low"
// without a detour through a settings page a shopkeeper never opens for
// that reason. See the report for the alternative this rejects.
export const STOCK: SingleSpec = {
  key: "stock",
  crop: { source: "products", top: 0, height: 800 },
  cssWidth: 480,
};
export const STOCK_AR_CROP: Crop = { source: "products-ar", top: 0, height: 800 };

// The dashboard's 30-day chart in full, cropped above the low-stock and
// best-seller tables so the trend line is never cut mid-way.
export const KNOWING: SingleSpec = {
  key: "knowing",
  crop: { source: "dashboard", top: 0, height: 950 },
  cssWidth: 480,
};
export const KNOWING_AR_CROP: Crop = { source: "dashboard-ar", top: 0, height: 950 };

export const SINGLE_SHOTS: readonly SingleSpec[] = [HERO, SELLING, INVOICING, STOCK, KNOWING];

/** Several framed screenshots composed side by side, for the band-of-screens section. */
export interface CollageSpec {
  readonly key: string;
  readonly crops: readonly Crop[];
  readonly cssWidth: number;
  readonly gapPx: number;
}

/**
 * The band of screens: three real screens side by side, the same three
 * screens in French and Arabic so the two collages are the same shape and
 * only the pixels inside differ. en reuses the French one, the way
 * `products.spec.ts` says English adds no committed shot of its own.
 */
export const COLLAGE: CollageSpec = {
  key: "collage",
  crops: [
    { source: "dashboard", top: 0, height: 800 },
    { source: "products", top: 0, height: 800 },
    { source: "customers", top: 0, height: 800 },
  ],
  cssWidth: 1120,
  gapPx: 24,
};

export const COLLAGE_AR: CollageSpec = {
  ...COLLAGE,
  crops: [
    { source: "dashboard-ar", top: 0, height: 800 },
    { source: "products-ar", top: 0, height: 800 },
    { source: "customers-ar", top: 0, height: 800 },
  ],
};

/** One rendered file: its name and pixel size, ready to write and to record in the manifest. */
export interface RenderedShot {
  readonly file: string;
  readonly width: number;
  readonly height: number;
  readonly bytes: Buffer;
}

const toWebp = async (image: Sharp, file: string): Promise<RenderedShot> => {
  const meta = await image.metadata();
  if (meta.width === undefined || meta.height === undefined) {
    throw new Error(`${file}: sharp returned no pixel dimensions`);
  }
  const bytes = await image.webp({ lossless: true }).toBuffer();
  return { file, width: meta.width, height: meta.height, bytes };
};

/**
 * `fileKey` defaults to the spec's own key; the Arabic crop of "stock" and
 * "knowing" passes a distinct one so its file does not collide with the
 * French render of the same spec.
 */
const renderSingle = async (
  spec: SingleSpec,
  crop: Crop,
  scale: 1 | 2,
  fileKey: string = spec.key,
): Promise<RenderedShot> => {
  const { image } = await frameTile(crop, spec.cssWidth * scale, scale);
  return toWebp(image, `${fileKey}-${scale}x.webp`);
};

const renderCollage = async (
  spec: CollageSpec,
  variant: "fr" | "ar",
  scale: 1 | 2,
): Promise<RenderedShot> => {
  const gap = spec.gapPx * scale;
  const targetWidth = spec.cssWidth * scale;
  const tileWidth = Math.round((targetWidth - gap * (spec.crops.length - 1)) / spec.crops.length);

  const tiles = await Promise.all(spec.crops.map((crop) => frameTile(crop, tileWidth, scale)));
  const canvasHeight = Math.max(...tiles.map((t) => t.height));
  let x = 0;
  const placed: { input: Buffer; left: number; top: number }[] = [];
  for (const tile of tiles) {
    placed.push({ input: await tile.image.png().toBuffer(), left: x, top: 0 });
    x += tile.width + gap;
  }
  const canvasWidth = x - gap;

  const composed = sharp({
    create: { width: canvasWidth, height: canvasHeight, channels: 4, background: { r: 0, g: 0, b: 0, alpha: 0 } },
  }).composite(placed);

  return toWebp(composed, `collage-${variant}-${scale}x.webp`);
};

export interface ShotEntry {
  readonly src1x: string;
  readonly src2x: string;
  readonly width: number;
  readonly height: number;
}

export type Locale = "fr" | "en" | "ar";

export interface ShotsManifest {
  readonly hero: Readonly<Record<Locale, ShotEntry>>;
  readonly selling: Readonly<Record<Locale, ShotEntry>>;
  readonly invoicing: Readonly<Record<Locale, ShotEntry>>;
  readonly stock: Readonly<Record<Locale, ShotEntry>>;
  readonly knowing: Readonly<Record<Locale, ShotEntry>>;
  readonly collage: Readonly<Record<Locale, ShotEntry>>;
}

/** Every file this module writes, and the manifest describing them. */
export interface ShotsBuild {
  readonly files: readonly RenderedShot[];
  readonly manifest: ShotsManifest;
}

export const buildShots = async (): Promise<ShotsBuild> => {
  const files: RenderedShot[] = [];

  const single = async (spec: SingleSpec, crop: Crop, fileKey?: string): Promise<ShotEntry> => {
    const r1 = await renderSingle(spec, crop, 1, fileKey);
    const r2 = await renderSingle(spec, crop, 2, fileKey);
    files.push(r1, r2);
    return { src1x: r1.file, src2x: r2.file, width: r1.width, height: r1.height };
  };

  // No French (or English) screenshot exists for the till or the documents
  // list, so all three locales share the Arabic crop for those two and for
  // the hero (all three built from HERO/SELLING/INVOICING's own crop, which
  // is already the Arabic source). Only "knowing" and "stock" have a real
  // French screen to show a French or English visitor instead, so those two
  // get a second, Arabic-suffixed file alongside the French one.
  const hero = await single(HERO, HERO.crop);
  const selling = await single(SELLING, SELLING.crop);
  const invoicing = await single(INVOICING, INVOICING.crop);
  const knowingFr = await single(KNOWING, KNOWING.crop);
  const knowingAr = await single(KNOWING, KNOWING_AR_CROP, "knowing-ar");
  const stockFr = await single(STOCK, STOCK.crop);
  const stockAr = await single(STOCK, STOCK_AR_CROP, "stock-ar");

  const collage = async (spec: CollageSpec, variant: "fr" | "ar"): Promise<ShotEntry> => {
    const r1 = await renderCollage(spec, variant, 1);
    const r2 = await renderCollage(spec, variant, 2);
    files.push(r1, r2);
    return { src1x: r1.file, src2x: r2.file, width: r1.width, height: r1.height };
  };
  const collageFr = await collage(COLLAGE, "fr");
  const collageAr = await collage(COLLAGE_AR, "ar");

  const manifest: ShotsManifest = {
    hero: { fr: hero, en: hero, ar: hero },
    selling: { fr: selling, en: selling, ar: selling },
    invoicing: { fr: invoicing, en: invoicing, ar: invoicing },
    stock: { fr: stockFr, en: stockFr, ar: stockAr },
    knowing: { fr: knowingFr, en: knowingFr, ar: knowingAr },
    collage: { fr: collageFr, en: collageFr, ar: collageAr },
  };

  // Stable order: whoever reruns this on another machine gets the files
  // listed the same way, not in a directory-listing order.
  files.sort((a, b) => a.file.localeCompare(b.file));

  return { files, manifest };
};
