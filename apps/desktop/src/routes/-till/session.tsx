// The drawer at the till: opened at sign-in with what is in it, and counted
// again at close (ruling 2b and ruling 4,
// `context/plans/20260921-till-shifts-a-float-and-a-count.md` T5). Every
// line `till.tsx` would otherwise have carried lives here, wired back with
// one import and one element, because `till.tsx` is pinned in
// `scripts/file-sizes.json` and may not grow.
//
// Neither DTO carries a moment (`dto/till.rs`'s own doc): the server stamps
// `opened_at` and the close's own moment, and this file never asks for
// either. A cashier's own figures come from `GET /till/shifts/open` before
// the close and from the close call's own answer after it —
// `GET /till/shifts/{id}` needs `see_reports`, which a cashier does not
// hold, so this file never calls it (`packages/shared/src/client/till.ts`
// carries it for the report screen T7 builds).
//
// Ruling 2b: the first sign-in of the day raises one popup, pre-filled with
// what the last close counted, with a box that opens silently at that
// amount from then on. What tells a fresh sign-in apart from an unlock is
// not the session — `establish()` in `lib/session.tsx` runs the same code
// for both — but the shift itself: unlocking finds the same shift still
// open (nothing closed it) and this file asks nothing, the way `till.tsx`'s
// own cart survives a lock because the route stays mounted underneath it.
// The "do not ask again" preference is what lets a fresh day skip the popup
// too, and it is kept in `localStorage` per `user_id`, because one desktop
// is shared by every cashier who signs in at it.
//
// Ruling 4: the close modal shows the expected figure — the server's, never
// `opening_cash + takings` worked out again here (that is
// `services::shifts::report`'s job and this screen reads its answer) —
// before the cashier types what is in the drawer, and a note is required
// the moment the two disagree. The migration's own CHECK
// (`shifts_a_difference_carries_a_reason`) is the backstop; this is the
// same rule with a box that will not submit.

import { useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ApiError } from "@dzpos/shared";
import type { ShiftDto, ShiftReportDto } from "@dzpos/shared";

import { api } from "@/api";
import { FormField } from "@/components/FormField";
import { Money } from "@/components/Money";
import { MoneyInput } from "@/components/MoneyInput";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useTranslation } from "@/i18n";
import { errorKey } from "@/lib/fields";
import { useSession } from "@/lib/session";

/** Its own key, not a slice of `productsQueryKey` and friends in
 * `apps/desktop/src/api.ts`: that file is not on this task's file list
 * (`till.tsx` is pinned, and the query keys there are shared by every
 * screen), and one query with one owner does not need a key another screen
 * could collide on. */
const OPEN_SHIFT_QUERY_KEY = ["till", "shift", "open"] as const;

/** What the "do not ask again" box remembers, per person: whether to open
 * silently at all, and the amount to open at. The amount is kept
 * separately from the box, and updated on every close whether or not the
 * box is ticked, because the ruling's own cost — a shift opening at last
 * night's figure when the owner emptied the change overnight — only makes
 * sense if "last night's figure" is actually the last close, not whatever
 * was typed the day the box was first ticked. */
interface OpenPreference {
  readonly autoOpen: boolean;
  readonly lastCountedCentimes: number | null;
}

const NO_PREFERENCE: OpenPreference = { autoOpen: false, lastCountedCentimes: null };

function storageKey(userId: number): string {
  return `dzpos:till-open:${userId}`;
}

/** Reads one field of a value already known to be an object, without a
 * cast: the same shape `apps/desktop/src/api.ts`'s own `injected` reads a
 * page global with, for the same reason — a field that is not there, or is
 * the wrong shape, is read as absent rather than trusted. */
function field(source: object, name: string): unknown {
  return name in source ? Reflect.get(source, name) : undefined;
}

function loadPreference(userId: number): OpenPreference {
  if (typeof window === "undefined") return NO_PREFERENCE;
  try {
    const raw = window.localStorage.getItem(storageKey(userId));
    if (raw === null) return NO_PREFERENCE;
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== "object" || parsed === null) return NO_PREFERENCE;
    const autoOpen = field(parsed, "autoOpen");
    const lastCounted = field(parsed, "lastCountedCentimes");
    return {
      autoOpen: autoOpen === true,
      // A tampered or corrupted stored figure never reaches the auto-open
      // path: the server refuses a negative `opening_cash` anyway
      // (`services::shifts::open`), but a figure the dialog's own bound
      // (`countInvalid`/`invalid`) would never let anyone type should not
      // be posted at all — the field falls back to no preference instead.
      lastCountedCentimes:
        typeof lastCounted === "number" && Number.isSafeInteger(lastCounted) && lastCounted >= 0
          ? lastCounted
          : null,
    };
  } catch {
    return NO_PREFERENCE;
  }
}

function savePreference(userId: number, next: OpenPreference): void {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(storageKey(userId), JSON.stringify(next));
  } catch {
    // No storage (tests, private mode): the popup simply asks again next
    // time, which is the safe direction to fail in.
  }
}

/** The bar `till.tsx` mounts: a badge while a drawer is open, a button to
 * open one otherwise, and the two dialogs. Renders nothing until somebody
 * is signed in — `useSession()`'s `me` is what the "do not ask again" box
 * is keyed on, and there is nothing to ask before that. */
export function TillShiftBar({ className }: { className?: string }) {
  const { t } = useTranslation();
  const { me } = useSession();
  const queryClient = useQueryClient();

  const openShift = useQuery({
    queryKey: OPEN_SHIFT_QUERY_KEY,
    queryFn: () => api.getOpenShift(),
    enabled: me !== null,
  });

  const [openDialogShown, setOpenDialogShown] = useState(false);
  const [closeDialogShown, setCloseDialogShown] = useState(false);
  const [openError, setOpenError] = useState<string | null>(null);
  const [closeError, setCloseError] = useState<string | null>(null);
  // Set when the server refused for a missing note on a count this screen's
  // own `differs` believed matched the expected figure — a sale rung between
  // the `refetch()` on opening this modal and the submit moved the expected
  // figure out from under it. `services::shifts::close`'s own guard is the
  // one that actually caught it; without this the note box the cashier needs
  // would never appear, because the client-side gate that shows it is still
  // reading the expected figure this screen fetched a moment too early.
  const [closeNoteRequired, setCloseNoteRequired] = useState(false);
  const [lastClosed, setLastClosed] = useState<ShiftDto | null>(null);
  // Asked once per mount. `till.tsx`'s route stays mounted through a lock
  // (the comment at the top of this file), so an unlock never sets this
  // back to false; only a fresh mount of the till route — a real sign-in —
  // does, and by then the query above will have found the same shift still
  // open if nobody closed it.
  const asked = useRef(false);

  const openMutation = useMutation({
    mutationFn: (openingCashCentimes: number) =>
      api.openShift({ opening_cash_centimes: openingCashCentimes }),
    onSuccess: async () => {
      setOpenDialogShown(false);
      setOpenError(null);
      setLastClosed(null);
      await queryClient.invalidateQueries({ queryKey: OPEN_SHIFT_QUERY_KEY });
    },
    onError: (error: unknown) => {
      setOpenError(t(errorKey(error)));
      // Unconditional: on the manual path the dialog is already open, so
      // this is a no-op; on the auto path (ruling 2b's "opens silently")
      // this is the only place a refusal — ordinarily a shop PC's clock
      // drifting behind this person's own last close — ever reaches the
      // screen. Without it the auto-open mutation fails silently and every
      // sale that follows is tagged `till.sale_outside_shift` with nobody
      // told why.
      setOpenDialogShown(true);
    },
  });

  const closeMutation = useMutation({
    mutationFn: ({ id, counted, note }: { id: number; counted: number; note: string | null }) =>
      api.closeShift(id, { counted_centimes: counted, note }),
    onSuccess: async (closed) => {
      setCloseDialogShown(false);
      setCloseError(null);
      setCloseNoteRequired(false);
      setLastClosed(closed);
      await queryClient.invalidateQueries({ queryKey: OPEN_SHIFT_QUERY_KEY });
    },
    onError: (error: unknown) => {
      // The one refusal this screen never composes its own wording for:
      // a manager closing somebody else's drawer past a coarse permission
      // both roles hold is the core's `close_another_persons_till` check,
      // and the sentence it sends already names the permission.
      setCloseError(
        error instanceof ApiError && error.code === "forbidden" ? error.message : t(errorKey(error)),
      );
      const noteRequired =
        error instanceof ApiError && error.code === "validation" && error.field === "note";
      setCloseNoteRequired(noteRequired);
      // This refusal only fires when the count typed looked exact against
      // the expected figure this screen fetched a moment too early — a sale
      // rung between the trigger's own `refetch()` and this submit moved it.
      // Without refetching again here, the expected figure and the
      // difference preview stay on that stale number while the note is
      // demanded, which reads as "the drawer is fine, but sign a note".
      if (noteRequired) void openShift.refetch();
    },
  });

  useEffect(() => {
    if (me === null) return;
    if (!openShift.isSuccess || openShift.data !== null) return;
    if (asked.current) return;
    asked.current = true;
    const preference = loadPreference(me.user_id);
    if (preference.autoOpen && preference.lastCountedCentimes !== null) {
      openMutation.mutate(preference.lastCountedCentimes);
      return;
    }
    setOpenDialogShown(true);
    // `openMutation` is a fresh object on every render; `asked` is what
    // keeps this from running twice, not the dependency list below.
  }, [me, openShift.isSuccess, openShift.data]);

  if (me === null) return null;

  const preference = loadPreference(me.user_id);
  const report: ShiftReportDto | null = openShift.data ?? null;

  return (
    <div className={className}>
      {report !== null ? (
        <Card>
          <CardContent className="flex flex-wrap items-center justify-between gap-3 py-3">
            <div className="flex items-center gap-2">
              <Badge data-testid="till-shift-badge">{t("till_shift_bar_open")}</Badge>
              <span className="text-sm text-muted-foreground">
                {t("till_shift_bar_opening_cash")}
              </span>
              <Money centimes={report.shift.opening_cash_centimes} />
            </div>
            <Button
              type="button"
              variant="outline"
              data-testid="till-close-trigger"
              onClick={() => {
                setCloseError(null);
                setCloseNoteRequired(false);
                setCloseDialogShown(true);
                void openShift.refetch();
              }}
            >
              {t("action_close_till")}
            </Button>
          </CardContent>
        </Card>
      ) : null}

      {/* Nothing while the query is still pending or has failed: a button
          whose only dialog cannot show an expected figure or read the caller's
          own drawer back is worse than no button, and `till.test.tsx` (whose
          catch-all stub answers this call with an unrelated shape) is the
          proof a screen without this guard renders it on every one of its 57
          cases. */}
      {report === null && openShift.isSuccess ? (
        <Button
          type="button"
          variant="outline"
          size="sm"
          data-testid="till-open-trigger"
          onClick={() => {
            setOpenError(null);
            setOpenDialogShown(true);
          }}
        >
          {t("action_open_till")}
        </Button>
      ) : null}

      {lastClosed !== null ? (
        <p role="status" data-testid="till-closed-banner" className="mt-2 flex items-center gap-2">
          <span>{t("till_shift_closed")}</span>
          {lastClosed.difference_centimes === null ? null : (
            <Money centimes={lastClosed.difference_centimes} data-testid="till-closed-difference" />
          )}
        </p>
      ) : null}

      <OpenShiftDialog
        open={openDialogShown}
        onOpenChange={setOpenDialogShown}
        defaultCentimes={preference.lastCountedCentimes}
        pending={openMutation.isPending}
        error={openError}
        onSubmit={(amount, remember) => {
          openMutation.mutate(amount, {
            onSuccess: () => savePreference(me.user_id, { autoOpen: remember, lastCountedCentimes: amount }),
          });
        }}
      />
      <CloseShiftDialog
        open={closeDialogShown}
        onOpenChange={setCloseDialogShown}
        report={report}
        pending={closeMutation.isPending}
        error={closeError}
        forceNote={closeNoteRequired}
        onSubmit={(counted, note) => {
          if (report === null) return;
          closeMutation.mutate(
            { id: report.shift.id, counted, note },
            {
              onSuccess: () =>
                savePreference(me.user_id, { ...loadPreference(me.user_id), lastCountedCentimes: counted }),
            },
          );
        }}
      />
    </div>
  );
}

/** The popup at sign-in: "opening the till", the float typed in, and the
 * box that skips this dialog on every sign-in after. */
function OpenShiftDialog({
  open,
  onOpenChange,
  defaultCentimes,
  pending,
  error,
  onSubmit,
}: {
  open: boolean;
  onOpenChange: (next: boolean) => void;
  defaultCentimes: number | null;
  pending: boolean;
  error: string | null;
  onSubmit: (openingCashCentimes: number, remember: boolean) => void;
}) {
  const { t } = useTranslation();
  const [amount, setAmount] = useState<number | null>(defaultCentimes);
  const [remember, setRemember] = useState(false);

  useEffect(() => {
    if (!open) return;
    setAmount(defaultCentimes);
    setRemember(false);
  }, [open, defaultCentimes]);

  const invalid = amount === null || amount < 0;

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next && pending) return;
        onOpenChange(next);
      }}
    >
      <DialogContent data-testid="till-open-dialog">
        <DialogHeader>
          <DialogTitle>{t("till_shift_open_title")}</DialogTitle>
          <DialogDescription>{t("till_shift_open_question")}</DialogDescription>
        </DialogHeader>
        <FormField
          label={t("field_opening_cash")}
          error={error ?? (invalid && amount !== null ? t("error_price_invalid") : undefined)}
        >
          {(parts) => (
            <MoneyInput {...parts} data-testid="till-open-amount" value={amount} onChange={setAmount} />
          )}
        </FormField>
        <div className="flex items-center gap-2">
          <Checkbox
            id="till-shift-remember"
            data-testid="till-open-remember"
            checked={remember}
            onCheckedChange={(next) => setRemember(next === true)}
          />
          <Label htmlFor="till-shift-remember">{t("till_shift_remember")}</Label>
        </div>
        <DialogFooter>
          <Button type="button" variant="ghost" disabled={pending} onClick={() => onOpenChange(false)}>
            {t("action_open_till_later")}
          </Button>
          <Button
            type="button"
            disabled={pending || invalid}
            data-testid="till-open-submit"
            onClick={() => {
              if (amount === null || amount < 0) return;
              onSubmit(amount, remember);
            }}
          >
            {pending ? t("action_opening_till") : t("action_open_till")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

/** The modal at close: the expected figure the server already worked out,
 * what the cashier counted, the difference once both are known, and a note
 * that is required — and blocks the submit — the moment the two differ. */
function CloseShiftDialog({
  open,
  onOpenChange,
  report,
  pending,
  error,
  forceNote,
  onSubmit,
}: {
  open: boolean;
  onOpenChange: (next: boolean) => void;
  report: ShiftReportDto | null;
  pending: boolean;
  error: string | null;
  /** The server refused the last submit for a missing note on a count this
   * screen believed matched the expected figure — read the doc beside
   * `closeNoteRequired` in `TillShiftBar` for why that happens. The note box
   * shows and is required exactly as if the figures differed, even though
   * `differs` below still reads them as equal. */
  forceNote: boolean;
  onSubmit: (countedCentimes: number, note: string | null) => void;
}) {
  const { t } = useTranslation();
  const [counted, setCounted] = useState<number | null>(null);
  const [note, setNote] = useState("");

  useEffect(() => {
    if (!open) return;
    setCounted(null);
    setNote("");
  }, [open]);

  const expected = report?.expected_centimes ?? null;
  const differs = (counted !== null && expected !== null && counted !== expected) || forceNote;
  const noteMissing = differs && note.trim() === "";
  const countInvalid = counted !== null && counted < 0;
  const canSubmit = counted !== null && !countInvalid && expected !== null && !noteMissing;

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next && pending) return;
        onOpenChange(next);
      }}
    >
      <DialogContent data-testid="till-close-dialog">
        <DialogHeader>
          <DialogTitle>{t("till_shift_close_title")}</DialogTitle>
          <DialogDescription>{t("till_shift_close_question")}</DialogDescription>
        </DialogHeader>

        <p className="flex items-center justify-between gap-2">
          <span>{t("till_shift_expected")}</span>
          {expected === null ? null : <Money centimes={expected} data-testid="till-expected" />}
        </p>

        <FormField
          label={t("field_counted_cash")}
          error={countInvalid ? t("error_price_invalid") : undefined}
        >
          {(parts) => (
            <MoneyInput {...parts} data-testid="till-counted" value={counted} onChange={setCounted} />
          )}
        </FormField>

        {differs ? (
          <p className="flex items-center justify-between gap-2">
            <span>{t("till_shift_difference")}</span>
            {/* A preview only: the stored figure is the server's own
                `difference_centimes` on the close answer, computed by the
                core's checked subtraction. Both operands here are already
                safe integers of centimes, so a plain subtraction for a
                figure never stored is not the float this rule is about. */}
            <Money
              centimes={counted !== null && expected !== null ? counted - expected : 0}
              data-testid="till-difference"
            />
          </p>
        ) : null}

        {differs ? (
          <FormField
            label={t("field_shift_note")}
            error={noteMissing ? t("error_shift_note_required") : undefined}
          >
            {(parts) => (
              <Input
                {...parts}
                data-testid="till-close-note"
                value={note}
                onChange={(event) => setNote(event.target.value)}
              />
            )}
          </FormField>
        ) : null}

        {error === null ? null : (
          <p role="alert" className="text-sm text-fg-danger">
            {error}
          </p>
        )}

        <DialogFooter>
          <Button type="button" variant="ghost" disabled={pending} onClick={() => onOpenChange(false)}>
            {t("action_cancel")}
          </Button>
          <Button
            type="button"
            disabled={pending || !canSubmit}
            data-testid="till-close-submit"
            onClick={() => {
              if (counted === null || countInvalid) return;
              onSubmit(counted, differs ? note.trim() : null);
            }}
          >
            {pending ? t("action_closing_till") : t("action_close_till")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
