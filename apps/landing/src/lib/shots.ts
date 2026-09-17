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

import { fileURLToPath } from "node:url";

import sharp, { type Sharp } from "sharp";

import { theme } from "@dzpos/design";

const SCREENSHOTS_DIR = fileURLToPath(
  new URL("../../../desktop/e2e/screenshots/", import.meta.url),
);

const screenshotPath = (name: string): string => `${SCREENSHOTS_DIR}${name}.png`;

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

/**
 * Every generated file is rendered at twice its CSS width, so the ring and
 * border stay crisp on a 2x screen at whichever breakpoint that file is
 * for. A 3x phone falls back to the next candidate up (still sharper than
 * the flat design's old desktop-sized 2x file, just not pixel-perfect);
 * chasing 3x everywhere would triple the file count for a gain nobody
 * measurably sees at these shot sizes.
 */
export const RETINA_FACTOR = 2;

/**
 * Every screenshot in apps/desktop/e2e/screenshots/ is captured 1280 device
 * px wide. A tile whose resize target lands within a device pixel or two of
 * that is not a real rescale -- sharp's resize is close enough to a no-op
 * that the pixels stay flat and lossless webp compresses them tight. A tile
 * resized further off, up or down, resamples the screenshot and introduces
 * blend noise that lossless stores pixel for pixel, which costs more than a
 * heavier, sharper photograph would (measured: the hero's 620-wide downscale
 * was 58490 B lossless, more than the 672-tier's untouched 1280-wide
 * 42592 B). Those tiles get lossy webp instead; see toWebp.
 */
const SOURCE_WIDTH = 1280;
const isNearOriginalScale = (targetWidth: number): boolean => Math.abs(targetWidth - SOURCE_WIDTH) <= 2;

/**
 * webp quality for a tile that isn't near-original-scale (see
 * isNearOriginalScale). Chosen by rendering the hero's most-resampled
 * tiers at the time -- 342 outer (a 2.06x downscale) and a 720-outer tier
 * that was since deleted (a 1.08x *upscale* of the 1280px source, so it
 * carried no pixel the 1344w tier didn't already have; see HERO_WIDTHS) --
 * at quality 70/78/82/88/92/96, comparing lossless vs each quality at 2x-4x
 * nearest-neighbour zoom on the ticket panel's smallest text (the ticket ID
 * and the price figures), the densest, smallest text either tile carries.
 * 82 was visually indistinguishable from lossless at that zoom on both
 * tiers, with real margin below it (70 was also clean); 82 is kept rather
 * than pushed lower so a future, more detailed screenshot has headroom
 * before this needs re-checking by eye.
 */
const LOSSY_QUALITY = 82;

/**
 * HERO_WIDTHS, PIECE_WIDTHS and SCREENS_WIDTHS below are the shot's own
 * *outer*, ring-included CSS width -- what a headless browser measures on
 * the rendered `<img>`, since the ring is baked into the bitmap rather than
 * drawn by CSS. frameTile resizes to a *content* width and adds the ring
 * on top, so every caller has to strip the ring back out first or the ring
 * gets counted twice and every file comes out about 2*FRAME.ringPx CSS px
 * (2x that many device px) wider than the breakpoint it is for.
 */
const contentWidth = (outerCssWidth: number): number => outerCssWidth - 2 * FRAME.ringPx;

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
 * One screenshot, cropped and scaled to `cssWidth * RETINA_FACTOR` device
 * pixels, set into the `.frame` ring at that same factor so the ring and
 * the radius stay the same physical size relative to the CSS box. The
 * screenshot itself keeps square corners: at `ring > radius` the bezel's
 * curvature never reaches it, the same way a photograph sits flat inside a
 * rounded frame.
 */
const frameTile = async (
  crop: Crop,
  cssWidth: number,
): Promise<{ image: Sharp; width: number; height: number; lossless: boolean }> => {
  const width = cssWidth * RETINA_FACTOR;
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

  const ring = FRAME.ringPx * RETINA_FACTOR;
  const radius = FRAME.radiusPx * RETINA_FACTOR;
  const outerWidth = shotWidth + ring * 2;
  const outerHeight = shotHeight + ring * 2;

  const image = sharp(roundedRectFill(outerWidth, outerHeight, radius, FRAME.ringColor)).composite([
    { input: shot, left: ring, top: ring },
    { input: roundedRectStroke(outerWidth, outerHeight, radius, FRAME.borderColor), left: 0, top: 0 },
  ]);

  return { image, width: outerWidth, height: outerHeight, lossless: isNearOriginalScale(width) };
};

/**
 * Hero.astro's .hero-shot: below .wrap's 1120px content cap (base.css),
 * hero-copy and hero-shot stack into a single column and the shot fills
 * the full content width; at and above 1120px hero-copy claims its own
 * lane and the shot stops growing at a fixed 672 CSS px. Measured with a
 * headless browser at 390/768/1120/1440px viewports, in fr, en and ar
 * (identical in all three -- the layout has no RTL-specific width).
 */
const HERO_SIZES = "(min-width: 1120px) 672px, calc(100vw - 48px)";

/**
 * The fluid single-column range measured 720 CSS px wide at the 768px
 * tablet breakpoint -- wider than the fixed two-column layout's 672 CSS px
 * desktop tier, because hero-copy's own text claims a lane there and stops
 * the shot from growing past it; see HERO_SIZES. That third, 720-wide tier
 * is dropped: at RETINA_FACTOR 2 it resizes to 1440 device px, past the
 * 1280px source, so it upscales rather than resamples down and carries no
 * pixel the 672 tier's 1344w file doesn't already have. A browser or
 * tablet that would have picked it falls back to the 1344w file and scales
 * it in CSS, the same source-to-screen mapping for free.
 */
const HERO_WIDTHS: readonly number[] = [342, 672];

/**
 * Pieces.astro's four shots share one grid: one column at or below the
 * 720px media query (full content width minus the card's own padding and
 * border), two columns above it (half the content width, same minus),
 * capped once the columns stop growing at .wrap's 1120px content width.
 * Measured the same way as HERO_SIZES: 308 CSS px at the 390px phone
 * measurement, 314 CSS px at the 768px tablet measurement, 490 CSS px
 * capped.
 *
 * Only two of those three numbers are kept below. 308 is dropped: it and
 * 314 are 6 CSS px (2.2%) apart, close enough that the 314 tier's own
 * 628-wide file already satisfies a 390px phone's DPR2 target (616) even
 * without a dedicated 308 tier, and close enough in rendered pixels that
 * lossy webp's output size does not reliably stay ordered between the two
 * -- measured: the "knowing" shot's 308 tier came out 82 B *larger* than
 * its 314 tier, which the ascending-bytes test below exists to catch.
 */
const PIECE_SIZES =
  "(min-width: 1120px) 490px, (max-width: 720px) calc(100vw - 82px), calc(50vw - 70px)";
const PIECE_WIDTHS: readonly number[] = [314, 490];

/**
 * Screens.astro's collage: the full content width the same way the hero's
 * fluid range is, with no two-column quirk to complicate it.
 */
const SCREENS_SIZES = "(min-width: 1120px) 1072px, calc(100vw - 48px)";
const SCREENS_WIDTHS: readonly number[] = [342, 720, 1072];

/** A single framed screenshot: the hero and the four named pieces. */
export interface SingleSpec {
  readonly key: string;
  readonly crop: Crop;
  /** CSS widths this shot actually renders at; each file is rendered at
   * RETINA_FACTOR times one of these. */
  readonly widths: readonly number[];
  /** The <img sizes> value a browser needs to pick the right one of those
   * widths before layout runs. */
  readonly sizes: string;
  /**
   * Which entry of `widths` the `width`/`height` attributes are derived
   * from. Defaults to the smallest (index 0), which is safe wherever the
   * component sets an explicit CSS width (Pieces.astro and Screens.astro
   * both set `width: 100%`, so the attribute value only supplies the
   * aspect ratio). Hero.astro does not -- `.hero-shot` is sized purely by
   * `flex: 1 1 320px`, and a flex item's automatic minimum size reads the
   * `<img>`'s own natural size from these attributes. Pointing that at the
   * smallest (phone) tier made the two-column layout's threshold jump from
   * ~1078px to 1120px; pointing it at the desktop tier (672 CSS px, the
   * same number the old fixed-size design always used) reproduces the
   * original breakpoint exactly. Checked by measuring the built page
   * before and after this field existed.
   */
  readonly attrWidthIndex?: number;
}

// The till, cropped above the (empty, in this fixture) ticket-details box so
// the cart, the totals and the payment keypad all stay in frame. The height
// is measured, not chosen: the Encaisser button is the last thing that must
// be whole, and it sits at rows 1439..1490 of the screenshot. When the till
// fixture gains a row the button moves down and a stale height slices it in
// half, which is what happened when the settings work reseeded the
// catalogue and pushed both tills 36 px taller.
export const HERO: SingleSpec = {
  key: "hero",
  crop: { source: "till", top: 0, height: 1486 },
  widths: HERO_WIDTHS,
  sizes: HERO_SIZES,
  // HERO_WIDTHS is [342, 672]: index 1 is the 672 CSS px desktop tier.
  attrWidthIndex: 1,
};
export const HERO_AR_CROP: Crop = { source: "till-ar", top: 0, height: 1486 };

// A different till moment from the hero: a credit sale past the customer's
// limit, warned and then overridden. No crop: the whole screen is the point.
export const SELLING: SingleSpec = {
  key: "selling",
  crop: { source: "till-credit", top: 0, height: 1060 },
  widths: PIECE_WIDTHS,
  sizes: PIECE_SIZES,
};
export const SELLING_AR_CROP: Crop = { source: "till-credit-ar", top: 0, height: 1060 };

// The documents list, cropped above the opened facture panel: one screen,
// every paper kind the shop issues (facture, avoir, ticket).
export const INVOICING: SingleSpec = {
  key: "invoicing",
  crop: { source: "documents-avoir", top: 0, height: 950 },
  widths: PIECE_WIDTHS,
  sizes: PIECE_SIZES,
};
export const INVOICING_AR_CROP: Crop = { source: "documents-avoir-ar", top: 0, height: 950 };

// The product catalogue, not the recount panel buried inside Settings: the
// one screen that answers "how much do I have and what's running low"
// without a detour through a settings page a shopkeeper never opens for
// that reason. See the report for the alternative this rejects.
export const STOCK: SingleSpec = {
  key: "stock",
  crop: { source: "products", top: 0, height: 800 },
  widths: PIECE_WIDTHS,
  sizes: PIECE_SIZES,
};
export const STOCK_AR_CROP: Crop = { source: "products-ar", top: 0, height: 800 };

// The dashboard's 30-day chart in full, cropped above the low-stock and
// best-seller tables so the trend line is never cut mid-way.
export const KNOWING: SingleSpec = {
  key: "knowing",
  crop: { source: "dashboard", top: 0, height: 950 },
  widths: PIECE_WIDTHS,
  sizes: PIECE_SIZES,
};
export const KNOWING_AR_CROP: Crop = { source: "dashboard-ar", top: 0, height: 950 };

/** Several framed screenshots composed side by side, for the band-of-screens section. */
export interface CollageSpec {
  readonly key: string;
  readonly crops: readonly Crop[];
  readonly widths: readonly number[];
  readonly sizes: string;
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
  widths: SCREENS_WIDTHS,
  sizes: SCREENS_SIZES,
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

/**
 * `lossless` selects the encode, not just a flag passed through: a tile
 * near the screenshot's own 1280px scale (see isNearOriginalScale) is flat
 * enough that lossless wins outright, while a genuinely resampled tile
 * carries resize noise lossless would store pixel for pixel, so it gets
 * lossy webp at LOSSY_QUALITY instead. See the doc comment on
 * isNearOriginalScale for the measurement that found this.
 */
const toWebp = async (image: Sharp, file: string, lossless: boolean): Promise<RenderedShot> => {
  const meta = await image.metadata();
  if (meta.width === undefined || meta.height === undefined) {
    throw new Error(`${file}: sharp returned no pixel dimensions`);
  }
  const bytes = await image
    .webp(lossless ? { lossless: true } : { quality: LOSSY_QUALITY })
    .toBuffer();
  return { file, width: meta.width, height: meta.height, bytes };
};

/**
 * `fileKey` defaults to the spec's own key; every spec's Arabic crop passes
 * a distinct one (`<key>-ar`) so its file does not collide with the French
 * render of the same spec. The file name carries the rendered pixel width
 * (the `w` descriptor srcset needs), not a density label: nothing outside
 * this module cares whether a given file happens to be one spec's 2x phone
 * render and another's 1x desktop one.
 */
const renderSingle = async (
  spec: SingleSpec,
  crop: Crop,
  cssWidth: number,
  fileKey: string = spec.key,
): Promise<RenderedShot> => {
  const { image, width, lossless } = await frameTile(crop, contentWidth(cssWidth));
  // The file's actual pixel width, ring included, not cssWidth*RETINA_FACTOR:
  // the ring adds pixels the srcset `w` descriptor has to account for too,
  // or the browser picks against a number the file doesn't actually match.
  return toWebp(image, `${fileKey}-${width}w.webp`, lossless);
};

const renderCollage = async (
  spec: CollageSpec,
  variant: "fr" | "ar",
  cssWidth: number,
): Promise<RenderedShot> => {
  const gap = spec.gapPx * RETINA_FACTOR;
  const n = spec.crops.length;
  // cssWidth is the whole collage's outer, ring-included width; each of the
  // n tiles carries its own ring on top of its own content width, so this
  // strips out n rings' worth (not just one, see contentWidth) alongside
  // the gaps between tiles before dividing.
  const tileWidth1x = Math.round((cssWidth - n * 2 * FRAME.ringPx - spec.gapPx * (n - 1)) / n);

  const tiles = await Promise.all(spec.crops.map((crop) => frameTile(crop, tileWidth1x)));
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

  // Every tile in one call shares tileWidth1x, so they all got the same
  // near-original-scale verdict from frameTile; take it from the first
  // rather than recomputing.
  const firstTile = tiles[0];
  if (firstTile === undefined) {
    throw new Error(`collage-${variant}: no crops to render`);
  }

  // canvasWidth, not cssWidth*RETINA_FACTOR: the three tiles' rings and the
  // rounding in tileWidth1x mean the composed canvas is not exactly that
  // product, and the filename has to match the file's real pixel width.
  return toWebp(composed, `collage-${variant}-${canvasWidth}w.webp`, firstTile.lossless);
};

/** One `srcset` candidate: the file and its actual rendered pixel width. */
export interface ShotSource {
  readonly url: string;
  readonly width: number;
}

export interface ShotEntry {
  /** Every rendered width, ascending, for the `srcset` `w` list. */
  readonly sources: readonly ShotSource[];
  /** The smallest source, for browsers old enough to ignore `srcset`. */
  readonly src: string;
  /**
   * CSS pixels, not the smallest source file's actual (retina) pixel size:
   * every generated file is RETINA_FACTOR times its CSS size, and Hero.astro
   * sits in a flex row where the `width` attribute feeds the browser's
   * automatic min-size for the flex item, not just the aspect ratio. Set
   * these to the file's own device pixels instead and the hero stops
   * wrapping to two columns until a much wider viewport than the CSS
   * actually needs -- verified by measuring the built page with and
   * without this divide, and only this version matches base.css.
   */
  readonly width: number;
  readonly height: number;
  /** The `sizes` attribute this shot's `<img>` must carry, copied from the
   * spec so every consumer reads the same string instead of retyping it. */
  readonly sizes: string;
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

/** `widths` must already be ascending; every caller below passes a spec's
 * own `widths`, which are written that way and checked once here rather
 * than trusted silently at every call site. */
const assertAscending = (widths: readonly number[], label: string): void => {
  for (let i = 1; i < widths.length; i += 1) {
    const previous = widths[i - 1];
    const current = widths[i];
    if (previous === undefined || current === undefined || current <= previous) {
      throw new Error(`${label}: widths must be ascending, got ${JSON.stringify(widths)}`);
    }
  }
};

/** Every generated file's pixel width is exactly RETINA_FACTOR times its CSS
 * width (the ring scales with it too, see frameTile), so this recovers the
 * CSS width from any one of them without threading a second number through
 * renderSingle/renderCollage's return value. Heights are not guaranteed
 * even (a crop's height is whatever the source screenshot gives it), so
 * this rounds rather than assuming an exact divide the way toCssPixels for
 * widths could skip; the `width`/`height` attributes only set the aspect
 * ratio the browser reserves space with, not what CSS displays, so a
 * sub-pixel rounding here is not a visible seam. */
const toCssPixels = (devicePixels: number): number => Math.round(devicePixels / RETINA_FACTOR);

export const buildShots = async (): Promise<ShotsBuild> => {
  const files: RenderedShot[] = [];

  const single = async (spec: SingleSpec, crop: Crop, fileKey?: string): Promise<ShotEntry> => {
    assertAscending(spec.widths, fileKey ?? spec.key);
    const rendered = await Promise.all(spec.widths.map((w) => renderSingle(spec, crop, w, fileKey)));
    files.push(...rendered);
    const smallest = rendered[0];
    if (smallest === undefined) {
      throw new Error(`${fileKey ?? spec.key}: no widths to render`);
    }
    const attrSource = rendered[spec.attrWidthIndex ?? 0];
    if (attrSource === undefined) {
      throw new Error(`${fileKey ?? spec.key}: attrWidthIndex ${String(spec.attrWidthIndex)} is out of range`);
    }
    return {
      sources: rendered.map((r) => ({ url: r.file, width: r.width })),
      src: smallest.file,
      width: toCssPixels(attrSource.width),
      height: toCssPixels(attrSource.height),
      sizes: spec.sizes,
    };
  };

  // Every single shot now has both a French and an Arabic source screenshot,
  // so each renders twice: the French file for fr and en (English keeps no
  // committed screens of its own; French is the nearer language), and a
  // second, Arabic-suffixed file for ar.
  const heroFr = await single(HERO, HERO.crop);
  const heroAr = await single(HERO, HERO_AR_CROP, "hero-ar");
  const sellingFr = await single(SELLING, SELLING.crop);
  const sellingAr = await single(SELLING, SELLING_AR_CROP, "selling-ar");
  const invoicingFr = await single(INVOICING, INVOICING.crop);
  const invoicingAr = await single(INVOICING, INVOICING_AR_CROP, "invoicing-ar");
  const knowingFr = await single(KNOWING, KNOWING.crop);
  const knowingAr = await single(KNOWING, KNOWING_AR_CROP, "knowing-ar");
  const stockFr = await single(STOCK, STOCK.crop);
  const stockAr = await single(STOCK, STOCK_AR_CROP, "stock-ar");

  const collage = async (spec: CollageSpec, variant: "fr" | "ar"): Promise<ShotEntry> => {
    assertAscending(spec.widths, `collage-${variant}`);
    const rendered = await Promise.all(spec.widths.map((w) => renderCollage(spec, variant, w)));
    files.push(...rendered);
    const smallest = rendered[0];
    if (smallest === undefined) {
      throw new Error(`collage-${variant}: no widths to render`);
    }
    return {
      sources: rendered.map((r) => ({ url: r.file, width: r.width })),
      src: smallest.file,
      width: toCssPixels(smallest.width),
      height: toCssPixels(smallest.height),
      sizes: spec.sizes,
    };
  };
  const collageFr = await collage(COLLAGE, "fr");
  const collageAr = await collage(COLLAGE_AR, "ar");

  const manifest: ShotsManifest = {
    hero: { fr: heroFr, en: heroFr, ar: heroAr },
    selling: { fr: sellingFr, en: sellingFr, ar: sellingAr },
    invoicing: { fr: invoicingFr, en: invoicingFr, ar: invoicingAr },
    stock: { fr: stockFr, en: stockFr, ar: stockAr },
    knowing: { fr: knowingFr, en: knowingFr, ar: knowingAr },
    collage: { fr: collageFr, en: collageFr, ar: collageAr },
  };

  // Stable order: whoever reruns this on another machine gets the files
  // listed the same way, not in a directory-listing order.
  files.sort((a, b) => a.file.localeCompare(b.file));

  return { files, manifest };
};
