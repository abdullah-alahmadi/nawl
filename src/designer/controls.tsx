// Small form controls shared by the designer panels.
import { useEffect, useId, useRef, useState, type ReactNode } from "react";
import type { Issue } from "../api";

/** Issues at `path` or below it (e.g. "page.margins" matches "page.margins.topMm"). */
export function issuesAt(issues: Issue[], path: string): Issue[] {
  return issues.filter(
    (i) => i.path === path || i.path.startsWith(path + ".") || i.path.startsWith(path + "["),
  );
}

/** A labelled row with an optional hint and any validation messages for it. */
export function Row({
  label,
  hint,
  issues = [],
  children,
}: {
  label: string;
  hint?: string;
  issues?: Issue[];
  children: ReactNode;
}) {
  const errors = issues.filter((i) => i.severity === "error");
  const warnings = issues.filter((i) => i.severity === "warning");
  return (
    <div className={`d-row${errors.length ? " has-error" : ""}`}>
      <div className="d-label">
        {label}
        {hint && <span className="d-hint">{hint}</span>}
      </div>
      <div className="d-control">
        {children}
        {[...errors, ...warnings].map((i, n) => (
          <div key={n} className={i.severity === "error" ? "d-error" : "d-warning"}>
            {i.message}
          </div>
        ))}
      </div>
    </div>
  );
}

/** Number input that tolerates half-typed values ("1.") and shows a unit. */
export function NumberInput({
  value,
  onChange,
  unit,
  min,
  max,
  step = 1,
  width = 80,
}: {
  value: number;
  onChange: (v: number) => void;
  unit?: string;
  min?: number;
  max?: number;
  step?: number;
  width?: number;
}) {
  const [text, setText] = useState(String(value));
  // Follow outside changes (undo, reload) without fighting the user's typing.
  useEffect(() => {
    if (Number(text) !== value) setText(String(value));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [value]);
  return (
    <span className="d-number">
      <input
        type="number"
        value={text}
        min={min}
        max={max}
        step={step}
        style={{ width }}
        onChange={(e) => {
          setText(e.target.value);
          const v = Number(e.target.value);
          if (e.target.value.trim() !== "" && Number.isFinite(v)) onChange(v);
        }}
      />
      {unit && <span className="d-unit">{unit}</span>}
    </span>
  );
}

export function Toggle({ checked, onChange, label }: { checked: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <label className="d-toggle">
      <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
      {label}
    </label>
  );
}

export function Select<T extends string | number>({
  value,
  options,
  onChange,
}: {
  value: T;
  options: { value: T; label: string }[];
  onChange: (v: T) => void;
}) {
  return (
    <select
      value={String(value)}
      onChange={(e) => {
        const opt = options.find((o) => String(o.value) === e.target.value);
        if (opt) onChange(opt.value);
      }}
    >
      {options.map((o) => (
        <option key={String(o.value)} value={String(o.value)}>
          {o.label}
        </option>
      ))}
    </select>
  );
}

/** Segmented buttons for small choices like alignment. */
export function Segmented<T extends string>({
  value,
  options,
  onChange,
}: {
  value: T;
  options: { value: T; label: string; title?: string }[];
  onChange: (v: T) => void;
}) {
  return (
    <span className="d-segmented" role="radiogroup">
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          role="radio"
          aria-checked={o.value === value}
          className={o.value === value ? "active" : ""}
          title={o.title}
          onClick={() => onChange(o.value)}
        >
          {o.label}
        </button>
      ))}
    </span>
  );
}

/** Optional colour: unchecked means "default (black)". */
export function ColorInput({ value, onChange }: { value?: string; onChange: (v: string | undefined) => void }) {
  return (
    <span className="d-color">
      <Toggle checked={value !== undefined} onChange={(on) => onChange(on ? "#1F3A93" : undefined)} label="Colour" />
      {value !== undefined && <input type="color" value={value} onChange={(e) => onChange(e.target.value.toUpperCase())} />}
    </span>
  );
}

export interface PlaceholderOption {
  name: string;
  description: string;
}

/** Text input with clickable chips that insert `{placeholder}` at the cursor. */
export function PlaceholderInput({
  value,
  onChange,
  placeholders,
  multiline = false,
}: {
  value: string;
  onChange: (v: string) => void;
  placeholders: PlaceholderOption[];
  multiline?: boolean;
}) {
  const ref = useRef<HTMLInputElement & HTMLTextAreaElement>(null);
  const listId = useId();
  const insert = (name: string) => {
    const el = ref.current;
    const token = `{${name}}`;
    const start = el?.selectionStart ?? value.length;
    const end = el?.selectionEnd ?? value.length;
    onChange(value.slice(0, start) + token + value.slice(end));
    requestAnimationFrame(() => {
      el?.focus();
      el?.setSelectionRange(start + token.length, start + token.length);
    });
  };
  return (
    <div className="d-placeholder-input">
      {multiline ? (
        <textarea ref={ref} rows={2} value={value} onChange={(e) => onChange(e.target.value)} />
      ) : (
        <input ref={ref} value={value} onChange={(e) => onChange(e.target.value)} aria-describedby={listId} />
      )}
      <div className="d-chips" id={listId}>
        {placeholders.map((p) => (
          <button key={p.name} type="button" title={p.description} onClick={() => insert(p.name)}>
            {`{${p.name}}`}
          </button>
        ))}
      </div>
    </div>
  );
}
