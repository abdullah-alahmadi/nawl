// The instant preview of the designer: the document as the editor draws it, styled
// with the template being designed, updated on every change without a compile. The
// PDF stays the exact version; this shows fonts, spacing, headings and numbering at
// once, while the PDF catches up.
import { useEffect, useRef, useState } from "react";
import type { Editor } from "@tiptap/react";
import type { Template } from "../api";
import { PAGE_WIDTH_PX, templateCss, type DeckInfo } from "../editor/templateStyle";

export function PagePreview({ editor, template, deck }: { editor: Editor | null; template: Template; deck?: DeckInfo }) {
  // The document doesn't change while the template is designed: one snapshot does.
  const [html] = useState(() => editor?.view.dom.innerHTML ?? "");
  const pane = useRef<HTMLDivElement>(null);
  const [zoom, setZoom] = useState(1);

  // Fit the page's width to the pane.
  useEffect(() => {
    const el = pane.current;
    if (!el) return;
    const fit = () => setZoom(Math.min(1, Math.max(0.3, (el.clientWidth - 32) / PAGE_WIDTH_PX)));
    fit();
    const ro = new ResizeObserver(fit);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const deckClass = template.kind === "presentation" ? " deck" : "";
  return (
    <div className="page-preview" ref={pane} aria-label="Page preview">
      <style>{templateCss(template, ".page-preview .page", deck)}</style>
      <div className={`page${deckClass}`} style={{ ["--page-zoom" as string]: zoom }}>
        {/* A static copy of the editor's page: nothing in it can be edited. */}
        <div className="tiptap ProseMirror" inert dangerouslySetInnerHTML={{ __html: html }} />
      </div>
    </div>
  );
}
