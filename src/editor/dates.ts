// Date fields on screen, written as the PDF writes them (crates/core/src/date.rs):
// "14 May 2026" / "١٤ مايو ٢٠٢٦", and the Hijri date (Umm al-Qura) for {field_hijri}.
import type { Metadata, Template } from "../api";

const MONTHS_EN = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const MONTHS_AR = ["يناير", "فبراير", "مارس", "أبريل", "مايو", "يونيو", "يوليو", "أغسطس", "سبتمبر", "أكتوبر", "نوفمبر", "ديسمبر"];
const HIJRI_EN = ["Muharram", "Safar", "Rabi al-Awwal", "Rabi al-Thani", "Jumada al-Ula", "Jumada al-Akhirah", "Rajab", "Shaban", "Ramadan", "Shawwal", "Dhu al-Qadah", "Dhu al-Hijjah"];
const HIJRI_AR = ["محرم", "صفر", "ربيع الأول", "ربيع الآخر", "جمادى الأولى", "جمادى الآخرة", "رجب", "شعبان", "رمضان", "شوال", "ذو القعدة", "ذو الحجة"];

export const HIJRI_SUFFIX = "_hijri";

type Lang = "english" | "arabic";
type Digits = "western" | "arabicIndic";

/** `YYYY-MM-DD` (what the date picker stores) as year, month, day; null otherwise. */
export function parseIso(value: string): [number, number, number] | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value.trim());
  if (!m) return null;
  const [y, mo, d] = [Number(m[1]), Number(m[2]), Number(m[3])];
  const date = new Date(Date.UTC(y, mo - 1, d));
  return date.getUTCMonth() === mo - 1 && date.getUTCDate() === d ? [y, mo, d] : null;
}

function digits(s: string, d: Digits): string {
  return d === "arabicIndic" ? s.replace(/[0-9]/g, (c) => String.fromCharCode(0x0660 + Number(c))) : s;
}

/** A date field as text; a value that isn't an ISO date stays as typed. */
export function formatDate(value: string, lang: Lang, d: Digits): string {
  const p = parseIso(value);
  if (!p) return value;
  const months = lang === "arabic" ? MONTHS_AR : MONTHS_EN;
  return digits(`${p[2]} ${months[p[1] - 1]} ${p[0]}`, d);
}

/** The same day in the Hijri calendar, or null when the value isn't an ISO date. */
export function formatHijri(value: string, lang: Lang, d: Digits): string | null {
  const p = parseIso(value);
  if (!p) return null;
  try {
    const parts = new Intl.DateTimeFormat("en-u-ca-islamic-umalqura-nu-latn", { day: "numeric", month: "numeric", year: "numeric", timeZone: "UTC" })
      .formatToParts(new Date(Date.UTC(p[0], p[1] - 1, p[2])));
    const part = (type: string) => Number(parts.find((x) => x.type === type)?.value ?? NaN);
    const [hy, hm, hd] = [part("year"), part("month"), part("day")];
    if ([hy, hm, hd].some(Number.isNaN)) return null;
    return lang === "arabic" ? digits(`${hd} ${HIJRI_AR[hm - 1]} ${hy}هـ`, d) : `${hd} ${HIJRI_EN[hm - 1]} ${hy} AH`;
  } catch {
    return null;
  }
}

/** Metadata with date fields written out and their Hijri forms added, for showing
 *  placeholders as they will print. */
export function withDates(t: Template, metadata: Metadata): Metadata {
  const lang: Lang = t.language?.main === "arabic" ? "arabic" : "english";
  const d: Digits = t.language?.digits ?? "western";
  const out: Metadata = { ...metadata };
  for (const f of t.fields) {
    const v = metadata[f.id];
    if (f.kind !== "date" || typeof v !== "string" || !v.trim()) continue;
    out[f.id] = formatDate(v, lang, d);
    const hijri = formatHijri(v, lang, d);
    if (hijri) out[f.id + HIJRI_SUFFIX] = hijri;
  }
  return out;
}
