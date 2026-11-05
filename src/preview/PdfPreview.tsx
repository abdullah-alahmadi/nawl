// Renders PDF bytes to canvases with pdf.js. New renders are drawn off-screen and
// swapped in when complete, so the preview doesn't flash blank while recompiling.
import { useEffect, useRef } from "react";
import * as pdfjs from "pdfjs-dist";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";

pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;

export function PdfPreview({ pdf }: { pdf: ArrayBuffer | null }) {
  const container = useRef<HTMLDivElement>(null);

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
      const cssWidth = Math.max(200, target.clientWidth - 32);

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
        pages.appendChild(canvas);
      }
      // Swapping in one synchronous step keeps the scroll position of the pane.
      if (!cancelled) target.replaceChildren(...pages.childNodes);
      await task.destroy();
    })().catch((e) => console.error("PDF render failed", e));

    return () => {
      cancelled = true;
    };
  }, [pdf]);

  return (
    <div className="pdf-preview">
      {!pdf && <p className="muted">The PDF preview appears here after the first compile.</p>}
      {/* Filled imperatively; React never renders children into it. */}
      <div className="pdf-pages" ref={container} />
    </div>
  );
}
