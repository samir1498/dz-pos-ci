import "@testing-library/jest-dom/vitest";

// jsdom has no scrolling and says so on every call. The router scrolls to
// the top when it mounts a route, so a test that renders a screen inside one
// would otherwise bury its own output under the same warning.
window.scrollTo = () => {};

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
