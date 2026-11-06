// Lets a dialog be moved by dragging its title bar. The dialog keeps at least its
// title bar inside the window, so it can always be dragged back.
import { useRef, useState, type CSSProperties, type PointerEvent as ReactPointerEvent, type RefObject } from "react";

export function useDraggable<T extends HTMLElement>(): {
  ref: RefObject<T | null>;
  /** For the dialog itself. */
  style: CSSProperties;
  /** For the title bar the dialog is dragged by. */
  handle: { onPointerDown: (e: ReactPointerEvent) => void; style: CSSProperties };
} {
  const ref = useRef<T>(null);
  const [offset, setOffset] = useState({ x: 0, y: 0 });

  const onPointerDown = (e: ReactPointerEvent) => {
    // Buttons and fields in the title bar keep working.
    if (e.button !== 0 || (e.target as HTMLElement).closest("button, input, select, textarea, a, label")) return;
    const el = ref.current;
    if (!el) return;
    e.preventDefault();
    const rect = el.getBoundingClientRect();
    const start = { x: e.clientX, y: e.clientY, ox: offset.x, oy: offset.y };
    const move = (ev: PointerEvent) => {
      let dx = ev.clientX - start.x;
      let dy = ev.clientY - start.y;
      // Keep the title bar (the top 40px) and part of the width on screen.
      const margin = 80;
      dx = Math.min(Math.max(dx, margin - rect.right), window.innerWidth - margin - rect.left);
      dy = Math.min(Math.max(dy, -rect.top), window.innerHeight - 40 - rect.top);
      setOffset({ x: start.ox + dx, y: start.oy + dy });
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      document.body.classList.remove("dragging-dialog");
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    document.body.classList.add("dragging-dialog");
  };

  return {
    ref,
    style: { transform: `translate(${offset.x}px, ${offset.y}px)` },
    handle: { onPointerDown, style: { cursor: "move", userSelect: "none", touchAction: "none" } },
  };
}
