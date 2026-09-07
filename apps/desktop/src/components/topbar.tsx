interface TopBarProps {
  title: string;
  onMenuClick: () => void;
  right?: React.ReactNode;
}

export function TopBar({ title, onMenuClick, right }: TopBarProps) {
  return (
    <header
      className="h-[52px] flex items-center justify-between px-6 lg:px-7 sticky top-0 z-30"
      style={{
        borderBottom: "1px solid var(--color-border)",
        background: "rgba(10,10,12,0.85)",
        backdropFilter: "blur(12px)",
      }}
    >
      <div className="flex items-center gap-3">
        <button
          className="lg:hidden text-text-sec hover:text-text text-lg p-1 -ml-1"
          onClick={onMenuClick}
        >
          ≡
        </button>
        <span
          className="text-[13px] font-medium tracking-tight"
          style={{ color: "var(--color-text)" }}
        >
          {title}
        </span>
      </div>
      {right && <div className="flex items-center gap-2">{right}</div>}
    </header>
  );
}
