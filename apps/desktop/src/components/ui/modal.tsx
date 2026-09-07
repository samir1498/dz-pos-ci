import { useEffect, useRef } from "react";

export function Modal({
  open, title, children, onClose, width = 420,
}: {
  open: boolean;
  title: string;
  children: React.ReactNode;
  onClose: () => void;
  width?: number;
}) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const handler = (e: KeyboardEvent) => { if (e.key === "Escape") onClose(); };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-[100] flex items-start justify-center pt-[10vh]"
      onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}
    >
      <div className="fixed inset-0 bg-black/50" />
      <div
        ref={ref}
        className="relative rounded-lg animate-in shadow-2xl"
        style={{
          width,
          maxWidth: "90vw",
          maxHeight: "80vh",
          background: "var(--color-surface)",
          border: "1px solid var(--color-border)",
        }}
      >
        <div
          className="flex items-center justify-between px-5 py-4"
          style={{ borderBottom: "1px solid var(--color-border)" }}
        >
          <h2 className="text-[14px] font-semibold" style={{ color: "var(--color-text)" }}>
            {title}
          </h2>
          <button
            onClick={onClose}
            className="w-6 h-6 flex items-center justify-center rounded-[4px] cursor-pointer
              hover:bg-raised text-text-sec hover:text-text transition-colors"
          >
            ✕
          </button>
        </div>
        <div className="p-5 overflow-y-auto" style={{ maxHeight: "calc(80vh - 56px)" }}>
          {children}
        </div>
      </div>
    </div>
  );
}
