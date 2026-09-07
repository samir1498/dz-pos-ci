type TagColor = "good" | "bad" | "warn" | "accent" | "text" | "text-sec";

const tagColors: Record<TagColor, { text: string; bg: string; border: string }> = {
  good:    { text: "var(--color-good)",  bg: "var(--color-good)0f",  border: "var(--color-good)33" },
  bad:     { text: "var(--color-bad)",   bg: "var(--color-bad)0f",   border: "var(--color-bad)33" },
  warn:    { text: "var(--color-warn)",  bg: "var(--color-warn)0f",  border: "var(--color-warn)33" },
  accent:  { text: "var(--color-accent)", bg: "var(--color-accent)0f", border: "var(--color-accent)33" },
  text:    { text: "var(--color-text)",  bg: "var(--color-text)0f",  border: "var(--color-text)33" },
  "text-sec": { text: "var(--color-text-sec)", bg: "var(--color-text-sec)0f", border: "var(--color-text-sec)33" },
};

export function Tag({ children, color = "accent" }: { children: React.ReactNode; color?: TagColor }) {
  const c = tagColors[color];
  return (
    <span
      className="inline-flex px-2 py-[2px] rounded-[4px] text-[10.5px] font-medium leading-[18px]"
      style={{ border: `1px solid ${c.border}`, background: c.bg, color: c.text }}
    >
      {children}
    </span>
  );
}
