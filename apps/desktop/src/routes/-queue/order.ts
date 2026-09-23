// The desk's order of the waiting room (C6b): what a drop does to the list
// of entry ids, kept apart from the drag library so it is tested without a
// pointer.

/** `ids` with `activeId` taken out and put where `overId` stood: dropped on
 *  a later row it lands after it, on an earlier one before it, the way a
 *  sortable list reads. The same list back when either id is missing or
 *  the two are the same. */
export function moveEntry(
  ids: readonly string[],
  activeId: string,
  overId: string,
): readonly string[] {
  const from = ids.indexOf(activeId);
  const to = ids.indexOf(overId);
  if (from === -1 || to === -1 || from === to) return ids;
  const rest = ids.filter((id) => id !== activeId);
  return [...rest.slice(0, to), activeId, ...rest.slice(to)];
}

/** `entries` in the order `ids` gives, any entry `ids` leaves out after
 *  them in the order it had, each with its new place: what the screen
 *  shows at once while the server writes the same order. */
export function inOrder<Entry extends { readonly id: string; readonly position: number }>(
  entries: readonly Entry[],
  ids: readonly string[],
): Entry[] {
  const listed = ids.flatMap((id) => entries.filter((entry) => entry.id === id));
  const rest = entries.filter((entry) => !ids.includes(entry.id));
  return [...listed, ...rest].map((entry, index) => ({ ...entry, position: index + 1 }));
}
