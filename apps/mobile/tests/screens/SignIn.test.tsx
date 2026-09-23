// `screens/SignIn.tsx` calls `router.replace(afterSignIn)` once, deep
// inside its own `signIn()` closure, only after a name is picked, a PIN is
// typed and the server rings back "rang" — a static read of the source
// cannot tell that prop actually reaches that call rather than a
// forgotten literal beside it (C7 review fix #4). Driving the real
// component through pick -> type -> submit is what proves it.
//
// `react-test-renderer` renders the real component tree with real
// `useState`, without the DOM `@testing-library/react` needs and without
// the native modules `@testing-library/react-native` (`jest-expo`,
// `react-native`'s own Jest preset) would need — nothing here reaches a
// device, so nothing here needs one. Every module `SignIn` imports that
// itself reaches into `react-native` (`components/ui`, `design/theme`,
// `providers/LanguageProvider`, `react-native` directly) is mocked with
// the smallest stand-in that keeps `.props` readable and, for `Screen`
// and `View`, still mounts its children: `react-native`'s own
// Flow-annotated source is unparseable by vitest's esbuild transform, so
// nothing may load the real module transitively either.
//
// Checked red first: replacing `router.replace(afterSignIn)` with
// `router.replace("/till")` in `screens/SignIn.tsx` failed the clinic case
// below (`afterSignIn="/settings"`) and left the retail case, which wants
// "/till" anyway, green — the same shape the review named for `Index`.

import type { StaffDto } from "@dzpos/shared";
import { PIN_DIGITS } from "@dzpos/shared";
import type { ReactNode } from "react";
import { act, create } from "react-test-renderer";
import { beforeEach, describe, expect, test, vi } from "vitest";

const replace = vi.hoisted(() => vi.fn());
const staffQuery = vi.hoisted(() => ({
  isPending: false,
  isError: false,
  data: [] as readonly StaffDto[],
  refetch: vi.fn(),
}));

vi.mock("expo-router", () => ({
  Redirect: () => null,
  useRouter: () => ({ replace }),
}));

vi.mock("@tanstack/react-query", () => ({
  useQuery: () => staffQuery,
}));

vi.mock("react-native", () => ({
  View: (props: { children?: ReactNode }) => <>{props.children}</>,
  Pressable: (_props: { onPress?: () => void; children?: ReactNode }) => null,
}));

vi.mock("../../design/theme", () => ({
  // Every field `SignIn.tsx` reads off `theme` for a `style` prop, whose
  // value never reaches an assertion below (`View` ignores `style`
  // entirely) — present only so a property read does not throw.
  useTheme: () => ({
    space: [0, 4, 8, 12, 16, 20, 24],
    radius: { sm: 4, md: 8 },
    layout: { "touch-min": 44 },
    colors: {
      border: { default: "#000" },
      surface: { raised: "#111", card: "#222" },
    },
  }),
}));

vi.mock("../../providers/LanguageProvider", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock("../../providers/SessionProvider", () => ({
  useSession: () => ({
    device: "device-token",
    session: null,
    ready: true,
    lost: null,
    paired: async () => {},
    signedIn: async () => {},
    signOut: async () => {},
    sessionLost: async () => {},
    deviceLost: async () => {},
  }),
}));

vi.mock("../../lib/api", () => ({
  ApiRefusal: class ApiRefusal extends Error {},
  call: vi.fn(async () => ({
    outcome: { kind: "rang" },
    body: { token: "session-token", me: { name: "Amel", role: "cashier" } },
    status: 200,
  })),
  get: vi.fn(async () => []),
}));

vi.mock("../../components/ui", () => ({
  Screen: (props: { children?: ReactNode }) => <>{props.children}</>,
  Text: () => null,
  Callout: () => null,
  Field: () => null,
  Button: (_props: { title: string; onPress?: () => void; disabled?: boolean }) => null,
  PinBoxes: (_props: { onChange: (next: string) => void }) => null,
}));

import { Pressable } from "react-native";

import { Button, PinBoxes } from "../../components/ui";

import ClinicSignIn from "../../app-clinic/sign-in";
import RetailSignIn from "../../app/sign-in";
import SignIn from "../../screens/SignIn";

const PERSON: StaffDto = { id: 1, name: "Amel", role: "cashier", has_pin: true, has_password: false };

beforeEach(() => {
  replace.mockReset();
  staffQuery.data = [PERSON];
});

/** Picks the one staff member the mocked query returns, types a PIN of
 *  the right length and submits — every step wrapped in `act()`, since
 *  each one is a real `useState` update or, for the submit, an awaited
 *  one. */
async function pickTypeAndSubmit(root: ReturnType<typeof create>) {
  const pressable = root.root.findByType(vi.mocked(Pressable));
  act(() => {
    pressable.props.onPress?.();
  });

  const pinBoxes = root.root.findByType(vi.mocked(PinBoxes));
  act(() => {
    pinBoxes.props.onChange("1".repeat(PIN_DIGITS));
  });

  const submit = root.root
    .findAllByType(vi.mocked(Button))
    .find((instance) => instance.props.title === "auth_sign_in");
  if (submit === undefined) throw new Error("the sign-in button was not found");
  await act(async () => {
    submit.props.onPress?.();
  });
}

/** `create()` outside `act()` leaves react-test-renderer's root looking
 *  unmounted the moment the staff list has a row in it (an effect fired
 *  during the initial commit that a bare `create()` never waits out): every
 *  render in this file goes through here rather than through `create()`
 *  directly. */
function renderScreen(element: Parameters<typeof create>[0]): ReturnType<typeof create> {
  let root: ReturnType<typeof create> | undefined;
  act(() => {
    root = create(element);
  });
  if (root === undefined) throw new Error("create() did not run inside act()");
  return root;
}

describe("SignIn: router.replace(afterSignIn) once the server rings back", () => {
  test("retail's own copy replaces to /till", async () => {
    const root = renderScreen(<RetailSignIn />);
    await pickTypeAndSubmit(root);
    expect(replace).toHaveBeenCalledWith("/till");
  });

  test("clinic's own copy replaces to /settings, its only signed-in screen", async () => {
    const root = renderScreen(<ClinicSignIn />);
    await pickTypeAndSubmit(root);
    expect(replace).toHaveBeenCalledWith("/settings");
  });

  test("the bare screen replaces to whatever afterSignIn it was given", async () => {
    const root = renderScreen(<SignIn afterSignIn="/patients" />);
    await pickTypeAndSubmit(root);
    expect(replace).toHaveBeenCalledWith("/patients");
  });
});
