// A button that opens a dropdown menu. Closes on a click outside, on Escape, and
// after choosing an item.
import { useEffect, useRef, useState, type ReactNode } from "react";

export function Menu({
  label,
  title,
  className = "",
  align = "left",
  up = false,
  children,
}: {
  label: ReactNode;
  title?: string;
  className?: string;
  align?: "left" | "right";
  /** Open above the button (for the status bar). */
  up?: boolean;
  /** Menu content; call `close` after an action. */
  children: (close: () => void) => ReactNode;
}) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (!root.current?.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <div className="menu-anchor" ref={root}>
      <button
        className={`${className}${open ? " active" : ""}`}
        title={title}
        aria-haspopup="menu"
        aria-expanded={open}
        onMouseDown={(e) => e.preventDefault()}
        onClick={() => setOpen((v) => !v)}
      >
        {label}
      </button>
      {open && (
        <div className={`menu menu-${align}${up ? " menu-up" : ""}`} role="menu">
          {children(() => setOpen(false))}
        </div>
      )}
    </div>
  );
}

/** A menu row: icon, label, and an optional shortcut on the right. */
export function MenuItem({
  icon,
  label,
  shortcut,
  onClick,
  disabled,
  current,
  title,
}: {
  icon?: ReactNode;
  label: ReactNode;
  shortcut?: string;
  onClick: () => void;
  disabled?: boolean;
  current?: boolean;
  title?: string;
}) {
  return (
    <button role="menuitem" className={`menu-item${current ? " current" : ""}`} onClick={onClick} disabled={disabled} title={title}>
      <span className="menu-icon">{icon}</span>
      <span className="menu-label">{label}</span>
      {shortcut && <kbd className="menu-shortcut">{shortcut}</kbd>}
    </button>
  );
}

export function MenuSeparator() {
  return <div className="menu-sep" role="separator" />;
}

export function MenuHeading({ children }: { children: ReactNode }) {
  return <div className="menu-heading">{children}</div>;
}
