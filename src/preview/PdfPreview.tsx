// Renders PDF bytes to canvases with pdf.js. New renders are drawn off-screen and
// swapped in when complete, so the preview doesn't flash blank while recompiling.
// Double-clicking a page reports the spot (SyncTeX jumps to the editor), and `spot`
// scrolls to a place in the PDF and marks it briefly.
import { useEffect, useRef } from "react";
import * as pdfjs from "pdfjs-dist";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import type { PdfSpot } from "../api";
import { useT } from "../i18n";

pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;

export function PdfPreview({
  pdf,
  docKey = 0,
  zoom = 100,
  onPages,
  onLocate,
  spot,
}: {
  pdf: ArrayBuffer | null;
  /** Changes when another document is shown: its PDF starts at the first page. */
  docKey?: number;
  /** Percent of the pane's width. */
  zoom?: number;
  onPages?: (count: number) => void;
  /** A double-click on a page: its number (1-based) and the point in PDF points from the top left. */
  onLocate?: (page: number, x: number, y: number) => void;
  /** A place to show; a new object each time, so the same place can be shown again. */
  spot?: PdfSpot | null;
}) {
  const t = useT();
  const reportPages = useRef(onPages);
  reportPages.current = onPages;
  const container = useRef<HTMLDivElement>(null);
  const scroller = useRef<HTMLDivElement>(null);
  /** A place to show once its page is drawn. */
  const pendingSpot = useRef<PdfSpot | null>(null);

  // Another document: back to the top now, and again when its own PDF is drawn (the
  // old one is still showing, and may be scrolled meanwhile).
  const toTop = useRef(false);
  useEffect(() => {
    toTop.current = true;
    scroller.current?.scrollTo({ top: 0 });
  }, [docKey]);

  /** Scrolls `spot` into view and marks it; false when its page isn't drawn yet. */
  const showSpot = (spot: PdfSpot): boolean => {
    const pane = scroller.current;
    const box = container.current?.querySelector<HTMLElement>(`.pdf-page-box[data-page="${spot.page}"]`);
    if (!pane || !box) return false;
    pendingSpot.current = null;
    const scale = box.clientWidth / Number(box.dataset.ptWidth);
    box.querySelectorAll(".pdf-sync-mark").forEach((m) => m.remove());
    const mark = document.createElement("div");
    mark.className = "pdf-sync-mark";
    mark.style.top = `${spot.y * scale - 3}px`;
    mark.style.height = `${spot.height * scale + 6}px`;
    box.appendChild(mark);
    mark.addEventListener("animationend", () => mark.remove());
    const top = box.getBoundingClientRect().top - pane.getBoundingClientRect().top + pane.scrollTop + spot.y * scale;
    pane.scrollTo({ top: Math.max(0, top - pane.clientHeight / 3), behavior: "smooth" });
    return true;
  };

  useEffect(() => {
    if (!pdf || !container.current) return;
    let cancelled = false;
    const target = container.current;

    (async () => {
      // pdf.js takes ownership of (detaches) the buffer, so hand it a copy.
      const task = pdfjs.getDocument({ data: new Uint8Array(pdf.slice(0)) });
      const doc = await task.promise;
      const pages = document.createElement("div");
      const dpr = window.devicePixelRatio || 1;
      reportPages.current?.(doc.numPages);
      // The pages' own box: inside the padding and next to the scrollbar.
      const cssWidth = Math.max(200, Math.floor(target.clientWidth * (zoom / 100)) - 2);

      for (let i = 1; i <= doc.numPages; i++) {
        if (cancelled) break;
        const page = await doc.getPage(i);
        const base = page.getViewport({ scale: 1 });
        const scale = cssWidth / base.width;
        const viewport = page.getViewport({ scale: scale * dpr });
        const canvas = document.createElement("canvas");
        canvas.className = "pdf-page";
        canvas.width = Math.floor(viewport.width);
        canvas.height = Math.floor(viewport.height);
        canvas.style.width = `${Math.floor(viewport.width / dpr)}px`;
        canvas.style.height = `${Math.floor(viewport.height / dpr)}px`;
        await page.render({ canvas, canvasContext: canvas.getContext("2d")!, viewport }).promise;
        // The box holds the page and the marks drawn over it; it knows the page's size
        // in PDF points to turn clicks into PDF coordinates.
        const box = document.createElement("div");
        box.className = "pdf-page-box";
        box.dataset.page = String(i);
        box.dataset.ptWidth = String(base.width);
        box.appendChild(canvas);
        pages.appendChild(box);
      }
      // Swapping in one synchronous step keeps the scroll position of the pane.
      if (!cancelled) {
        target.replaceChildren(...pages.childNodes);
        if (toTop.current) {
          toTop.current = false;
          scroller.current?.scrollTo({ top: 0 });
        }
        if (pendingSpot.current) showSpot(pendingSpot.current);
      }
      await task.destroy();
    })().catch((e) => console.error("PDF render failed", e));

    return () => {
      cancelled = true;
    };
  }, [pdf, zoom]);

  // Show a place: scroll it to a third of the way down the pane and mark it. Before
  // the pages are drawn (just switched back from the LaTeX view), it waits for them.
  useEffect(() => {
    pendingSpot.current = spot ?? null;
    if (spot) showSpot(spot);
  }, [spot]);

  return (
    <div
      className="pdf-preview"
      ref={scroller}
      onDoubleClick={(e) => {
        const box = (e.target as HTMLElement).closest<HTMLElement>(".pdf-page-box");
        if (!box || !onLocate) return;
        const rect = box.getBoundingClientRect();
        const scale = rect.width / Number(box.dataset.ptWidth);
        onLocate(Number(box.dataset.page), (e.clientX - rect.left) / scale, (e.clientY - rect.top) / scale);
      }}
    >
      {!pdf && <p className="muted pdf-empty">{t("dialogs.pdfEmpty")}</p>}
      {/* Filled imperatively; React never renders children into it. */}
      <div className="pdf-pages" ref={container} />
    </div>
  );
}
