// The question a shop answers before something it cannot take back.
//
// A browser `confirm()` asks that question in a box Windows draws. The
// sentence inside is ours; the two buttons and the box are not, so they come
// out in the language the machine is in and they do not mirror on an Arabic
// screen. `suppliers.tsx` wrote that down before this file existed and
// answered it with a dialog of its own. This is the same answer, in one
// place, for the questions that need nothing typed into them.
//
// The dialog is driven by state and not by a trigger: the caller already
// knows what is being confirmed, and a `DialogTrigger` would want the pointer
// capture jsdom does not have.

import type { ReactNode } from "react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useTranslation, type Key } from "@/i18n";

export function ConfirmDialog({
  open,
  onCancel,
  onConfirm,
  title,
  question,
  confirm,
  pending = false,
  destructive = false,
  children,
  "data-testid": testId,
}: {
  open: boolean;
  onCancel: () => void;
  onConfirm: () => void;
  /** The heading, and what the shop pressed to get here. */
  title: Key;
  /** The sentence that says what happens and what it costs. */
  question: Key;
  /** The wording on the button that goes through with it. */
  confirm: Key;
  pending?: boolean;
  destructive?: boolean;
  /** Anything the question needs shown with it: an amount, a name, a date. */
  children?: ReactNode;
  "data-testid": string;
}) {
  const { t } = useTranslation();

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) onCancel();
      }}
    >
      <DialogContent data-testid={testId}>
        <DialogHeader>
          <DialogTitle>{t(title)}</DialogTitle>
          <DialogDescription>{t(question)}</DialogDescription>
        </DialogHeader>
        {children}
        <DialogFooter>
          <Button
            type="button"
            variant="ghost"
            data-testid={`${testId}-cancel`}
            onClick={onCancel}
          >
            {t("action_cancel")}
          </Button>
          <Button
            type="button"
            variant={destructive ? "destructive" : "default"}
            disabled={pending}
            data-testid={`${testId}-confirm`}
            onClick={onConfirm}
          >
            {t(confirm)}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
