// "See again in N days": the earliest slot from a date, an optional visit
// type and an optional day offset (C5b's `next-free`). Found, it closes and
// hands the start to the caller, which jumps the grid there and opens the
// booking dialog on it, preselected; not found, it says so in place.

import { useMutation, useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { FormField } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { api, visitTypesQueryKey } from "@/api";
import { useTranslation } from "@/i18n";
import { errorKey } from "@/lib/fields";

const NO_VISIT_TYPE = "none";

export function NextFreeDialog({
  open,
  from,
  onOpenChange,
  onFound,
}: {
  open: boolean;
  /** `YYYY-MM-DD`, the day the search starts from before the offset. */
  from: string;
  onOpenChange: (open: boolean) => void;
  /** The slot found (`YYYY-MM-DD HH:MM:SS`). Never called when there was no
   *  room. */
  onFound: (startsAt: string) => void;
}) {
  const { t } = useTranslation();
  const [offsetDays, setOffsetDays] = useState("0");
  const [visitTypeId, setVisitTypeId] = useState(NO_VISIT_TYPE);

  const visitTypes = useQuery({
    queryKey: visitTypesQueryKey,
    queryFn: () => api.listVisitTypes(),
    enabled: open,
  });

  const search = useMutation({
    mutationFn: () =>
      api.nextFreeSlot({
        from,
        offsetDays: Number.isFinite(Number(offsetDays)) ? Number(offsetDays) : 0,
        visitTypeId: visitTypeId === NO_VISIT_TYPE ? undefined : visitTypeId,
      }),
    onSuccess: (found) => {
      if (found.starts_at !== null) onFound(found.starts_at);
    },
  });

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) {
          setOffsetDays("0");
          setVisitTypeId(NO_VISIT_TYPE);
          search.reset();
        }
        onOpenChange(next);
      }}
    >
      <DialogContent data-testid="next-free-dialog">
        <DialogHeader>
          <DialogTitle>{t("book_next_free_title")}</DialogTitle>
        </DialogHeader>

        <FormField label={t("book_next_free_offset")} hint={t("book_next_free_offset_hint")}>
          {(parts) => (
            <Input
              {...parts}
              type="number"
              min={0}
              data-testid="next-free-offset"
              value={offsetDays}
              onChange={(event) => setOffsetDays(event.target.value)}
            />
          )}
        </FormField>

        <FormField label={t("book_visit_type")}>
          {(parts) => (
            <Select value={visitTypeId} onValueChange={setVisitTypeId}>
              <SelectTrigger id={parts.id} className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value={NO_VISIT_TYPE}>{t("book_visit_type_none")}</SelectItem>
                {(visitTypes.data?.visit_types ?? []).map((type) => (
                  <SelectItem key={type.id} value={type.id}>
                    {type.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          )}
        </FormField>

        {search.isSuccess && search.data.starts_at === null ? (
          <p className="text-sm text-muted-foreground">{t("book_next_free_none")}</p>
        ) : null}
        {search.isError ? (
          <p role="alert" className="text-sm text-fg-danger">
            {t(errorKey(search.error))}
          </p>
        ) : null}

        <DialogFooter>
          <Button
            type="button"
            data-testid="next-free-search"
            disabled={search.isPending}
            onClick={() => search.mutate()}
          >
            {t("book_next_free_search")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
