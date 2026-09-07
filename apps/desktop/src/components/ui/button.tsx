interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "default" | "ghost" | "danger";
  size?: "sm" | "md";
}

const variants = {
  primary: {
    background: "var(--color-text)",
    color: "var(--color-bg)",
    border: "1px solid var(--color-text)",
  },
  default: {
    background: "transparent",
    color: "var(--color-text-sec)",
    border: "1px solid var(--color-border)",
  },
  ghost: {
    background: "transparent",
    color: "var(--color-text-sec)",
    border: "1px solid transparent",
  },
  danger: {
    background: "transparent",
    color: "var(--color-bad)",
    border: "1px solid transparent",
  },
};

export function Button({ variant = "default", size = "md", className = "", style, ...props }: ButtonProps) {
  const v = variants[variant];
  const h = size === "sm" ? 26 : 30;
  const px = size === "sm" ? 10 : 14;
  const fs = size === "sm" ? 11 : 12;
  return (
    <button
      className={`inline-flex items-center gap-1.5 rounded-[6px] font-medium cursor-pointer
        hover:opacity-85 transition-opacity disabled:opacity-40 disabled:cursor-not-allowed ${className}`}
      style={{ height: h, paddingInline: px, fontSize: fs, ...v, ...style }}
      {...props}
    />
  );
}

export function IconButton({ children, ...props }: { children: React.ReactNode } & React.ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button
      className="inline-flex items-center justify-center w-[28px] h-[28px] rounded-[4px] cursor-pointer
        hover:bg-raised transition-colors disabled:opacity-30"
      style={{
        border: "1px solid transparent",
        color: "var(--color-text-sec)",
      }}
      {...props}
    >
      {children}
    </button>
  );
}
