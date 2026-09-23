// `screens/Index.tsx` picks one of three redirects, or nothing while
// `ready` is still false, purely from `useSession()`'s state and the
// `home` prop it is given (C7 review fix #4: nothing before this file
// called the component at all, so a redirect target that stopped reading
// `home` — or a route wrapper passing the wrong literal — would have
// shipped unnoticed).
//
// It is a plain function component with no state of its own, so calling
// it directly, the way a reducer or a pure helper is called, returns the
// element it would render: `React.createElement`'s result is a plain
// object, and nothing here inspects more of the tree than its own
// `.type`/`.props`. That is also true of `RetailIndex`/`ClinicIndex`
// (`app/index.tsx`/`app-clinic/index.tsx`), which have no hooks of their
// own either — each is one line returning `<Index home="..." />`.
//
// Every module reached from here is mocked, `expo-router` included:
// `Screen` (`components/ui`) and `expo-router` itself both reach into
// `react-native`, whose Flow-annotated source vitest's esbuild transform
// cannot parse, so nothing may load the real module at all. Mocking
// `Index`'s own three imports keeps `react-native` out of the module
// graph entirely, rather than mocking it directly.
//
// Checked red first: swapping the final `<Redirect href={home} />` for a
// hardcoded `<Redirect href="/till" />` failed "signed in: redirects to
// the home prop" for `home="/settings"` and left the retail-shaped case
// beside it green — which is why both are asserted, not one.

import { beforeEach, describe, expect, test, vi } from "vitest";

vi.mock("expo-router", () => ({ Redirect: () => null }));
vi.mock("../../components/ui", () => ({ Screen: () => null }));
vi.mock("../../providers/SessionProvider", () => ({ useSession: vi.fn() }));

import { Redirect } from "expo-router";

import ClinicIndex from "../../app-clinic/index";
import RetailIndex from "../../app/index";
import { useSession } from "../../providers/SessionProvider";
import Index from "../../screens/Index";

const session = vi.mocked(useSession);

/** A full `SessionState`-shaped fake (the type itself is not exported by
 *  `SessionProvider.tsx`; structural typing is enough for `mockReturnValue`
 *  to accept this), overridable one field at a time per test. */
function fakeSession(
  overrides: Partial<ReturnType<typeof useSession>> = {},
): ReturnType<typeof useSession> {
  return {
    device: "device-token",
    session: { deviceToken: "device-token", sessionToken: "s", name: "Amel", role: "cashier" },
    ready: true,
    lost: null,
    paired: async () => {},
    signedIn: async () => {},
    signOut: async () => {},
    sessionLost: async () => {},
    deviceLost: async () => {},
    ...overrides,
  };
}

beforeEach(() => {
  session.mockReset();
});

describe("Index: which redirect, from useSession()'s state and the home prop", () => {
  test("not ready: renders the empty frame, no redirect", () => {
    session.mockReturnValue(fakeSession({ ready: false }));
    const element = Index({ home: "/till" });
    expect(element.type).not.toBe(Redirect);
  });

  test("no device: redirects to pairing, whatever home is", () => {
    session.mockReturnValue(fakeSession({ device: null }));
    const element = Index({ home: "/till" });
    expect(element.type).toBe(Redirect);
    expect(element.props.href).toBe("/pair");
  });

  test("device but no session: redirects to sign-in, whatever home is", () => {
    session.mockReturnValue(fakeSession({ session: null }));
    const element = Index({ home: "/till" });
    expect(element.type).toBe(Redirect);
    expect(element.props.href).toBe("/sign-in");
  });

  test("signed in: redirects to the home prop (retail-shaped)", () => {
    session.mockReturnValue(fakeSession());
    const element = Index({ home: "/till" });
    expect(element.type).toBe(Redirect);
    expect(element.props.href).toBe("/till");
  });

  test("signed in: redirects to the home prop (clinic-shaped)", () => {
    session.mockReturnValue(fakeSession());
    const element = Index({ home: "/settings" });
    expect(element.type).toBe(Redirect);
    expect(element.props.href).toBe("/settings");
  });
});

describe("the route wrappers pass the home this root's own build should land on", () => {
  test("app/index.tsx (retail) passes /till", () => {
    const element = RetailIndex();
    expect(element.type).toBe(Index);
    expect(element.props.home).toBe("/till");
  });

  test("app-clinic/index.tsx (clinic) passes /settings, its only signed-in screen", () => {
    const element = ClinicIndex();
    expect(element.type).toBe(Index);
    expect(element.props.home).toBe("/settings");
  });
});
