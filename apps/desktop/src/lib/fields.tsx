// The pieces a form that takes money is built out of, and the reading of a
// refusal the server sent. The customers screen wrote them first and the
// suppliers screen needs the same ones; they live here rather than in one of
// the two, because a screen importing a field out of another screen is how
// two screens become one screen nobody can move.
//
// The other screens still carry their own copies. They are moved as each is
// next opened, not in a sweep that touches five files a task is not about.

import { ApiError, formatCentimes, parseAmountToCentimes } from "@dzpos/shared";

import { isKey, useTranslation, type Key } from "@/i18n";

/** What the UI says for a code the server sent. The server sends a code,
 *  never a sentence; the UI owns the wording. */
const ERROR_KEY: Record<string, Key> = {
  validation: "error_validation",
  conflict: "error_conflict",
  not_found: "error_not_found",
  money: "error_money",
  storage: "error_storage",
  restart_needed: "error_restart_needed",
  bad_request: "error_bad_request",
  bad_response: "error_bad_response",
  unauthorized: "error_unauthorized",
  unreachable: "error_unreachable",
  forbidden: "error_forbidden",
};

export function errorKey(error: unknown): Key {
  if (error instanceof ApiError) {
    return ERROR_KEY[error.code] ?? "error_unknown";
  }
  return "error_unknown";
}

/** An amount drawn positive whatever its sign. A balance is one signed
 *  number, and "Créance -1 000,00" is not a sentence anyone says at a
 *  counter, so the sign is spent on the word beside the figure instead. */
export function shownPositive(centimes: number): string {
  return formatCentimes(Math.abs(centimes));
}

/** Blank is "nothing", not an empty string: the column is cleared. */
export function cleared(text: string): string | null {
  const trimmed = text.trim();
  return trimmed === "" ? null : trimmed;
}

/** Blank is "no amount at all"; anything unreadable was refused by the
 *  validator before this runs. */
export function amount(text: string): number | null {
  if (text.trim() === "") return null;
  return parseAmountToCentimes(text);
}

/** Whether an optional amount can be read: blank counts, since blank means
 *  the field was left empty. */
export function readable(text: string): boolean {
  return text.trim() === "" || parseAmountToCentimes(text) !== null;
}

/** An amount in dinars, typed by a person. */
export function AmountField({
  label,
  hint,
  value,
  onChange,
  errors,
}: {
  label: string;
  hint?: string;
  value: string;
  onChange: (value: string) => void;
  errors: unknown[];
}) {
  // The hint sits outside the label on purpose: inside it, it would be read
  // as part of the field's name, and a test or a screen reader asking for
  // "Créance de départ (DA)" would not find the box.
  return (
    <div className="flex flex-col gap-1">
      <label className="flex flex-col gap-1">
        <span>{label}</span>
        <input
          dir="ltr"
          inputMode="decimal"
          className="rounded border px-2 py-1 font-mono text-end"
          value={value}
          onChange={(e) => onChange(e.target.value)}
        />
      </label>
      {hint === undefined ? null : <span className="text-sm opacity-70">{hint}</span>}
      <FieldError messages={errors} />
    </div>
  );
}

/** Field validators return translation keys, never sentences. */
export function FieldError({ messages }: { messages: unknown[] }) {
  const { t } = useTranslation();
  const key = messages.find((m): m is string => typeof m === "string");
  if (key === undefined) return null;
  return (
    <span role="alert" className="text-sm text-red-700">
      {t(isKey(key) ? key : "error_unknown")}
    </span>
  );
}
