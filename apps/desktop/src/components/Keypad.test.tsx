// The pad's own contract: which keys it draws, what a press sends out, and
// that the keyboard says the same thing the keys do.
//
// What is not here is any idea of an amount. The pad sends a token and the
// screen decides what it means, so the arithmetic is tested where it lives
// (the till's cash box), not twice.

import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, test } from "vitest";

import { I18nProvider } from "@/i18n";

import { Keypad, keyedAmount, keyedDigits, type KeypadKey } from "./Keypad";

/** The pad, and the keys it sent, in order. */
function mount(disabled = false): KeypadKey[] {
  const pressed: KeypadKey[] = [];
  render(
    <I18nProvider lang="fr">
      <Keypad onKey={(key) => pressed.push(key)} disabled={disabled} />
    </I18nProvider>,
  );
  return pressed;
}

const FACE = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "00"];

describe("Keypad", () => {
  test("draws the ten digits, the double zero, a backspace and the wide key", () => {
    mount();
    for (const key of FACE) {
      expect(screen.getByRole("button", { name: key })).toBeInTheDocument();
    }
    expect(screen.getByRole("button", { name: "Effacer un chiffre" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Valider" })).toBeInTheDocument();
  });

  test("sends the key that was pressed and nothing else", async () => {
    const user = userEvent.setup();
    const pressed = mount();

    await user.click(screen.getByRole("button", { name: "7" }));
    await user.click(screen.getByRole("button", { name: "00" }));
    await user.click(screen.getByRole("button", { name: "Effacer un chiffre" }));
    await user.click(screen.getByRole("button", { name: "Valider" }));

    expect(pressed).toEqual(["7", "00", "backspace", "enter"]);
  });

  /**
   * The mirror. A cashier reaching for the numeric row of the keyboard gets
   * what the pad would have given them, so the two ways of typing an amount
   * cannot disagree.
   */
  test("the keyboard says the same thing the keys do", async () => {
    const user = userEvent.setup();
    const pressed = mount();

    screen.getByRole("button", { name: "5" }).focus();
    await user.keyboard("4");
    await user.keyboard("{Backspace}");

    expect(pressed).toEqual(["4", "backspace"]);
  });

  test("with captureWindow, a digit types without a key being focused first", async () => {
    const user = userEvent.setup();
    const pressed: KeypadKey[] = [];
    render(
      <I18nProvider lang="fr">
        <Keypad captureWindow onKey={(key) => pressed.push(key)} />
      </I18nProvider>,
    );

    await user.keyboard("1");
    await user.keyboard("{Enter}");

    expect(pressed).toEqual(["1", "enter"]);
  });

  /**
   * The double count this test exists for: a focused key already fires its
   * own click on Enter, so the pad must not count that press a second time.
   */
  test("Enter on a key of the pad is one press, not two", async () => {
    const user = userEvent.setup();
    const pressed = mount();

    screen.getByRole("button", { name: "5" }).focus();
    await user.keyboard("{Enter}");

    expect(pressed).toEqual(["5"]);
  });

  /**
   * The pad on an Arabic till. The page is `rtl`, so a grid left to itself
   * puts the first key on the right and the cashier's thumb lands on the 3
   * where the 1 used to be. The assertion is the box the keys are laid out
   * in, because that is what decides the order: the `dir` nearest a key is
   * the pad's own `ltr` and not the document's `rtl`.
   */
  test("the keys run left to right on an Arabic till too", () => {
    render(
      <I18nProvider lang="ar">
        <Keypad onKey={() => {}} />
      </I18nProvider>,
    );

    expect(document.documentElement.dir).toBe("rtl");
    for (const key of FACE) {
      expect(screen.getByRole("button", { name: key }).closest("[dir]")?.getAttribute("dir")).toBe(
        "ltr",
      );
    }
  });

  test("sends nothing at all while it is disabled", async () => {
    const user = userEvent.setup();
    const pressed = mount(true);

    await user.click(screen.getByRole("button", { name: "3" }));
    fireEvent.keyDown(screen.getByTestId("keypad"), { key: "3" });

    expect(pressed).toEqual([]);
  });
});

/**
 * What a press does to an amount. The pad types whole dinars, so the digits
 * land where a cashier saying "mille cinq cents" expects them, and an empty
 * box stays the empty box rather than becoming a zero somebody typed.
 */
describe("keyedAmount", () => {
  /** The keys pressed in order, from an empty box. */
  function typed(keys: readonly KeypadKey[]): number | null {
    return keys.reduce<number | null>((amount, key) => keyedAmount(amount, key), null);
  }

  test("a run of digits is that many dinars, in centimes", () => {
    expect(typed(["1", "5", "0", "0"])).toBe(150_000);
  });

  test("the double zero is two of them", () => {
    expect(typed(["7", "00"])).toBe(70_000);
  });

  test("a leading zero is not kept, so 0 then 5 is five dinars", () => {
    expect(typed(["0", "5"])).toBe(500);
  });

  test("backspace takes the last dinar off and empties the box at the end", () => {
    expect(typed(["1", "2", "backspace"])).toBe(100);
    expect(typed(["4", "backspace"])).toBeNull();
    expect(typed(["backspace"])).toBeNull();
  });

  test("a centime typed in the box itself is dropped, never shifted up", () => {
    // 12,50 with a 5 pressed on the pad is 125 DA, not 125,05.
    expect(keyedAmount(1_250, "5")).toBe(12_500);
  });

  test("stops taking digits rather than growing past a billion dinars", () => {
    const nine = typed(["1", "2", "3", "4", "5", "6", "7", "8", "9"]);
    expect(nine).toBe(123_456_789 * 100);
    expect(keyedAmount(nine, "1")).toBe(nine);
  });

  test("validating is not an amount and changes nothing", () => {
    expect(keyedAmount(150_000, "enter")).toBe(150_000);
    expect(keyedAmount(null, "enter")).toBeNull();
  });
});

/**
 * What a press does to a string of digits: a user id, a PIN. Every digit
 * typed is kept in order and a leading zero is a digit like any other,
 * because `0512` and `512` are two different PINs.
 */
describe("keyedDigits", () => {
  /** The keys pressed in order, from an empty string, capped at `max`. */
  function typed(keys: readonly KeypadKey[], max = 6): string {
    return keys.reduce<string>((digits, key) => keyedDigits(digits, key, max), "");
  }

  test("a run of digits is kept in order", () => {
    expect(typed(["1", "3", "7", "9"])).toBe("1379");
  });

  test("a leading zero is kept, unlike an amount", () => {
    expect(typed(["0", "5", "1", "2"])).toBe("0512");
  });

  test("the double zero types two digits", () => {
    expect(typed(["1", "00", "2"])).toBe("1002");
  });

  test("backspace on an empty string stays empty", () => {
    expect(typed(["backspace"])).toBe("");
    expect(keyedDigits("", "backspace", 6)).toBe("");
  });

  test("backspace takes the last digit off", () => {
    expect(typed(["1", "3", "7", "backspace"])).toBe("13");
  });

  test("stops taking digits past the cap, and 00 is refused whole rather than truncated", () => {
    expect(typed(["1", "2", "3", "4"], 3)).toBe("123");
    expect(keyedDigits("12", "00", 3)).toBe("12");
  });

  test("validating is not a digit and changes nothing", () => {
    expect(keyedDigits("1379", "enter", 6)).toBe("1379");
    expect(keyedDigits("", "enter", 6)).toBe("");
  });

  test("does not mutate its input", () => {
    const before = "13";
    const digits = keyedDigits(before, "7", 6);
    expect(before).toBe("13");
    expect(digits).toBe("137");
  });
});
