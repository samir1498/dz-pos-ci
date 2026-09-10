import "@testing-library/jest-dom/vitest";

// jsdom has no scrolling and says so on every call. The router scrolls to
// the top when it mounts a route, so a test that renders a screen inside one
// would otherwise bury its own output under the same warning.
window.scrollTo = () => {};
