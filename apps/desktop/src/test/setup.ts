import "@testing-library/jest-dom/vitest";

// jsdom has no scrolling and says so on every call. The router scrolls to
// the top when it mounts a route, so a test that renders a screen inside one
// would otherwise bury its own output under the same warning.
window.scrollTo = () => {};

// jsdom implements neither pointer capture nor scrolling, and Radix's Select
// and DropdownMenu call all four of these on the way to opening. Without them
// a click on a select trigger throws "hasPointerCapture is not a function"
// and no screen carrying a select can be tested at all.
//
// These are stubs and not implementations, so what they buy is the ability to
// open a popover and read what is in it, never a claim about where it was
// painted. A layout question about an overlay is an e2e's (kit.spec.ts), the
// same way the kit's own tests say.
Element.prototype.hasPointerCapture = () => false;
Element.prototype.setPointerCapture = () => {};
Element.prototype.releasePointerCapture = () => {};
Element.prototype.scrollIntoView = () => {};

// Radix measures its trigger with a ResizeObserver before it places a
// popover, and jsdom has none. A stub that observes nothing is enough: there
// is no layout to react to here, only the question of what the popover
// contains.
class NoResizeObserver {
  observe(): void {}
  unobserve(): void {}
  disconnect(): void {}
}
window.ResizeObserver = NoResizeObserver;

// jsdom implements no matchMedia. The theme provider guards for its absence
// (a webview with the API off falls to the light theme), but a test that wants
// to drive the machine preference needs one to spy on, so a stub that answers
// "not dark" stands in and a test overrides it.
window.matchMedia = (query: string): MediaQueryList => ({
  matches: false,
  media: query,
  onchange: null,
  addEventListener: () => {},
  removeEventListener: () => {},
  addListener: () => {},
  removeListener: () => {},
  dispatchEvent: () => false,
});
