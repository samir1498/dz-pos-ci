// Absence blocks (C6): a cashier holds `edit_patients` and the server lets
// her create and remove a block and move or cancel what it hits, but the
// settings room is gated on `edit_settings` alone (`settings.tsx`'s own
// `SETTINGS_SECTIONS`). This form sits on the book screen itself instead,
// behind the same nav gate (`view_patients`) that already opens `/book`,
// so the tool she is allowed to use is one she can reach.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import type { AbsenceBlockDto, AppointmentDto } from "@dzpos/shared";

import { FormField } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { absenceBlocksQueryKey, api } from "@/api";
import { useTranslation } from "@/i18n";
import { errorKey } from "@/lib/fields";

export function AbsenceBlocksForm() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [startsAt, setStartsAt] = useState("");
  const [endsAt, setEndsAt] = useState("");
  const [label, setLabel] = useState("");
  const [hits, setHits] = useState<readonly AppointmentDto[]>([]);

  const blocks = useQuery({ queryKey: absenceBlocksQueryKey, queryFn: () => api.listAbsenceBlocks() });

  const invalidate = () => queryClient.invalidateQueries({ queryKey: absenceBlocksQueryKey });

  const create = useMutation({
    mutationFn: () =>
      api.createAbsenceBlock({
        starts_at: `${startsAt.replace("T", " ")}:00`,
        ends_at: `${endsAt.replace("T", " ")}:00`,
        label: label.trim() === "" ? null : label.trim(),
      }),
    onSuccess: async (made) => {
      setStartsAt("");
      setEndsAt("");
      setLabel("");
      setHits(made.hits);
      await invalidate();
    },
  });
  const remove = useMutation({
    mutationFn: (id: string) => api.removeAbsenceBlock(id),
    onSuccess: invalidate,
  });
  const cancelHit = useMutation({
    mutationFn: (id: string) => api.cancelAppointment(id),
    onSuccess: (_, id) => setHits((current) => current.filter((hit) => hit.id !== id)),
  });
  // The hit's own visit type is not on `AppointmentDto` (only the minutes it
  // was booked for, `slot_minutes`), so this asks for the shop's own default
  // slot length rather than the hit's exact one; a longer visit could still
  // be offered a slot too short for it and be refused there too.
  const NO_FREE_SLOT = "no-free-slot";
  const moveHit = useMutation({
    mutationFn: async (hit: AppointmentDto) => {
      const free = await api.nextFreeSlot({});
      if (free.starts_at === null) throw new Error(NO_FREE_SLOT);
      return api.moveAppointment(hit.id, { starts_at: free.starts_at });
    },
    onSuccess: (_, hit) => setHits((current) => current.filter((h) => h.id !== hit.id)),
  });
  const moveHitErrorKey =
    moveHit.error instanceof Error && moveHit.error.message === NO_FREE_SLOT
      ? "book_no_free_slot"
      : moveHit.error === null
        ? null
        : errorKey(moveHit.error);

  return (
    <section className="flex flex-col gap-3" data-testid="absence-blocks-form">
      <h3 className="text-sm font-semibold text-foreground">{t("book_settings_blocks_title")}</h3>
      {blocks.isError ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(blocks.error))}
        </p>
      ) : null}
      <ul className="flex flex-col gap-1">
        {(blocks.data?.blocks ?? []).map((block: AbsenceBlockDto) => (
          <li key={block.id} className="flex items-center justify-between gap-2 text-sm">
            <span dir="ltr" className="font-numeric">
              {block.starts_at} — {block.ends_at}
            </span>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              data-testid={`block-remove-${block.id}`}
              disabled={remove.isPending}
              onClick={() => remove.mutate(block.id)}
            >
              {t("book_settings_remove")}
            </Button>
          </li>
        ))}
      </ul>

      <div className="flex flex-wrap items-end gap-2">
        <FormField label={t("book_settings_block_starts")}>
          {(parts) => (
            <Input
              {...parts}
              type="datetime-local"
              data-testid="block-starts"
              value={startsAt}
              onChange={(event) => setStartsAt(event.target.value)}
            />
          )}
        </FormField>
        <FormField label={t("book_settings_block_ends")}>
          {(parts) => (
            <Input
              {...parts}
              type="datetime-local"
              data-testid="block-ends"
              value={endsAt}
              onChange={(event) => setEndsAt(event.target.value)}
            />
          )}
        </FormField>
        <FormField label={t("book_settings_block_label")}>
          {(parts) => (
            <Input
              {...parts}
              data-testid="block-label"
              value={label}
              onChange={(event) => setLabel(event.target.value)}
            />
          )}
        </FormField>
        {create.isError ? (
          <p role="alert" className="text-sm text-fg-danger">
            {t(errorKey(create.error))}
          </p>
        ) : null}
        <Button
          type="button"
          data-testid="block-create"
          disabled={create.isPending || startsAt === "" || endsAt === ""}
          onClick={() => create.mutate()}
        >
          {t("book_settings_add")}
        </Button>
      </div>

      {hits.length === 0 ? null : (
        <div className="flex flex-col gap-2 rounded-md border border-border p-3">
          <p className="text-sm text-foreground">{t("book_settings_block_hits")}</p>
          {moveHitErrorKey === null ? null : (
            <p role="alert" className="text-sm text-fg-danger">
              {t(moveHitErrorKey)}
            </p>
          )}
          {cancelHit.isError ? (
            <p role="alert" className="text-sm text-fg-danger">
              {t(errorKey(cancelHit.error))}
            </p>
          ) : null}
          <ul className="flex flex-col gap-1">
            {hits.map((hit) => (
              <li key={hit.id} className="flex items-center justify-between gap-2 text-sm">
                <span>
                  {hit.first_name} {hit.last_name}{" "}
                  <span dir="ltr" className="font-numeric text-muted-foreground">
                    {hit.starts_at}
                  </span>
                </span>
                <div className="flex gap-1">
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    data-testid={`hit-move-${hit.id}`}
                    disabled={moveHit.isPending}
                    onClick={() => moveHit.mutate(hit)}
                  >
                    {t("book_move")}
                  </Button>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    data-testid={`hit-cancel-${hit.id}`}
                    disabled={cancelHit.isPending}
                    onClick={() => cancelHit.mutate(hit.id)}
                  >
                    {t("book_cancel_appointment")}
                  </Button>
                </div>
              </li>
            ))}
          </ul>
        </div>
      )}
    </section>
  );
}
