// Title page designer: a page-sized canvas where items are placed by dragging.
//
// Geometry matches the generated TikZ exactly: each item's (xMm, yMm) is its anchor
// point measured from the paper's top-left corner, text boxes are `widthMm` wide with
// no padding, and line height is 1.2 × the font size × the template's line spacing.
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import * as api from "../api";
import type { Anchor } from "../bindings/Anchor";
import type { FontChoice } from "../bindings/FontChoice";
import type { TitleItem } from "../bindings/TitleItem";
import { NumberInput, PlaceholderInput, Row, Toggle, issuesAt } from "./controls";
import { fieldPlaceholders, type SectionProps } from "./common";
import { TextStyleControls } from "./StyleSections";

const MM_PER_PT = 25.4 / 72.27; // TeX points
const SNAP_MM = 2; // distance at which items snap to the page's centre line

const ANCHORS: Anchor[][] = [
  ["topLeft", "top", "topRight"],
  ["left", "center", "right"],
  ["bottomLeft", "bottom", "bottomRight"],
];

/** Fraction of the item's box to shift left/up so its anchor lands on (x, y). */
function anchorShift(a: Anchor): [number, number] {
  const h = a.endsWith("Left") || a === "left" ? 0 : a.endsWith("Right") || a === "right" ? 1 : 0.5;
  const v = a.startsWith("top") ? 0 : a.startsWith("bottom") ? 1 : 0.5;
  return [h, v];
}

/** Line-spacing factor LaTeX's setspace package applies (it affects the title page too). */
function lineStretch(t: SectionProps["t"]): number {
  const size = t.fonts.baseSizePt;
  switch (t.page.lineSpacing.kind) {
    case "single":
      return 1;
    case "oneHalf":
      return size === 10 ? 1.25 : size === 11 ? 1.213 : 1.241;
    case "double":
      return size === 10 ? 1.667 : size === 11 ? 1.618 : 1.655;
    case "custom":
      return t.page.lineSpacing.stretch;
  }
}

/** CSS stack approximating the template's main font. */
function cssFont(f: FontChoice): string {
  if (f.kind === "system") return `"${f.family}", serif`;
  return {
    latinModern: '"Latin Modern Roman", "CMU Serif", serif',
    termes: '"TeX Gyre Termes", "Times New Roman", Times, serif',
    pagella: '"TeX Gyre Pagella", Palatino, "Palatino Linotype", serif',
    bonum: '"TeX Gyre Bonum", "Bookman Old Style", serif',
    schola: '"TeX Gyre Schola", "Century Schoolbook", serif',
    heros: '"TeX Gyre Heros", Helvetica, Arial, sans-serif',
    adventor: '"TeX Gyre Adventor", "Avant Garde", sans-serif',
    cursor: '"TeX Gyre Cursor", "Courier New", monospace',
  }[f.font];
}

/** Text as it will print: placeholders filled from the document, or shown as {name}. */
function resolveText(text: string, metadata: api.Metadata): string {
  return text
    .replace(/\{\{/g, "\u0000")
    .replace(/\}\}/g, "\u0001")
    .replace(/\{([a-z][a-z0-9_]*)\}/g, (whole, name: string) => {
      const v = metadata[name];
      // Lists of strings or of authors: one entry per line.
      const lines = v === undefined ? [] : Array.isArray(v) ? v.map((x) => (typeof x === "string" ? x : x.name)) : [v];
      const shown = lines.filter((s) => s.trim());
      return shown.length ? shown.join("\n") : whole;
    })
    .replace(/\u0000/g, "{")
    .replace(/\u0001/g, "}");
}

/** Loads template images as object URLs (PDF images can't be shown; they get a box). */
function useAssetUrls(assets: string[], templatePath: string | null) {
  const [urls, setUrls] = useState<Record<string, string>>({});
  const key = assets.join("|") + "@" + templatePath;
  useEffect(() => {
    let cancelled = false;
    const made: string[] = [];
    Promise.all(
      assets
        .filter((a) => !a.toLowerCase().endsWith(".pdf"))
        .map(async (a) => {
          try {
            const bytes = await api.readTemplateAsset(a);
            const url = URL.createObjectURL(new Blob([bytes]));
            made.push(url);
            return [a, url] as const;
          } catch {
            return [a, ""] as const;
          }
        }),
    ).then((pairs) => {
      if (!cancelled) setUrls(Object.fromEntries(pairs));
    });
    return () => {
      cancelled = true;
      made.forEach((u) => URL.revokeObjectURL(u));
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  return urls;
}

interface Drag {
  index: number;
  mode: "move" | "resize";
  startX: number;
  startY: number;
  orig: TitleItem;
}

export function TitlePageCanvas({
  t,
  edit,
  issues,
  metadata,
  templatePath,
  canImport,
}: SectionProps & { metadata: api.Metadata; templatePath: string | null; canImport: boolean }) {
  const [selected, setSelected] = useState<number | null>(null);
  const [guide, setGuide] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const wrap = useRef<HTMLDivElement>(null);
  const pageRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(480);
  const drag = useRef<Drag | null>(null);
  // Each item's rendered size in mm (text height depends on the font), for anchor changes.
  const sizes = useRef(new Map<number, { w: number; h: number }>());

  const paper = t.page.paper;
  const [pageW, pageH] = {
    a4: [210, 297],
    a5: [148, 210],
    letter: [215.9, 279.4],
    legal: [215.9, 355.6],
    custom: paper.kind === "custom" ? [paper.widthMm, paper.heightMm] : [210, 297],
  }[paper.kind];
  const scale = Math.min(width, 560) / pageW; // px per mm
  const items = t.titlePage.items;
  const item = selected !== null ? items[selected] : undefined;

  useLayoutEffect(() => {
    const el = wrap.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setWidth(el.clientWidth));
    ro.observe(el);
    setWidth(el.clientWidth);
    return () => ro.disconnect();
  }, []);

  // Keep the selection valid when items are removed elsewhere (undo, reload).
  useEffect(() => {
    if (selected !== null && selected >= items.length) setSelected(null);
  }, [items.length, selected]);

  const assets = useMemo(
    () => items.flatMap((i) => (i.content.type === "image" ? [i.content.asset] : [])),
    [items],
  );
  const assetUrls = useAssetUrls(assets, templatePath);

  const setItem = (index: number, fn: (it: TitleItem) => void) => edit((d) => fn(d.titlePage.items[index]));

  // ---- dragging (pointer events work for mouse, pen and touch)
  const onPointerDown = (e: React.PointerEvent, index: number, mode: Drag["mode"]) => {
    e.preventDefault(); // no text selection while dragging…
    e.stopPropagation();
    pageRef.current?.focus({ preventScroll: true }); // …but keep keyboard nudging working
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    setSelected(index);
    drag.current = { index, mode, startX: e.clientX, startY: e.clientY, orig: structuredClone(items[index]) };
  };

  const onPointerMove = (e: React.PointerEvent) => {
    const d = drag.current;
    if (!d) return;
    const dx = (e.clientX - d.startX) / scale;
    const dy = (e.clientY - d.startY) / scale;
    const free = e.altKey; // hold Alt for 0.1 mm steps and no snapping
    const round = (v: number) => (free ? Math.round(v * 10) / 10 : Math.round(v));
    if (d.mode === "move") {
      let x = round(d.orig.xMm + dx);
      const y = round(d.orig.yMm + dy);
      const [h] = anchorShift(d.orig.anchor);
      const snaps = h === 0.5 && !free && Math.abs(x - pageW / 2) < SNAP_MM;
      if (snaps) x = Math.round(pageW / 2 * 10) / 10;
      setGuide(snaps);
      setItem(d.index, (it) => {
        it.xMm = x;
        it.yMm = y;
      });
    } else {
      // Resize from the right edge; a centred box grows on both sides.
      const [h] = anchorShift(d.orig.anchor);
      const factor = h === 0.5 ? 2 : h === 1 ? -1 : 1;
      setItem(d.index, (it) => {
        const c = it.content;
        const o = d.orig.content;
        if (c.type === "text" && o.type === "text") c.widthMm = Math.max(10, round((o.widthMm ?? 60) + dx * factor));
        if (c.type === "image" && o.type === "image") c.widthMm = Math.max(5, round(o.widthMm + dx * factor));
        if (c.type === "rule" && o.type === "rule") c.lengthMm = Math.max(5, round(o.lengthMm + dx * factor));
      });
    }
  };

  const onPointerUp = () => {
    drag.current = null;
    setGuide(false);
  };

  // ---- keyboard: arrows nudge (Shift = 10 mm), Delete removes
  const onKeyDown = (e: React.KeyboardEvent) => {
    if (selected === null) return;
    const step = e.shiftKey ? 10 : e.altKey ? 0.1 : 1;
    const moves: Record<string, [number, number]> = {
      ArrowLeft: [-step, 0],
      ArrowRight: [step, 0],
      ArrowUp: [0, -step],
      ArrowDown: [0, step],
    };
    if (moves[e.key]) {
      e.preventDefault();
      const [mx, my] = moves[e.key];
      setItem(selected, (it) => {
        it.xMm = Math.round((it.xMm + mx) * 10) / 10;
        it.yMm = Math.round((it.yMm + my) * 10) / 10;
      });
    } else if (e.key === "Delete" || e.key === "Backspace") {
      e.preventDefault();
      remove(selected);
    } else if (e.key === "Escape") {
      setSelected(null);
    }
  };

  // ---- item actions
  const add = (it: TitleItem) => {
    edit((d) => void d.titlePage.items.push(it));
    setSelected(items.length);
  };
  const addText = () =>
    add({
      xMm: pageW / 2,
      yMm: 150,
      anchor: "top",
      content: { type: "text", text: "New text", widthMm: 150, style: { sizePt: 14, bold: false, italic: false, smallCaps: false, uppercase: false, align: "center" } },
    });
  const addLine = () => add({ xMm: pageW / 2, yMm: 150, anchor: "top", content: { type: "rule", lengthMm: 100, thicknessPt: 0.8 } });
  const pickImage = async () => {
    const file = await open({ multiple: false, directory: false, filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "pdf"] }] });
    if (!file) return null;
    try {
      return await api.importTemplateAsset(file);
    } catch (e) {
      setNotice(api.asAppError(e).kind === "other" ? (api.asAppError(e) as { message: string }).message : String(e));
      return null;
    }
  };
  const addImage = async () => {
    setNotice(null);
    if (!canImport) {
      setNotice("Save the template first (Save as…) so its images have a folder to live in.");
      return;
    }
    const asset = await pickImage();
    if (asset) add({ xMm: pageW / 2, yMm: 20, anchor: "top", content: { type: "image", asset, widthMm: 35 } });
  };
  const remove = (index: number) => {
    edit((d) => void d.titlePage.items.splice(index, 1));
    setSelected(null);
  };
  const duplicate = (index: number) => {
    const copy = structuredClone(items[index]);
    copy.yMm += 10;
    add(copy);
  };

  const font = cssFont(t.fonts.main);

  return (
    <div className="tp">
      <div className="tp-toolbar">
        <button type="button" onClick={addText}>
          + Text
        </button>
        <button type="button" onClick={addImage} title={canImport ? "PNG, JPEG or PDF" : "Save the template first"}>
          + Image
        </button>
        <button type="button" onClick={addLine}>
          + Line
        </button>
        <span className="d-hint">Drag to move · drag the right handle to resize · arrows nudge (Shift ×10, Alt fine)</span>
      </div>
      {notice && <div className="d-warning">{notice}</div>}

      <div className="tp-layout">
        <div className="tp-canvas-wrap" ref={wrap}>
          <div
            ref={pageRef}
            className="tp-page"
            style={{ width: pageW * scale, height: pageH * scale, fontFamily: font }}
            tabIndex={0}
            role="application"
            aria-label="Title page canvas"
            onKeyDown={onKeyDown}
            onPointerMove={onPointerMove}
            onPointerUp={onPointerUp}
            onPointerCancel={onPointerUp}
            onPointerDown={() => {
              pageRef.current?.focus({ preventScroll: true });
              setSelected(null);
            }}
          >
            {guide && <div className="tp-guide" style={{ left: (pageW / 2) * scale }} />}
            {items.map((it, i) => (
              <CanvasItem
                key={i}
                item={it}
                scale={scale}
                selected={i === selected}
                hasError={issuesAt(issues, `titlePage.items[${i}]`).length > 0}
                metadata={metadata}
                stretch={lineStretch(t)}
                onMeasure={(el) => {
                  if (el) sizes.current.set(i, { w: el.offsetWidth / scale, h: el.offsetHeight / scale });
                }}
                imageUrl={it.content.type === "image" ? assetUrls[it.content.asset] : undefined}
                onPointerDown={(e, mode) => onPointerDown(e, i, mode)}
              />
            ))}
          </div>
          <div className="d-hint tp-caption">
            On-screen approximation; the PDF preview is exact. Placeholders show this document’s values.
          </div>
        </div>

        <div className="tp-props">
          {item === undefined || selected === null ? (
            <p className="d-note">Select an item on the page to edit it, or add one above.</p>
          ) : (
            <ItemProperties
              key={selected}
              item={item}
              index={selected}
              pageW={pageW}
              t={t}
              issues={issues}
              set={(fn) => setItem(selected, fn)}
              boxSize={() => sizes.current.get(selected)}
              onDuplicate={() => duplicate(selected)}
              onRemove={() => remove(selected)}
              onReplaceImage={async () => {
                const asset = await pickImage();
                if (asset) setItem(selected, (it) => void (it.content.type === "image" && (it.content.asset = asset)));
              }}
              canImport={canImport}
            />
          )}
        </div>
      </div>
    </div>
  );
}

function CanvasItem({
  item,
  scale,
  selected,
  hasError,
  metadata,
  stretch,
  imageUrl,
  onPointerDown,
  onMeasure,
}: {
  item: TitleItem;
  stretch: number;
  onMeasure: (el: HTMLDivElement | null) => void;
  scale: number;
  selected: boolean;
  hasError: boolean;
  metadata: api.Metadata;
  imageUrl?: string;
  onPointerDown: (e: React.PointerEvent, mode: "move" | "resize") => void;
}) {
  const [h, v] = anchorShift(item.anchor);
  const c = item.content;
  const base: React.CSSProperties = {
    left: item.xMm * scale,
    top: item.yMm * scale,
    transform: `translate(${-h * 100}%, ${-v * 100}%)`,
  };
  let body: React.ReactNode;
  let style: React.CSSProperties = base;

  if (c.type === "text") {
    const s = c.style;
    const sizePx = s.sizePt * MM_PER_PT * scale;
    style = {
      ...base,
      width: c.widthMm !== undefined ? c.widthMm * scale : undefined,
      whiteSpace: c.widthMm !== undefined ? "pre-line" : "pre",
      fontSize: sizePx,
      lineHeight: 1.2 * stretch,
      fontWeight: s.bold ? 700 : 400,
      fontStyle: s.italic ? "italic" : "normal",
      fontVariant: s.smallCaps ? "small-caps" : "normal",
      textTransform: s.uppercase ? "uppercase" : "none",
      textAlign: s.align,
      color: s.color ?? "#000",
    };
    const resolved = resolveText(c.text, metadata);
    body = resolved.trim() === "" ? <span className="tp-empty">(empty)</span> : resolved;
  } else if (c.type === "image") {
    style = { ...base, width: c.widthMm * scale };
    body = imageUrl ? (
      <img src={imageUrl} alt={c.asset} draggable={false} style={{ width: "100%", display: "block" }} />
    ) : (
      <div className="tp-image-missing" style={{ height: c.widthMm * scale * 0.6 }}>
        {c.asset}
      </div>
    );
  } else {
    // Lines: (x, y) is on the line itself; only the horizontal anchor matters.
    const thickness = Math.max(1, c.thicknessPt * MM_PER_PT * scale);
    style = {
      left: item.xMm * scale,
      top: item.yMm * scale,
      width: c.lengthMm * scale,
      transform: `translate(${-h * 100}%, -50%)`,
      padding: "4px 0",
    };
    body = <div style={{ height: thickness, background: "#000" }} />;
  }

  return (
    <div
      ref={onMeasure}
      className={`tp-item${selected ? " selected" : ""}${hasError ? " error" : ""}`}
      style={style}
      onPointerDown={(e) => onPointerDown(e, "move")}
    >
      {body}
      {selected && (c.type !== "text" || c.widthMm !== undefined) && (
        <span className="tp-handle" title="Drag to resize" onPointerDown={(e) => onPointerDown(e, "resize")} />
      )}
      {selected && <span className="tp-anchor-dot" style={{ left: `${h * 100}%`, top: c.type === "rule" ? "50%" : `${v * 100}%` }} />}
    </div>
  );
}

function AnchorPicker({ value, onChange }: { value: Anchor; onChange: (a: Anchor) => void }) {
  return (
    <div className="tp-anchor" role="radiogroup" aria-label="Anchor point">
      {ANCHORS.flat().map((a) => (
        <button key={a} type="button" role="radio" aria-checked={a === value} title={a} className={a === value ? "active" : ""} onClick={() => onChange(a)} />
      ))}
    </div>
  );
}

function ItemProperties({
  item,
  index,
  pageW,
  t,
  issues,
  set,
  boxSize,
  onDuplicate,
  onRemove,
  onReplaceImage,
  canImport,
}: {
  item: TitleItem;
  index: number;
  pageW: number;
  t: SectionProps["t"];
  issues: SectionProps["issues"];
  set: (fn: (it: TitleItem) => void) => void;
  boxSize: () => { w: number; h: number } | undefined;
  onDuplicate: () => void;
  onRemove: () => void;
  onReplaceImage: () => void;
  canImport: boolean;
}) {
  const c = item.content;
  const path = `titlePage.items[${index}]`;
  const kind = c.type === "text" ? "Text" : c.type === "image" ? "Image" : "Line";
  const centre = () =>
    set((it) => {
      const [h] = anchorShift(it.anchor);
      // Switch to a centred anchor so the item stays centred when its width changes.
      if (h !== 0.5) it.anchor = it.anchor.startsWith("bottom") ? "bottom" : it.anchor.startsWith("top") ? "top" : "center";
      it.xMm = pageW / 2;
      if (it.content.type === "text") it.content.style.align = "center";
    });

  return (
    <div className="tp-props-inner">
      <div className="d-list-row">
        <strong>{kind}</strong>
        <span className="d-inline">
          <button type="button" onClick={centre} title="Centre horizontally on the page">
            Centre
          </button>
          <button type="button" onClick={onDuplicate}>
            Duplicate
          </button>
          <button type="button" className="danger" onClick={onRemove}>
            Delete
          </button>
        </span>
      </div>

      {c.type === "text" && (
        <>
          <Row label="Text" issues={issuesAt(issues, `${path}.content.text`)}>
            <PlaceholderInput
              multiline
              value={c.text}
              onChange={(v) => set((it) => void (it.content.type === "text" && (it.content.text = v)))}
              placeholders={fieldPlaceholders(t, ["text", "list", "paragraphs"])}
            />
          </Row>
          <TextStyleControls
            style={c.style}
            path={`${path}.content.style`}
            issues={issues}
            onChange={(fn) => set((it) => void (it.content.type === "text" && fn(it.content.style)))}
          />
          <Row label="Wrap width" issues={issuesAt(issues, `${path}.content.widthMm`)}>
            <span className="d-inline">
              <Toggle
                checked={c.widthMm !== undefined}
                label="Wrap long text"
                onChange={(on) =>
                  set((it) => {
                    if (it.content.type !== "text") return;
                    if (on) it.content.widthMm = 120;
                    else delete it.content.widthMm;
                  })
                }
              />
              {c.widthMm !== undefined && (
                <NumberInput value={c.widthMm} unit="mm" min={10} onChange={(v) => set((it) => void (it.content.type === "text" && (it.content.widthMm = v)))} />
              )}
            </span>
          </Row>
        </>
      )}

      {c.type === "image" && (
        <>
          <Row label="Image" issues={issuesAt(issues, `${path}.content.asset`)}>
            <span className="d-inline">
              <code>{c.asset}</code>
              <button type="button" onClick={onReplaceImage} disabled={!canImport}>
                Replace…
              </button>
            </span>
          </Row>
          <Row label="Width" issues={issuesAt(issues, `${path}.content.widthMm`)}>
            <NumberInput value={c.widthMm} unit="mm" min={1} onChange={(v) => set((it) => void (it.content.type === "image" && (it.content.widthMm = v)))} />
          </Row>
        </>
      )}

      {c.type === "rule" && (
        <>
          <Row label="Length" issues={issuesAt(issues, `${path}.content.lengthMm`)}>
            <NumberInput value={c.lengthMm} unit="mm" min={1} onChange={(v) => set((it) => void (it.content.type === "rule" && (it.content.lengthMm = v)))} />
          </Row>
          <Row label="Thickness" issues={issuesAt(issues, `${path}.content.thicknessPt`)}>
            <NumberInput
              value={c.thicknessPt}
              unit="pt"
              min={0.1}
              step={0.1}
              onChange={(v) => set((it) => void (it.content.type === "rule" && (it.content.thicknessPt = v)))}
            />
          </Row>
        </>
      )}

      <Row label="Position" hint="From the paper's top-left corner">
        <span className="d-inline">
          <NumberInput value={item.xMm} unit="mm across" step={0.5} onChange={(v) => set((it) => void (it.xMm = v))} />
          <NumberInput value={item.yMm} unit="mm down" step={0.5} onChange={(v) => set((it) => void (it.yMm = v))} />
        </span>
      </Row>
      <Row label="Anchor" hint="The point the position refers to; changing it doesn't move the item">
        <AnchorPicker
          value={item.anchor}
          onChange={(a) =>
            set((it) => {
              // Keep the item where it is on the page: move the reference point instead.
              const size = boxSize();
              if (size) {
                const [h0, v0] = anchorShift(it.anchor);
                const [h1, v1] = anchorShift(a);
                const height = it.content.type === "rule" ? 0 : size.h;
                it.xMm = Math.round((it.xMm + (h1 - h0) * size.w) * 10) / 10;
                it.yMm = Math.round((it.yMm + (v1 - v0) * height) * 10) / 10;
              }
              it.anchor = a;
            })
          }
        />
      </Row>
    </div>
  );
}
