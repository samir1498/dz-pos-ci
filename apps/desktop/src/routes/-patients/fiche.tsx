// The patient's file: identity plus notes, no debt shape at all (C3 of
// `the-first-clinic-module-patients-queue-appointments`: a cabinet's file is
// identity and notes, and the money columns a customer's fiche carries
// would not fit here).
//
// Notes are doctor-only (Samir's ruling, 2026-09-23; C3b). The field is
// rendered only for a session holding `view_patient_notes`, and a session
// without it always writes `null` there — never the empty string, which the
// server reads as "clear the notes" and refuses from a caller who may not
// even see them (403, `crates/clinic/src/services/patients.rs`). A session
// that cannot see the field can therefore never send a value able to change
// it, whatever was left in a stale form.

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useForm } from "@tanstack/react-form";
import type { PatientDto, PatientWriteDto, SexDto } from "@dzpos/shared";
import { useState } from "react";

import { ChoiceRow } from "@/components/ChoiceRow";
import { FormField } from "@/components/FormField";
import { DateField } from "@/components/ui/date-field";
import { Input } from "@/components/ui/input";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { patientsQueryKey, api } from "@/api";
import { useTranslation, type Key } from "@/i18n";
import { cleared, errorKey } from "@/lib/fields";
import { useHasPermission } from "@/lib/session";

import { SEXES, SEX_KEY } from "./parts";

type SexChoice = SexDto | "unspecified";

const SEX_CHOICES: readonly SexChoice[] = ["unspecified", ...SEXES];

interface FicheValues {
  firstName: string;
  lastName: string;
  sex: SexChoice;
  dateOfBirth: string;
  phone: string;
  notes: string;
}

function blank(): FicheValues {
  return { firstName: "", lastName: "", sex: "unspecified", dateOfBirth: "", phone: "", notes: "" };
}

/** `notes` is read only when `sees` says the session may: a receptionist's
 *  `PatientDto` carries no `notes` key at all, and filling the field from
 *  `undefined` would show the empty string as "no notes today" when the
 *  truth is "not shown to you". */
function filled(patient: PatientDto, sees: boolean): FicheValues {
  return {
    firstName: patient.first_name,
    lastName: patient.last_name,
    sex: patient.sex ?? "unspecified",
    dateOfBirth: patient.date_of_birth ?? "",
    phone: patient.phone ?? "",
    notes: sees ? (patient.notes ?? "") : "",
  };
}

export function PatientFicheSheet({
  open,
  initial,
  onOpenChange,
}: {
  open: boolean;
  /** `null` is the blank fiche. */
  initial: PatientDto | null;
  onOpenChange: (open: boolean) => void;
}) {
  const { t, dir } = useTranslation();
  const seesNotes = useHasPermission("view_patient_notes");
  const name = initial === null ? null : `${initial.first_name} ${initial.last_name}`.trim();
  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent
        side={dir === "rtl" ? "left" : "right"}
        className="w-full gap-0 sm:max-w-xl"
        data-testid="patient-fiche"
      >
        <SheetHeader className="border-b border-border">
          <SheetTitle>{initial === null ? t("patients_new") : name}</SheetTitle>
          <SheetDescription>{t("patients_fiche_hint")}</SheetDescription>
        </SheetHeader>
        <PatientForm
          // The sheet stays mounted under the lock screen (`__root.tsx`
          // marks the app `inert`, never unmounted, while locked), so a
          // receptionist's open fiche can still be on screen when the
          // doctor unlocks. `seesNotes` rides the key alongside the
          // patient's id: a permission change remounts the form exactly
          // like a different patient would, reseeding `notes` from
          // `filled()` under the *new* session rather than leaving the
          // stale, notes-blanked defaults from the old one sitting in
          // state ready to be saved over the real notes.
          key={`${initial === null ? "new" : initial.id}:${seesNotes ? "sees" : "blind"}`}
          initial={initial}
          seesNotes={seesNotes}
          onDone={() => onOpenChange(false)}
        />
      </SheetContent>
    </Sheet>
  );
}

function PatientForm({
  initial,
  seesNotes,
  onDone,
}: {
  initial: PatientDto | null;
  seesNotes: boolean;
  onDone: () => void;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<Key | null>(null);

  const save = useMutation({
    mutationFn: (input: PatientWriteDto) =>
      initial === null ? api.createPatient(input) : api.updatePatient(initial.id, input),
    onSuccess: async () => {
      setServerError(null);
      await queryClient.invalidateQueries({ queryKey: patientsQueryKey });
      onDone();
    },
    onError: (error: unknown) => setServerError(errorKey(error)),
  });

  const form = useForm({
    defaultValues: initial === null ? blank() : filled(initial, seesNotes),
    onSubmit: async ({ value }) => {
      // The rejection is swallowed on purpose: onError has already turned
      // the server's code into a translated message on the form.
      await save
        .mutateAsync({
          first_name: value.firstName,
          last_name: value.lastName,
          sex: value.sex === "unspecified" ? null : value.sex,
          date_of_birth: value.dateOfBirth === "" ? null : value.dateOfBirth,
          phone: cleared(value.phone),
          // Never the empty string: a session that cannot see the field
          // must never be able to clear it, and `cleared("")` would answer
          // `null` for the right reason but for the wrong one, so the
          // permission is asked here rather than folded into that helper.
          notes: seesNotes ? cleared(value.notes) : null,
        })
        .catch(() => undefined);
    },
  });

  return (
    <form
      noValidate
      className="flex min-h-0 flex-1 flex-col"
      onSubmit={(event) => {
        event.preventDefault();
        void form.handleSubmit();
      }}
    >
      <div className="grid min-h-0 flex-1 gap-4 overflow-y-auto p-4 sm:grid-cols-2">
        <form.Field
          name="firstName"
          validators={{
            onSubmit: ({ value }) => (value.trim() === "" ? "error_name_required" : undefined),
          }}
        >
          {(field) => (
            <FormField
              label={t("field_first_name")}
              error={field.state.meta.errors.length > 0 ? t("error_name_required") : undefined}
            >
              {(parts) => (
                <Input
                  {...parts}
                  data-testid="patient-first-name"
                  value={field.state.value}
                  onBlur={field.handleBlur}
                  onChange={(event) => field.handleChange(event.target.value)}
                />
              )}
            </FormField>
          )}
        </form.Field>

        <form.Field
          name="lastName"
          validators={{
            onSubmit: ({ value }) => (value.trim() === "" ? "error_name_required" : undefined),
          }}
        >
          {(field) => (
            <FormField
              label={t("field_last_name")}
              error={field.state.meta.errors.length > 0 ? t("error_name_required") : undefined}
            >
              {(parts) => (
                <Input
                  {...parts}
                  data-testid="patient-last-name"
                  value={field.state.value}
                  onBlur={field.handleBlur}
                  onChange={(event) => field.handleChange(event.target.value)}
                />
              )}
            </FormField>
          )}
        </form.Field>

        <form.Field name="sex">
          {(field) => (
            <ChoiceRow
              label={t("field_sex")}
              value={field.state.value}
              onChange={field.handleChange}
              options={SEX_CHOICES.map((choice) => ({
                value: choice,
                label: t(choice === "unspecified" ? "sex_unspecified" : SEX_KEY[choice]),
              }))}
            />
          )}
        </form.Field>

        <form.Field name="dateOfBirth">
          {(field) => (
            <FormField label={t("field_date_of_birth")}>
              {(parts) => (
                <DateField
                  {...parts}
                  data-testid="patient-date-of-birth"
                  value={field.state.value}
                  onChange={field.handleChange}
                />
              )}
            </FormField>
          )}
        </form.Field>

        <form.Field name="phone">
          {(field) => (
            <FormField label={t("field_phone")}>
              {(parts) => (
                <Input
                  {...parts}
                  dir="ltr"
                  className="font-numeric"
                  data-testid="patient-phone"
                  value={field.state.value}
                  onChange={(event) => field.handleChange(event.target.value)}
                />
              )}
            </FormField>
          )}
        </form.Field>

        {seesNotes ? (
          <form.Field name="notes">
            {(field) => (
              <FormField label={t("field_notes")} className="sm:col-span-2">
                {(parts) => (
                  <Textarea
                    {...parts}
                    rows={4}
                    data-testid="patient-notes"
                    value={field.state.value}
                    onChange={(event) => field.handleChange(event.target.value)}
                  />
                )}
              </FormField>
            )}
          </form.Field>
        ) : (
          <p className="text-sm text-muted-foreground sm:col-span-2">
            {t("patients_notes_hidden")}
          </p>
        )}

        {serverError === null ? null : (
          <p role="alert" className="text-sm text-fg-danger sm:col-span-2">
            {t(serverError)}
          </p>
        )}
      </div>

      <SheetFooter className="flex-row justify-end border-t border-border">
        <Button type="button" variant="ghost" onClick={onDone}>
          {t("action_cancel")}
        </Button>
        <Button type="submit" disabled={save.isPending}>
          {save.isPending ? t("action_saving") : t("action_save")}
        </Button>
      </SheetFooter>
    </form>
  );
}
