// Title page designer: a page-sized canvas where items are placed by dragging. The
// same canvas lays out fixed pages (approval and signature pages, declarations…).
// Items snap to the page's centre and margins and to each other; several can be
// selected (Shift-click or drag a frame around them) to move and align together.
//
// Geometry matches the generated TikZ exactly: each item's (xMm, yMm) is its anchor
// point measured from the paper's top-left corner, text boxes are `widthMm` wide with
// no padding, and line height is 1.2 × the font size × the template's line spacing.
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { ask, open } from "@tauri-apps/plugin-dialog";
import * as api from "../api";
import type { Anchor } from "../bindings/Anchor";
import type { Template } from "../bindings/Template";
import type { FontChoice } from "../bindings/FontChoice";
import type { TitleItem } from "../bindings/TitleItem";
import type { AuthorInfo } from "../bindings/AuthorInfo";
import { NumberInput, PlaceholderInput, Row, Select, Toggle, issuesAt } from "./controls";
import { AUTHOR_SUFFIXES, fieldPlaceholders, fieldUses, placeholderNames, type SectionProps } from "./common";
import { TextStyleControls } from "./StyleSections";
import { translate, useT, type MessageKey } from "../i18n";
import { withDates } from "../editor/dates";
import { libraryFamily } from "../editor/libraryFonts";
import { pageFontStack } from "../editor/templateStyle";

const MM_PER_PT = 25.4 / 72.27; // TeX points
const SNAP_MM = 2; // distance at which items snap to the page's centre, margins and other items

/** What each anchor is called (its tooltip in the anchor picker). */
const ANCHOR_NAMES: Record<Anchor, MessageKey> = {
  topLeft: "designerContent.anchorTopLeft",
  top: "designerContent.anchorTop",
  topRight: "designerContent.anchorTopRight",
  left: "designerContent.anchorLeft",
  center: "designerContent.anchorCenter",
  right: "designerContent.anchorRight",
  bottomLeft: "designerContent.anchorBottomLeft",
  bottom: "designerContent.anchorBottom",
  bottomRight: "designerContent.anchorBottomRight",
};

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
  if (f.kind === "library") return `"${libraryFamily(f.id)}", "Amiri", serif`;
  return {
    latinModern: '"Latin Modern Roman", "CMU Serif", serif',
    termes: '"TeX Gyre Termes", "Times New Roman", Times, serif',
    pagella: '"TeX Gyre Pagella", Palatino, "Palatino Linotype", serif',
    bonum: '"TeX Gyre Bonum", "Bookman Old Style", serif',
    schola: '"TeX Gyre Schola", "Century Schoolbook", serif',
    heros: '"TeX Gyre Heros", Helvetica, Arial, sans-serif',
    adventor: '"TeX Gyre Adventor", "Avant Garde", sans-serif',
    cursor: '"TeX Gyre Cursor", "Courier New", monospace',
    amiri: '"Amiri", "Traditional Arabic", serif',
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

/** When authors don't share one set of affiliations: the affiliations without repeats
 * and each author's numbers in that list (as `author_entries` in the backend). */
function affiliationNumbers(authors: AuthorInfo[]) {
  const unique: string[] = [];
  for (const a of authors) for (const f of a.affiliations ?? []) if (f.trim() && !unique.includes(f.trim())) unique.push(f.trim());
  const marks = (a: AuthorInfo) => [...new Set((a.affiliations ?? []).map((f) => unique.indexOf(f.trim()) + 1).filter((n) => n > 0))];
  const first = authors[0] ? marks(authors[0]).join() : "";
  const differ = unique.length > 1 && authors.some((a) => marks(a).join() !== first);
  return differ ? { unique, marks } : null;
}

/** A title-page image that shows a document's image field (`{logo}`). */
const isFieldImage = (asset: string) => /^\{[a-z][a-z0-9_]*\}$/.test(asset);

/** The field's label in brackets, as the designer shows empty fields. */
const fieldImageLabel = (t: Template, asset: string) => `[${t.fields.find((f) => `{${f.id}}` === asset)?.label ?? asset}]`;

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
  /** The item under the pointer (it snaps; resizing changes only it). */
  index: number;
  mode: "move" | "resize";
  startX: number;
  startY: number;
  /** Every item that moves, with where it started. */
  origs: Map<number, TitleItem>;
}

/** A frame dragged on the empty page to select the items it touches (in mm). */
interface Marquee {
  x0: number;
  y0: number;
  x1: number;
  y1: number;
  /** Shift held: add to the selection instead of replacing it. */
  add: boolean;
  base: number[];
}

/** Keeps pointer events coming to the element while dragging (a pointer that's no
 *  longer down can't be captured; then the drag just ends at the element's edge). */
function capture(e: React.PointerEvent) {
  try {
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  } catch {
    // nothing to capture
  }
}

/** An item's box on the page in mm. Lines are as thin as their position. */
interface Box {
  left: number;
  top: number;
  w: number;
  h: number;
}

/** The items of the page being designed: the title page, or fixed page `page`. */
function pageItems(t: Template, page: number): TitleItem[] {
  if (page < 0) return t.titlePage.items;
  const fp = t.frontMatter[page];
  return fp?.type === "fixedPage" ? fp.items : t.titlePage.items;
}

/** A starting layout for a new fixed page: a heading and three signature lines. */
export function approvalPageItems(pageW: number): TitleItem[] {
  const text = (text: string, y: number, size: number, bold = false): TitleItem => ({
    xMm: pageW / 2,
    yMm: y,
    anchor: "top",
    content: { type: "text", text, widthMm: 150, style: { sizePt: size, bold, italic: false, smallCaps: false, uppercase: false, align: "center" } },
  });
  const signature = (label: string, y: number): TitleItem[] => [
    { xMm: pageW / 2, yMm: y, anchor: "top", content: { type: "rule", lengthMm: 90, thicknessPt: 0.6 } },
    text(label, y + 2, 11),
  ];
  return [
    text("APPROVAL PAGE", 40, 16, true),
    text("This report, “{title}”, has been read and approved by:", 70, 12),
    ...signature("Advisor", 130),
    ...signature("Committee member", 170),
    ...signature("Department chair", 210),
  ];
}

export function TitlePageCanvas({
  t,
  edit,
  issues,
  metadata,
  templatePath,
  canImport,
}: SectionProps & { metadata: api.Metadata; templatePath: string | null; canImport: boolean }) {
  const tr = useT();
  /** The page being designed: -1 for the title page, else a fixed page's index in the front matter. */
  const [page, setPage] = useState(-1);
  const [selection, setSelection] = useState<number[]>([]);
  /** The one selected item, whose properties show on the right. */
  const selected = selection.length === 1 ? selection[0] : null;
  const setSelected = (i: number | null) => setSelection(i === null ? [] : [i]);
  /** Snap lines shown while dragging (mm). */
  const [guides, setGuides] = useState<{ x?: number; y?: number }>({});
  const [marquee, setMarquee] = useState<Marquee | null>(null);
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
  const fixedPage = page >= 0 && t.frontMatter[page]?.type === "fixedPage" ? t.frontMatter[page] : null;
  /** The document's values as they print (dates written out, Hijri forms added). */
  const shownMetadata = useMemo(() => {
    const shown = withDates(t, metadata);
    // An empty field shows as "[Label]", as the PDF shows a missing one: text in the
    // template's language, so its font can be judged ("{title}" says little).
    for (const f of t.fields) {
      // An authors field's affiliations, English names and e-mail (as the PDF writes them).
      if (f.kind === "authors") {
        const list = Array.isArray(metadata[f.id]) ? (metadata[f.id] as unknown[]) : [];
        const authors = list.filter((a): a is AuthorInfo => typeof a === "object" && a !== null);
        const unique = (xs: string[]) => [...new Set(xs.map((x) => x.trim()).filter(Boolean))];
        const email = authors.find((a) => a.corresponding && a.email?.trim()) ?? authors.find((a) => a.email?.trim());
        const derived: Record<string, string[]> = {
          _affiliations: unique(authors.flatMap((a) => a.affiliations ?? [])),
          _en: unique(authors.map((a) => a.nameEn ?? "")),
          _affiliations_en: unique(authors.flatMap((a) => a.affiliationsEn ?? [])),
          _email: email ? [email.email] : [],
        };
        // Authors with different affiliations: numbers pair names and affiliations
        // (raised in the PDF; superscript digits here).
        const numbered = affiliationNumbers(authors);
        if (numbered) {
          const sup = (n: number[]) => n.join(",").replace(/\d/g, (d) => "⁰¹²³⁴⁵⁶⁷⁸⁹"[Number(d)]);
          const names = authors.filter((a) => a.name.trim());
          if (names.length) shown[f.id] = names.map((a) => a.name.trim() + sup(numbered.marks(a)));
          derived._en = authors.filter((a) => a.nameEn?.trim()).map((a) => a.nameEn!.trim() + sup(numbered.marks(a)));
          derived._affiliations = numbered.unique.map((u, i) => sup([i + 1]) + u);
          derived._affiliations_en = numbered.unique.flatMap((u, i) => {
            const en = authors.map((a) => a.affiliationsEn?.[(a.affiliations ?? []).findIndex((x) => x.trim() === u)]?.trim()).find(Boolean);
            return en ? [sup([i + 1]) + en] : [];
          });
        }
        for (const [suffix, key] of AUTHOR_SUFFIXES) {
          const v = derived[suffix];
          shown[f.id + suffix] = v.length ? v : `[${translate(t.language?.main === "arabic" ? "ar" : "en", key, { label: f.label })}]`;
        }
      }
      const v = shown[f.id];
      const empty = v === undefined || (typeof v === "string" ? !v.trim() : v.length === 0);
      if (empty && f.kind !== "image") {
        shown[f.id] = `[${f.label}]`;
        if (f.kind === "date") shown[`${f.id}_hijri`] = `[${f.label}]`;
      }
    }
    return shown;
  }, [t, metadata]);
  /** An article's title block: where the text starts below it (mm); undefined for a title page. */
  const textStart = t.titlePage.blockHeightMm;
  const items = pageItems(t, fixedPage ? page : -1);
  const itemsOf = (d: Template) => pageItems(d, fixedPage ? page : -1);
  const pathPrefix = fixedPage ? `frontMatter[${page}].items` : "titlePage.items";
  const item = selected !== null ? items[selected] : undefined;
  const margins = t.page.margins;
  const marginBox = { left: margins.innerMm + t.page.bindingOffsetMm, right: pageW - margins.outerMm, top: margins.topMm, bottom: pageH - margins.bottomMm };

  useLayoutEffect(() => {
    const el = wrap.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setWidth(el.clientWidth));
    ro.observe(el);
    setWidth(el.clientWidth);
    return () => ro.disconnect();
  }, []);

  // Keep the selection valid when items are removed elsewhere (undo, reload), and
  // go back to the title page when the fixed page being designed is removed.
  useEffect(() => {
    if (selection.some((i) => i >= items.length)) setSelection((sel) => sel.filter((i) => i < items.length));
  }, [items.length, selection]);
  useEffect(() => {
    if (page >= 0 && !fixedPage) setPage(-1);
  }, [page, fixedPage]);
  const choosePage = (p: number) => {
    setPage(p);
    setSelection([]);
    sizes.current.clear();
    setNotice(null);
  };

  const assets = useMemo(
    // An image field's picture is the document's, not a template file.
    () => items.flatMap((i) => (i.content.type === "image" && !isFieldImage(i.content.asset) ? [i.content.asset] : [])),
    [items],
  );

  /** An item's box in mm, from its measured size. */
  const boxOf = (it: TitleItem, i: number): Box => {
    const size = sizes.current.get(i) ?? { w: 0, h: 0 };
    const [h, v] = anchorShift(it.anchor);
    if (it.content.type === "rule") return { left: it.xMm - h * it.content.lengthMm, top: it.yMm, w: it.content.lengthMm, h: 0 };
    return { left: it.xMm - h * size.w, top: it.yMm - v * size.h, w: size.w, h: size.h };
  };
  const assetUrls = useAssetUrls(assets, templatePath);

  const setItem = (index: number, fn: (it: TitleItem) => void) => edit((d) => fn(itemsOf(d)[index]));

  // ---- dragging (pointer events work for mouse, pen and touch)
  const onPointerDown = (e: React.PointerEvent, index: number, mode: Drag["mode"]) => {
    e.preventDefault(); // no text selection while dragging…
    e.stopPropagation();
    pageRef.current?.focus({ preventScroll: true }); // …but keep keyboard nudging working
    let moving = selection;
    if ((e.shiftKey || e.ctrlKey || e.metaKey) && mode === "move") {
      // Shift/Ctrl-click adds or removes an item from the selection.
      if (selection.includes(index)) {
        setSelection(selection.filter((i) => i !== index));
        return;
      }
      moving = [...selection, index];
    } else if (!selection.includes(index) || mode === "resize") {
      moving = [index];
    }
    setSelection(moving);
    capture(e);
    const origs = new Map(moving.map((i) => [i, structuredClone(items[i])] as const));
    drag.current = { index, mode, startX: e.clientX, startY: e.clientY, origs };
  };

  /** Where the dragged item's edges snap: the page's centre and margins, and the
   *  other items' edges and centres. Returns the shift to apply (mm). */
  const snap = (box: Box, moving: Map<number, TitleItem>): { dx: number; dy: number; x?: number; y?: number } => {
    const xs = [pageW / 2, marginBox.left, marginBox.right];
    const ys = [pageH / 2, marginBox.top, marginBox.bottom];
    items.forEach((it, i) => {
      if (moving.has(i)) return;
      const b = boxOf(it, i);
      xs.push(b.left, b.left + b.w / 2, b.left + b.w);
      ys.push(b.top, b.top + b.h / 2, b.top + b.h);
    });
    const best = (edges: number[], targets: number[]) => {
      let found: { d: number; at: number } | undefined;
      for (const e of edges)
        for (const at of targets) {
          const d = at - e;
          if (Math.abs(d) < SNAP_MM && (!found || Math.abs(d) < Math.abs(found.d))) found = { d, at };
        }
      return found;
    };
    const bx = best([box.left, box.left + box.w / 2, box.left + box.w], xs);
    const by = best([box.top, box.top + box.h / 2, box.top + box.h], ys);
    return { dx: bx?.d ?? 0, dy: by?.d ?? 0, x: bx?.at, y: by?.at };
  };

  const onPointerMove = (e: React.PointerEvent) => {
    if (marquee) {
      const r = pageRef.current!.getBoundingClientRect();
      setMarquee({ ...marquee, x1: (e.clientX - r.left) / scale, y1: (e.clientY - r.top) / scale });
      return;
    }
    const d = drag.current;
    if (!d) return;
    const dx = (e.clientX - d.startX) / scale;
    const dy = (e.clientY - d.startY) / scale;
    const free = e.altKey; // hold Alt for 0.1 mm steps and no snapping
    const round = (v: number) => (free ? Math.round(v * 10) / 10 : Math.round(v));
    const orig = d.origs.get(d.index)!;
    if (d.mode === "move") {
      // Move the dragged item, snap it, and move the others by the same amount.
      let mx = round(orig.xMm + dx) - orig.xMm;
      let my = round(orig.yMm + dy) - orig.yMm;
      let shown: { x?: number; y?: number } = {};
      if (!free) {
        const b = boxOf(orig, d.index);
        const s = snap({ ...b, left: b.left + mx, top: b.top + my }, d.origs);
        mx = Math.round((mx + s.dx) * 10) / 10;
        my = Math.round((my + s.dy) * 10) / 10;
        shown = { x: s.x, y: s.y };
      }
      setGuides(shown);
      edit((draft) => {
        const list = itemsOf(draft);
        for (const [i, o] of d.origs) {
          if (!list[i]) continue;
          list[i].xMm = Math.round((o.xMm + mx) * 10) / 10;
          list[i].yMm = Math.round((o.yMm + my) * 10) / 10;
        }
      });
    } else {
      // Resize from the right edge; a centred box grows on both sides.
      const [h] = anchorShift(orig.anchor);
      const factor = h === 0.5 ? 2 : h === 1 ? -1 : 1;
      setItem(d.index, (it) => {
        const c = it.content;
        const o = orig.content;
        if (c.type === "text" && o.type === "text") c.widthMm = Math.max(10, round((o.widthMm ?? 60) + dx * factor));
        if (c.type === "image" && o.type === "image") c.widthMm = Math.max(5, round(o.widthMm + dx * factor));
        if (c.type === "rule" && o.type === "rule") c.lengthMm = Math.max(5, round(o.lengthMm + dx * factor));
      });
    }
  };

  const onPointerUp = () => {
    drag.current = null;
    setGuides({});
    if (marquee) {
      // Select every item the frame touches.
      const [l, r] = [Math.min(marquee.x0, marquee.x1), Math.max(marquee.x0, marquee.x1)];
      const [tp, bt] = [Math.min(marquee.y0, marquee.y1), Math.max(marquee.y0, marquee.y1)];
      const hit = items
        .map((it, i) => [boxOf(it, i), i] as const)
        .filter(([b]) => b.left <= r && b.left + b.w >= l && b.top <= bt && b.top + b.h >= tp)
        .map(([, i]) => i);
      setSelection(marquee.add ? [...new Set([...marquee.base, ...hit])] : hit);
      setMarquee(null);
    }
  };

  // ---- keyboard: arrows nudge (Shift = 10 mm), Delete removes
  const onKeyDown = (e: React.KeyboardEvent) => {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "a") {
      e.preventDefault();
      setSelection(items.map((_, i) => i));
      return;
    }
    if (selection.length === 0) return;
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
      edit((d) => {
        for (const i of selection) {
          const it = itemsOf(d)[i];
          if (!it) continue;
          it.xMm = Math.round((it.xMm + mx) * 10) / 10;
          it.yMm = Math.round((it.yMm + my) * 10) / 10;
        }
      });
    } else if (e.key === "Delete" || e.key === "Backspace") {
      e.preventDefault();
      remove(selection);
    } else if (e.key === "Escape") {
      setSelected(null);
    }
  };

  // ---- item actions
  const add = (it: TitleItem) => {
    edit((d) => void itemsOf(d).push(it));
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
    const file = await open({ multiple: false, directory: false, filters: [{ name: tr("designerContent.imagesFilter"), extensions: ["png", "jpg", "jpeg", "pdf"] }] });
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
      setNotice(tr("designerContent.saveBeforeImage"));
      return;
    }
    const asset = await pickImage();
    if (asset) add({ xMm: pageW / 2, yMm: 20, anchor: "top", content: { type: "image", asset, widthMm: 35 } });
  };
  // Removing the only text that shows a field ({course}) offers to remove the field
  // too; otherwise Document info keeps asking for a value that appears nowhere.
  const remove = async (indices: number[]) => {
    const gone = new Set(indices);
    const shown = indices.flatMap((i) => {
      const content = items[i]?.content;
      return content?.type === "text" ? placeholderNames(content.text) : [];
    });
    const keep = (list: TitleItem[]) => list.filter((_, i) => !gone.has(i));
    const after = structuredClone(t);
    const afterList = itemsOf(after);
    afterList.splice(0, afterList.length, ...keep(afterList));
    const uses = fieldUses(after);
    const orphans = t.fields.filter((f) => shown.includes(f.id) && !uses.has(f.id));
    setSelected(null);
    // Fields still shown elsewhere stay in Document info; say where, or it looks like a bug.
    const kept = t.fields.filter((f) => shown.includes(f.id) && uses.has(f.id));
    setNotice(
      kept.length > 0
        ? kept
            .map((f) => tr("designerContent.fieldStillUsed", { label: f.label, places: uses.get(f.id)!.join(tr("designerContent.and")) }))
            .join(" ")
        : null,
    );
    let dropFields = false;
    if (orphans.length > 0) {
      const names = orphans.map((f) => tr("designerContent.quoted", { text: f.label })).join(tr("designerContent.listSeparator"));
      const one = orphans.length === 1;
      dropFields = await ask(tr(one ? "designerContent.removeOrphanField" : "designerContent.removeOrphanFields", { names }), {
        title: tr("designerContent.removeField"),
        kind: "info",
        okLabel: tr(one ? "designerContent.removeField" : "designerContent.removeFields"),
        cancelLabel: tr("designerContent.keep"),
      });
    }
    edit((d) => {
      const list = itemsOf(d);
      list.splice(0, list.length, ...keep(list));
      if (dropFields) d.fields = d.fields.filter((f) => !orphans.some((o) => o.id === f.id));
    });
  };
  const duplicate = (index: number) => {
    const copy = structuredClone(items[index]);
    copy.yMm += 10;
    add(copy);
  };

  /** Lines up the selected items: their left edges, centres… (relative to the group). */
  const align = (how: "left" | "center" | "right" | "top" | "middle" | "bottom" | "page") => {
    const boxes = selection.map((i) => [i, boxOf(items[i], i)] as const);
    const minL = Math.min(...boxes.map(([, b]) => b.left));
    const maxR = Math.max(...boxes.map(([, b]) => b.left + b.w));
    const minT = Math.min(...boxes.map(([, b]) => b.top));
    const maxB = Math.max(...boxes.map(([, b]) => b.top + b.h));
    edit((d) => {
      const list = itemsOf(d);
      for (const [i, b] of boxes) {
        const it = list[i];
        if (!it) continue;
        const shift = {
          left: [minL - b.left, 0],
          center: [(minL + maxR) / 2 - (b.left + b.w / 2), 0],
          right: [maxR - (b.left + b.w), 0],
          top: [0, minT - b.top],
          middle: [0, (minT + maxB) / 2 - (b.top + b.h / 2)],
          bottom: [0, maxB - (b.top + b.h)],
          // The group as a whole, centred between the page's edges.
          page: [pageW / 2 - (minL + maxR) / 2, 0],
        }[how];
        it.xMm = Math.round((it.xMm + shift[0]) * 10) / 10;
        it.yMm = Math.round((it.yMm + shift[1]) * 10) / 10;
      }
    });
  };

  /** Spreads three or more items so the gaps between them are equal. */
  const distribute = (axis: "x" | "y") => {
    const boxes = selection.map((i) => [i, boxOf(items[i], i)] as const);
    const start = (b: Box) => (axis === "x" ? b.left : b.top);
    const size = (b: Box) => (axis === "x" ? b.w : b.h);
    boxes.sort((a, b) => start(a[1]) - start(b[1]));
    const first = start(boxes[0][1]);
    const last = boxes[boxes.length - 1][1];
    const total = start(last) + size(last) - first;
    const gap = (total - boxes.reduce((sum, [, b]) => sum + size(b), 0)) / (boxes.length - 1);
    let at = first;
    edit((d) => {
      const list = itemsOf(d);
      for (const [i, b] of boxes) {
        const it = list[i];
        if (it) {
          const delta = Math.round((at - start(b)) * 10) / 10;
          if (axis === "x") it.xMm = Math.round((it.xMm + delta) * 10) / 10;
          else it.yMm = Math.round((it.yMm + delta) * 10) / 10;
        }
        at += size(b) + gap;
      }
    });
  };

  const addFixedPage = () => {
    // After the title page (or first), with a starting layout to adapt.
    const at = Math.max(0, t.frontMatter.findIndex((f) => f.type === "titlePage") + 1);
    edit((d) => void d.frontMatter.splice(at, 0, { type: "fixedPage", title: "Approval page", items: approvalPageItems(pageW), inToc: false }));
    choosePage(at);
  };
  const fixedPages = t.frontMatter.flatMap((f, i) => (f.type === "fixedPage" ? [{ index: i, title: f.title }] : []));

  // The page's font: the main font for Latin letters, the Arabic font for Arabic, as
  // the editor page and the PDF.
  const font = pageFontStack(t);

  return (
    <div className="tp">
      <div className="tp-toolbar tp-pages">
        <label>
          {tr("designerContent.page")}{" "}
          <select value={fixedPage ? page : -1} onChange={(e) => choosePage(Number(e.target.value))} aria-label={tr("designerContent.pageToDesign")}>
            <option value={-1}>{tr("designerContent.titlePage")}</option>
            {fixedPages.map((f) => (
              <option key={f.index} value={f.index}>
                {f.title.trim() || tr("designerContent.fixedPage")}
              </option>
            ))}
          </select>
        </label>
        <button type="button" onClick={addFixedPage} title={tr("designerContent.addFixedPageHint")}>
          {tr("designerContent.addFixedPage")}
        </button>
        {!fixedPage && (
          <>
            <label title={tr("designerContent.titleLayoutHint")}>
              {tr("designerContent.titleLayout")}{" "}
              <select
                value={textStart === undefined ? "page" : "block"}
                onChange={(e) =>
                  edit((d) => {
                    if (e.target.value === "page") delete d.titlePage.blockHeightMm;
                    // Below the lowest item, to start with.
                    else d.titlePage.blockHeightMm = Math.round(Math.max(d.page.margins.topMm, ...d.titlePage.items.map((it) => it.yMm)) + 15);
                  })
                }
              >
                <option value="page">{tr("designerContent.ownPage")}</option>
                <option value="block">{tr("designerContent.titleBlock")}</option>
              </select>
            </label>
            {textStart !== undefined && (
              <label title={tr("designerContent.textStartsAtHint")}>
                {tr("designerContent.textStartsAt")}{" "}
                <NumberInput value={textStart} unit="mm" step={1} min={0} onChange={(v) => edit((d) => void (d.titlePage.blockHeightMm = v))} />
              </label>
            )}
          </>
        )}
      </div>
      <div className="tp-toolbar">
        <button type="button" onClick={addText}>
          {tr("designerContent.addText")}
        </button>
        <button type="button" onClick={addImage} title={tr(canImport ? "designerContent.imageFormats" : "designerContent.saveTemplateFirst")}>
          {tr("designerContent.addImage")}
        </button>
        <button type="button" onClick={addLine}>
          {tr("designerContent.addLine")}
        </button>
        <span className="d-hint">{tr("designerContent.canvasHint")}</span>
      </div>
      {notice && <div className="d-warning">{notice}</div>}

      <div className="tp-layout">
        <div className="tp-canvas-wrap" ref={wrap}>
          <div
            ref={pageRef}
            className="tp-page"
            // The page keeps the document's direction whatever the interface's is;
            // item positions stay physical (mm from the paper's top-left corner).
            dir={t.language?.main === "arabic" ? "rtl" : "ltr"}
            style={{ width: pageW * scale, height: pageH * scale, fontFamily: font }}
            tabIndex={0}
            role="application"
            aria-label={tr("designerContent.canvasLabel")}
            onKeyDown={onKeyDown}
            onPointerMove={onPointerMove}
            onPointerUp={onPointerUp}
            onPointerCancel={onPointerUp}
            onPointerDown={(e) => {
              pageRef.current?.focus({ preventScroll: true });
              // On the empty page: start a selection frame.
              const r = pageRef.current!.getBoundingClientRect();
              const x = (e.clientX - r.left) / scale;
              const y = (e.clientY - r.top) / scale;
              capture(e);
              setMarquee({ x0: x, y0: y, x1: x, y1: y, add: e.shiftKey, base: selection });
              if (!e.shiftKey) setSelection([]);
            }}
          >
            {!fixedPage && textStart !== undefined && (
              <div className="tp-text-start" style={{ top: textStart * scale }}>
                <span>{tr("designerContent.textStartsHere")}</span>
              </div>
            )}
            {guides.x !== undefined && <div className="tp-guide" style={{ left: guides.x * scale }} />}
            {guides.y !== undefined && <div className="tp-guide tp-guide-h" style={{ top: guides.y * scale }} />}
            {marquee && (
              <div
                className="tp-marquee"
                style={{
                  left: Math.min(marquee.x0, marquee.x1) * scale,
                  top: Math.min(marquee.y0, marquee.y1) * scale,
                  width: Math.abs(marquee.x1 - marquee.x0) * scale,
                  height: Math.abs(marquee.y1 - marquee.y0) * scale,
                }}
              />
            )}
            {items.map((it, i) => (
              <CanvasItem
                key={i}
                item={it}
                scale={scale}
                selected={selection.includes(i)}
                hasError={issuesAt(issues, `${pathPrefix}[${i}]`).length > 0}
                metadata={shownMetadata}
                stretch={lineStretch(t)}
                onMeasure={(el) => {
                  if (el) sizes.current.set(i, { w: el.offsetWidth / scale, h: el.offsetHeight / scale });
                }}
                imageUrl={it.content.type === "image" ? assetUrls[it.content.asset] : undefined}
                imageLabel={it.content.type === "image" && isFieldImage(it.content.asset) ? fieldImageLabel(t, it.content.asset) : undefined}
                onPointerDown={(e, mode) => onPointerDown(e, i, mode)}
              />
            ))}
          </div>
          <div className="d-hint tp-caption">
            {tr("designerContent.canvasCaption")}
          </div>
        </div>

        <div className="tp-props">
          {selection.length > 1 ? (
            <div className="tp-props-inner">
              <div className="d-list-row">
                <strong>{tr("designerContent.itemsSelected", { n: selection.length })}</strong>
                <button type="button" className="d-remove" onClick={() => remove(selection)}>
                  {tr("common.remove")}
                </button>
              </div>
              <Row label={tr("designerContent.align")}>
                <div className="tp-align">
                  {(
                    [
                      ["left", "designerContent.alignLeft"],
                      ["center", "designerContent.alignCenter"],
                      ["right", "designerContent.alignRight"],
                      ["top", "designerContent.alignTop"],
                      ["middle", "designerContent.alignMiddle"],
                      ["bottom", "designerContent.alignBottom"],
                    ] as const
                  ).map(([how, label]) => (
                    <button key={how} type="button" onClick={() => align(how)}>
                      {tr(label)}
                    </button>
                  ))}
                </div>
              </Row>
              <Row label={tr("designerContent.spread")}>
                <div className="tp-align">
                  <button type="button" disabled={selection.length < 3} onClick={() => distribute("x")} title={tr("designerContent.spreadAcrossHint")}>
                    {tr("designerContent.spreadAcross")}
                  </button>
                  <button type="button" disabled={selection.length < 3} onClick={() => distribute("y")} title={tr("designerContent.spreadDownHint")}>
                    {tr("designerContent.spreadDown")}
                  </button>
                  <button type="button" onClick={() => align("page")} title={tr("designerContent.centreOnPageHint")}>
                    {tr("designerContent.centreOnPage")}
                  </button>
                </div>
              </Row>
            </div>
          ) : item === undefined || selected === null ? (
            <>
              {fixedPage && fixedPage.type === "fixedPage" && (
                <div className="tp-props-inner">
                  <Row label={tr("common.name")} issues={issuesAt(issues, `frontMatter[${page}].title`)}>
                    <input
                      value={fixedPage.title}
                      onChange={(e) =>
                        edit((d) => {
                          const f = d.frontMatter[page];
                          if (f.type === "fixedPage") f.title = e.target.value;
                        })
                      }
                    />
                  </Row>
                  <Row label={tr("designerContent.contents")}>
                    <Toggle
                      checked={fixedPage.inToc}
                      label={tr("designerContent.listInToc")}
                      onChange={(v) =>
                        edit((d) => {
                          const f = d.frontMatter[page];
                          if (f.type === "fixedPage") f.inToc = v;
                        })
                      }
                    />
                  </Row>
                  <p className="d-note">{tr("designerContent.fixedPagePlaceNote")}</p>
                  <button type="button" className="d-remove" onClick={() => edit((d) => void d.frontMatter.splice(page, 1))}>
                    {tr("designerContent.removeThisPage")}
                  </button>
                </div>
              )}
              <p className="d-note">{tr("designerContent.selectItemNote")}</p>
            </>
          ) : (
            <ItemProperties
              key={selected}
              item={item}
              path={`${pathPrefix}[${selected}]`}
              pageW={pageW}
              t={t}
              issues={issues}
              set={(fn) => setItem(selected, fn)}
              boxSize={() => sizes.current.get(selected)}
              onDuplicate={() => duplicate(selected)}
              onRemove={() => remove([selected])}
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
  imageLabel,
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
  /** For an image field: what the box shows ("[University logo]"). */
  imageLabel?: string;
  onPointerDown: (e: React.PointerEvent, mode: "move" | "resize") => void;
}) {
  const tr = useT();
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
      // An item's own font (a title in another face), else the page's.
      ...(s.font ? { fontFamily: cssFont(s.font) } : {}),
    };
    const resolved = resolveText(c.text, metadata);
    body = resolved.trim() === "" ? <span className="tp-empty">{tr("designerContent.empty")}</span> : resolved;
  } else if (c.type === "image") {
    style = { ...base, width: c.widthMm * scale };
    body = imageUrl ? (
      <img src={imageUrl} alt={c.asset} draggable={false} style={{ width: "100%", display: "block" }} />
    ) : (
      <div className="tp-image-missing" style={{ height: c.widthMm * scale * 0.6 }}>
        {imageLabel ?? c.asset}
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
        <span className="tp-handle" title={tr("designerContent.dragToResize")} onPointerDown={(e) => onPointerDown(e, "resize")} />
      )}
      {selected && <span className="tp-anchor-dot" style={{ left: `${h * 100}%`, top: c.type === "rule" ? "50%" : `${v * 100}%` }} />}
    </div>
  );
}

function AnchorPicker({ value, onChange }: { value: Anchor; onChange: (a: Anchor) => void }) {
  const tr = useT();
  // Anchors are physical points of the item (top-left is top-left on the paper), so
  // the grid doesn't mirror in a right-to-left interface.
  return (
    <div className="tp-anchor" role="radiogroup" aria-label={tr("designerContent.anchorPoint")} dir="ltr">
      {ANCHORS.flat().map((a) => (
        <button key={a} type="button" role="radio" aria-checked={a === value} title={tr(ANCHOR_NAMES[a])} className={a === value ? "active" : ""} onClick={() => onChange(a)} />
      ))}
    </div>
  );
}

function ItemProperties({
  item,
  path,
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
  /** Where its issues are reported, e.g. titlePage.items[2]. */
  path: string;
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
  const tr = useT();
  const c = item.content;
  const imageFields = t.fields.filter((f) => f.kind === "image");
  const kind = tr(c.type === "text" ? "designerContent.kindText" : c.type === "image" ? "designerContent.kindImage" : "designerContent.kindLine");
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
          <button type="button" onClick={centre} title={tr("designerContent.centreHint")}>
            {tr("designerContent.centre")}
          </button>
          <button type="button" onClick={onDuplicate}>
            {tr("designerContent.duplicate")}
          </button>
          <button type="button" className="danger" onClick={onRemove}>
            {tr("common.delete")}
          </button>
        </span>
      </div>

      {c.type === "text" && (
        <>
          <Row label={tr("designerContent.kindText")} issues={issuesAt(issues, `${path}.content.text`)}>
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
          <Row label={tr("designerContent.wrapWidth")} issues={issuesAt(issues, `${path}.content.widthMm`)}>
            <span className="d-inline">
              <Toggle
                checked={c.widthMm !== undefined}
                label={tr("designerContent.wrapLongText")}
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
          {imageFields.length > 0 && (
            <Row label={tr("designerContent.imageSource")} hint={tr("designerContent.imageSourceHint")}>
              <Select
                value={isFieldImage(c.asset) ? c.asset : "file"}
                options={[
                  { value: "file", label: tr("designerContent.imageFile") },
                  ...imageFields.map((f) => ({ value: `{${f.id}}`, label: f.label })),
                ]}
                onChange={(v) => (v === "file" ? onReplaceImage() : set((it) => void (it.content.type === "image" && (it.content.asset = v))))}
              />
            </Row>
          )}
          <Row label={tr("designerContent.kindImage")} issues={issuesAt(issues, `${path}.content.asset`)}>
            <span className="d-inline">
              <code>{c.asset}</code>
              <button type="button" onClick={onReplaceImage} disabled={!canImport}>
                {tr("designerContent.replace")}
              </button>
            </span>
          </Row>
          <Row label={tr("designerContent.width")} issues={issuesAt(issues, `${path}.content.widthMm`)}>
            <NumberInput value={c.widthMm} unit="mm" min={1} onChange={(v) => set((it) => void (it.content.type === "image" && (it.content.widthMm = v)))} />
          </Row>
        </>
      )}

      {c.type === "rule" && (
        <>
          <Row label={tr("designerContent.length")} issues={issuesAt(issues, `${path}.content.lengthMm`)}>
            <NumberInput value={c.lengthMm} unit="mm" min={1} onChange={(v) => set((it) => void (it.content.type === "rule" && (it.content.lengthMm = v)))} />
          </Row>
          <Row label={tr("designerContent.thickness")} issues={issuesAt(issues, `${path}.content.thicknessPt`)}>
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

      <Row label={tr("designerContent.position")} hint={tr("designerContent.positionHint")}>
        <span className="d-inline">
          <NumberInput value={item.xMm} unit={tr("designerContent.mmAcross")} step={0.5} onChange={(v) => set((it) => void (it.xMm = v))} />
          <NumberInput value={item.yMm} unit={tr("designerContent.mmDown")} step={0.5} onChange={(v) => set((it) => void (it.yMm = v))} />
        </span>
      </Row>
      <Row label={tr("designerContent.anchor")} hint={tr("designerContent.anchorHint")}>
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
