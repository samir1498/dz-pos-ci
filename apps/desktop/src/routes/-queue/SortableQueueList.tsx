// The waiting room as a list the desk drags into its own order (C6b, Samir
// 2026-09-23 19:32). dnd-kit (`@dnd-kit/core` and `@dnd-kit/sortable`, MIT)
// rather than the browser's own drag and drop: it runs on pointer events,
// which the Tauri webview on Windows passes through where it takes native
// drag events for file drops, and it moves a row from the keyboard too
// (Space to lift, arrows to move, Space to drop). A list and not the
// shared `DataTable`: a sortable row needs its own element to move, and a
// table row cannot be one without breaking the table's layout mid-drag.
//
// The list decides nothing: a drop hands the new order of ids to
// `onReorder`, and the screen writes it.

import {
  closestCenter,
  DndContext,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  type Announcements,
  type DragEndEvent,
  type UniqueIdentifier,
} from "@dnd-kit/core";
import {
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import type { QueueEntryDto } from "@dzpos/shared";
import { GripVertical } from "lucide-react";
import type { ReactNode } from "react";

import { Icon } from "@/components/Icon";
import { Button } from "@/components/ui/button";
import { useTranslation } from "@/i18n";

import { moveEntry } from "./order";

export function SortableQueueList({
  entries,
  label,
  onReorder,
  renderRow,
}: {
  entries: readonly QueueEntryDto[];
  /** Read out as the list's name. */
  label: string;
  /** The whole day's ids in their new order, after a drop that moved one. */
  onReorder: (ids: readonly string[]) => void;
  /** Everything a row shows after its handle. */
  renderRow: (entry: QueueEntryDto) => ReactNode;
}) {
  const { t } = useTranslation();
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 4 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );
  const ids = entries.map((entry) => entry.id);

  const nameOf = (id: UniqueIdentifier) => {
    const entry = entries.find((e) => e.id === String(id));
    return entry === undefined ? "" : `${entry.first_name} ${entry.last_name}`;
  };
  const placeOf = (id: UniqueIdentifier) => String(ids.indexOf(String(id)) + 1);
  const say = (text: string, id: UniqueIdentifier, place?: UniqueIdentifier) =>
    text.replace("{name}", nameOf(id)).replace("{place}", place === undefined ? "" : placeOf(place));
  const announcements: Announcements = {
    onDragStart: ({ active }) => say(t("queue_drag_picked"), active.id),
    onDragOver: ({ active, over }) =>
      over === null ? undefined : say(t("queue_drag_over"), active.id, over.id),
    onDragEnd: ({ active, over }) =>
      over === null
        ? say(t("queue_drag_cancelled"), active.id)
        : say(t("queue_drag_dropped"), active.id, over.id),
    onDragCancel: ({ active }) => say(t("queue_drag_cancelled"), active.id),
  };

  const handleDragEnd = ({ active, over }: DragEndEvent) => {
    if (over === null) return;
    const next = moveEntry(ids, String(active.id), String(over.id));
    if (next !== ids) onReorder(next);
  };

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCenter}
      onDragEnd={handleDragEnd}
      accessibility={{
        announcements,
        screenReaderInstructions: { draggable: t("queue_drag_instructions") },
      }}
    >
      <SortableContext items={ids} strategy={verticalListSortingStrategy}>
        <ol aria-label={label} className="flex flex-col gap-1" data-testid="queue-list">
          {entries.map((entry) => (
            <SortableRow key={entry.id} entry={entry}>
              {renderRow(entry)}
            </SortableRow>
          ))}
        </ol>
      </SortableContext>
    </DndContext>
  );
}

function SortableRow({ entry, children }: { entry: QueueEntryDto; children: ReactNode }) {
  const { t } = useTranslation();
  const { attributes, listeners, setNodeRef, setActivatorNodeRef, transform, transition, isDragging } =
    useSortable({ id: entry.id });
  return (
    <li
      ref={setNodeRef}
      data-testid={`queue-row-${entry.id}`}
      style={{ transform: CSS.Transform.toString(transform), transition }}
      className={`flex items-center gap-3 rounded-md border border-border bg-background px-2 py-1.5 ${
        isDragging ? "relative z-10 shadow-md" : ""
      }`}
    >
      <Button
        ref={setActivatorNodeRef}
        type="button"
        variant="ghost"
        size="sm"
        className="cursor-grab touch-none"
        data-testid={`queue-drag-${entry.id}`}
        aria-label={t("queue_drag_handle").replace("{name}", `${entry.first_name} ${entry.last_name}`)}
        {...attributes}
        {...listeners}
      >
        <Icon as={GripVertical} size={18} />
      </Button>
      <span dir="ltr" className="w-6 text-end font-numeric tabular-nums text-muted-foreground">
        {entry.position}
      </span>
      {children}
    </li>
  );
}
