// The shop's staff (M4 T8). Listing, adding a fiche, resetting a PIN, and
// switching a fiche off or back on: everything on this screen is behind
// `crate::gates`' `ManageUsers`, which only the owner holds (a manager is
// refused the same as a cashier), so a fiche that types this address by
// hand without that gate gets the translated 403 (`error_forbidden`) in
// place of a table. The last-owner and self refusals are never offered as
// a choice here: the server enforces them on the row
// (`services::users::deactivate`) and this screen only ever shows the
// server's own refusal back, translated. A PIN reset never shows the PIN
// it replaces, because the server never sends one back to show
// (`UserDto` carries `has_pin`, never a hash).
//
// One of the settings rooms: it renders inside `/settings`'s layout, beside
// the rail that lists the rooms, which is why it heads itself at `h3` and
// carries no link back — the rail is the way back and it never left.

import { ApiError } from "@dzpos/shared";
import type { NewUserDto, RoleDto, UserDto } from "@dzpos/shared";
import { useForm } from "@tanstack/react-form";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { KeyRound, UserPlus, Users as UsersIcon } from "lucide-react";
import { useState } from "react";

import { api, usersQueryKey } from "@/api";
import { DataTable, type Column } from "@/components/DataTable";
import { EmptyState } from "@/components/EmptyState";
import { FormField } from "@/components/FormField";
import { Icon } from "@/components/Icon";
import { PageHeader } from "@/components/PageHeader";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { isKey, useTranslation, type Key } from "@/i18n";
import { errorKey } from "@/lib/fields";
import { DEFAULT_ROLE, ROLE_LABEL, ROLES } from "@/lib/roles";

export const Route = createFileRoute("/settings/users")({ component: UsersScreen });

/** The staff screen shows a role the same way the topbar does. */
const ROLE_KEY = ROLE_LABEL;

/** A role is never guessed: an unknown option blocks the submit, the same
 *  rule `settings.tsx` reads a régime by. */
function toRole(value: string): RoleDto | undefined {
  return ROLES.find((r) => r === value);
}



/** A field validator answers with a translation key, never a sentence, and
 *  `FormField` wants the sentence. This screen keeps its own copy rather
 *  than importing the customers screen's private one
 *  (`lib/fields.tsx`'s own note on why each screen still carries its own). */
function useFieldError(): (messages: readonly unknown[]) => string | undefined {
  const { t } = useTranslation();
  return (messages) => {
    const key = messages.find((message): message is string => typeof message === "string");
    if (key === undefined) return undefined;
    return t(isKey(key) ? key : "error_unknown");
  };
}

export function UsersScreen() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const users = useQuery({ queryKey: usersQueryKey, queryFn: () => api.listUsers() });
  const [adding, setAdding] = useState(false);
  const [resetting, setResetting] = useState<UserDto | null>(null);
  const [rowError, setRowError] = useState<Key | null>(null);

  const toggle = useMutation({
    mutationFn: (row: UserDto) =>
      row.active ? api.deactivateUser(row.id) : api.reactivateUser(row.id),
    onSuccess: async () => {
      setRowError(null);
      await queryClient.invalidateQueries({ queryKey: usersQueryKey });
    },
    onError: (error: unknown) => {
      setRowError(errorKey(error));
    },
  });

  const columns: readonly Column<UserDto>[] = [
    { id: "name", header: t("col_name"), cell: (row) => row.name },
    { id: "role", header: t("col_role"), cell: (row) => t(ROLE_KEY[row.role]) },
    {
      id: "status",
      header: t("col_status"),
      cell: (row) => (
        <div className="flex flex-wrap gap-1">
          {row.active ? null : <Badge variant="outline">{t("users_inactive")}</Badge>}
          {row.has_pin ? null : <Badge variant="outline">{t("users_no_pin")}</Badge>}
        </div>
      ),
    },
  ];

  return (
    <section className="flex flex-col gap-4">
      <PageHeader
        title={t("users_title")}
        level={3}
        description={t("users_hint")}
        actions={
          <>
            <Button
              onClick={() => {
                setRowError(null);
                setAdding(true);
              }}
            >
              <Icon as={UserPlus} size={18} />
              {t("users_add")}
            </Button>
          </>
        }
      />

      {users.isPending ? (
        <div className="flex flex-col gap-3" aria-busy="true">
          <p className="text-sm text-muted-foreground">{t("users_loading")}</p>
          <Skeleton className="h-40 w-full" />
        </div>
      ) : null}

      {users.isError ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(users.error))}
        </p>
      ) : null}

      {users.isSuccess ? (
        <>
          {rowError === null ? null : (
            <p role="alert" className="text-sm text-fg-danger">
              {t(rowError)}
            </p>
          )}
          <DataTable
            data-testid="users-table"
            caption={t("users_title")}
            columns={columns}
            rows={users.data}
            rowKey={(row) => row.id}
            rowTestId={(row) => `user-row-${row.id}`}
            empty={
              <EmptyState
                icon={UsersIcon}
                title={t("users_empty")}
                description={t("users_empty_hint")}
              />
            }
            actions={(row) => (
              <>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => {
                    setRowError(null);
                    setResetting(row);
                  }}
                >
                  <Icon as={KeyRound} size={18} />
                  {t("action_reset_pin")}
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  disabled={toggle.isPending && toggle.variables?.id === row.id}
                  onClick={() => {
                    setRowError(null);
                    toggle.mutate(row);
                  }}
                >
                  {row.active ? t("action_deactivate") : t("action_reactivate")}
                </Button>
              </>
            )}
          />
        </>
      ) : null}

      <AddUserDialog open={adding} onOpenChange={setAdding} />
      <ResetPinDialog
        user={resetting}
        onOpenChange={(open) => {
          if (!open) setResetting(null);
        }}
      />
    </section>
  );
}

interface NewUserValues {
  name: string;
  role: string;
}

function AddUserDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { t } = useTranslation();
  const said = useFieldError();
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<Key | null>(null);

  const create = useMutation({
    mutationFn: (input: NewUserDto) => api.createUser(input),
    onSuccess: async () => {
      setServerError(null);
      onOpenChange(false);
      await queryClient.invalidateQueries({ queryKey: usersQueryKey });
    },
    onError: (error: unknown) => {
      setServerError(errorKey(error));
    },
  });

  const blank: NewUserValues = { name: "", role: DEFAULT_ROLE };
  const form = useForm({
    defaultValues: blank,
    onSubmit: async ({ value }) => {
      const name = value.name.trim();
      if (name === "") return;
      const role = toRole(value.role);
      if (role === undefined) {
        setServerError("error_validation");
        return;
      }
      const written = await create
        .mutateAsync({ name, role })
        .then(() => true)
        .catch(() => false);
      if (!written) return;
      form.reset();
    },
  });

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) {
          setServerError(null);
          form.reset();
        }
        onOpenChange(next);
      }}
    >
      <DialogContent data-testid="add-user-dialog">
        <DialogHeader>
          <DialogTitle>{t("users_add")}</DialogTitle>
        </DialogHeader>
        <form
          noValidate
          className="flex flex-col gap-4"
          onSubmit={(event) => {
            event.preventDefault();
            void form.handleSubmit();
          }}
        >
          <form.Field
            name="name"
            validators={{
              onSubmit: ({ value }) => (value.trim() === "" ? "error_name_required" : undefined),
            }}
          >
            {(field) => (
              <FormField label={t("field_name")} error={said(field.state.meta.errors)}>
                {(parts) => (
                  <Input
                    {...parts}
                    value={field.state.value}
                    onBlur={field.handleBlur}
                    onChange={(event) => field.handleChange(event.target.value)}
                  />
                )}
              </FormField>
            )}
          </form.Field>

          <form.Field name="role">
            {(field) => (
              <FormField label={t("col_role")}>
                {(parts) => (
                  <Select value={field.state.value} onValueChange={(next) => field.handleChange(next)}>
                    <SelectTrigger
                      id={parts.id}
                      aria-describedby={parts["aria-describedby"]}
                      className="w-full"
                    >
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {ROLES.map((r) => (
                        <SelectItem key={r} value={r}>
                          {t(ROLE_KEY[r])}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                )}
              </FormField>
            )}
          </form.Field>

          {serverError === null ? null : (
            <p role="alert" className="text-sm text-fg-danger">
              {t(serverError)}
            </p>
          )}

          <DialogFooter>
            <Button type="button" variant="ghost" onClick={() => onOpenChange(false)}>
              {t("action_cancel")}
            </Button>
            <Button type="submit" disabled={create.isPending}>
              {create.isPending ? t("action_saving") : t("users_add")}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

interface PinValues {
  pin: string;
}

/** Gives a fiche its first PIN or resets a forgotten one; the server does
 *  not tell the two apart (`SetPinDto`'s own doc) and neither does this
 *  dialog: it opens from the same "Reset PIN" row action whether the fiche
 *  already had one or not. */
function ResetPinDialog({
  user,
  onOpenChange,
}: {
  user: UserDto | null;
  onOpenChange: (open: boolean) => void;
}) {
  const { t } = useTranslation();
  const said = useFieldError();
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<Key | null>(null);

  const setPin = useMutation({
    mutationFn: (input: { id: number; pin: string }) =>
      api.setUserPin(input.id, { pin: input.pin }),
    onSuccess: async () => {
      setServerError(null);
      onOpenChange(false);
      await queryClient.invalidateQueries({ queryKey: usersQueryKey });
    },
    onError: (error: unknown) => {
      setServerError(errorKey(error));
    },
  });

  const blank: PinValues = { pin: "" };
  const form = useForm({
    defaultValues: blank,
    onSubmit: async ({ value }) => {
      if (user === null) return;
      const written = await setPin
        .mutateAsync({ id: user.id, pin: value.pin })
        .then(() => true)
        .catch(() => false);
      if (!written) return;
      form.reset();
    },
  });

  return (
    <Dialog
      open={user !== null}
      onOpenChange={(next) => {
        if (!next) {
          setServerError(null);
          form.reset();
        }
        onOpenChange(next);
      }}
    >
      <DialogContent data-testid="reset-pin-dialog">
        <DialogHeader>
          <DialogTitle>{t("users_reset_pin_title")}</DialogTitle>
          <DialogDescription>
            {user === null ? "" : `${user.name} · ${t("users_reset_pin_hint")}`}
          </DialogDescription>
        </DialogHeader>
        <form
          noValidate
          className="flex flex-col gap-4"
          onSubmit={(event) => {
            event.preventDefault();
            void form.handleSubmit();
          }}
        >
          <form.Field
            name="pin"
            validators={{
              onSubmit: ({ value }) => (value.trim() === "" ? "error_validation" : undefined),
            }}
          >
            {(field) => (
              <FormField label={t("field_pin")} error={said(field.state.meta.errors)}>
                {(parts) => (
                  // Never pre-filled: there is no old PIN to show
                  // (`UserDto` carries `has_pin`, never a hash), so the box
                  // opens blank whether this is the fiche's first PIN or a
                  // reset of a forgotten one.
                  <Input
                    {...parts}
                    type="password"
                    inputMode="numeric"
                    dir="ltr"
                    className="font-numeric"
                    value={field.state.value}
                    onChange={(event) => field.handleChange(event.target.value)}
                  />
                )}
              </FormField>
            )}
          </form.Field>

          {serverError === null ? null : (
            <p role="alert" className="text-sm text-fg-danger">
              {t(serverError)}
            </p>
          )}

          <DialogFooter>
            <Button type="button" variant="ghost" onClick={() => onOpenChange(false)}>
              {t("action_cancel")}
            </Button>
            <Button type="submit" disabled={setPin.isPending}>
              {setPin.isPending ? t("action_saving") : t("action_reset_pin")}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
