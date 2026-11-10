// Arabic mathematics on the page: formulas written right to left, as the PDF sets
// them (crates/codegen/src/armath.rs, which this mirrors rule for rule). The formula
// is drawn left to right and mirrored as a whole (CSS); every letter, word and number
// is marked so it's mirrored back and reads normally. What Arabic notation mirrors
// (the integral, sum and root, brackets, <, →) is left alone.
//
// Marks are CSS classes MathLive puts on its markup: `tl-v` a variable (in the
// variables' font), `tl-w` a word or number (the text font), `tl-b` a symbol that
// stays the right way round (a Greek letter).

/** Latin function commands and their Arabic names; true when limits go below. */
const FUNCTIONS: Record<string, [string, boolean]> = {
  sin: ["جا", false],
  cos: ["جتا", false],
  tan: ["ظا", false],
  cot: ["ظتا", false],
  sec: ["قا", false],
  csc: ["قتا", false],
  sinh: ["جاز", false],
  cosh: ["جتاز", false],
  tanh: ["ظاز", false],
  log: ["لو", false],
  ln: ["لط", false],
  lim: ["نهـــا", true],
};

/** Function names typed in Arabic (without their kashida), longest first. */
const ARABIC_NAMES: [string, string, boolean][] = [
  ["جتاز", "جتاز", false],
  ["جتا", "جتا", false],
  ["ظتا", "ظتا", false],
  ["قتا", "قتا", false],
  ["جاز", "جاز", false],
  ["ظاز", "ظاز", false],
  ["نها", "نهـــا", true],
  ["جا", "جا", false],
  ["ظا", "ظا", false],
  ["قا", "قا", false],
  ["لو", "لو", false],
  ["لط", "لط", false],
  ["مجـ", "مجـ", true],
];

const OPERATORS: Record<string, boolean> = {
  max: true, min: true, sup: true, inf: true, det: true, gcd: true, Pr: true, limsup: true, liminf: true,
  exp: false, arg: false, deg: false, dim: false, ker: false, hom: false, arcsin: false, arccos: false, arctan: false, coth: false, lg: false,
};

const TURNED_BACK = new Set([
  "alpha", "beta", "gamma", "delta", "epsilon", "varepsilon", "zeta", "eta", "theta", "vartheta", "iota", "kappa", "lambda", "mu", "nu", "xi", "pi", "varpi", "rho",
  "varrho", "sigma", "varsigma", "tau", "upsilon", "phi", "varphi", "chi", "psi", "omega", "Gamma", "Delta", "Theta", "Lambda", "Xi", "Pi", "Sigma", "Upsilon", "Phi",
  "Psi", "Omega", "ell", "hbar", "imath", "jmath", "partial", "Re", "Im", "aleph", "wp", "emptyset", "varnothing", "checkmark",
]);
const ALPHABETS = new Set(["mathrm", "mathbf", "mathit", "mathsf", "mathtt", "mathbb", "mathcal", "mathfrak", "mathscr", "boldsymbol", "mathnormal"]);
const TEXT = new Set(["text", "textrm", "textbf", "textit", "textsf", "texttt", "textnormal", "mbox"]);
const RAW = new Set(["hspace", "vspace", "kern", "mkern", "hskip", "mskip", "rule", "raisebox", "label", "ref", "eqref", "begin", "end", "color", "mathcolor", "tag"]);
const WITH_COLUMNS = new Set(["array", "subarray", "alignedat", "tabular"]);

const isArabicLetter = (c: string) => (c >= "ء" && c <= "ي") || (c >= "ٮ" && c <= "ۓ");
const isArabicMark = (c: string) => (c >= "ً" && c <= "ٟ") || c === "ٰ";
const isDigit = (c: string) => (c >= "0" && c <= "9") || (c >= "٠" && c <= "٩") || (c >= "۰" && c <= "۹");
const isAsciiLetter = (c: string) => /^[A-Za-z]$/.test(c);
const KASHIDA = "ـ";

const variable = (s: string) => `{\\class{tl-v}{\\text{${s}}}}`;
const word = (s: string) => `\\class{tl-w}{\\text{${s}}}`;
const back = (s: string) => `\\class{tl-b}{${s}}`;

/** A function name at the start of `run`: the characters it takes, what is printed,
 *  and whether limits go below. Kashidas inside the name don't count. */
function arabicName(run: string[]): [number, string, boolean] | null {
  for (const [name, shown, limits] of ARABIC_NAMES) {
    const wanted = [...name];
    let taken = 0;
    let matched = 0;
    while (matched < wanted.length && taken < run.length) {
      const c = run[taken];
      if (c === wanted[matched]) matched++;
      else if (!(c === KASHIDA && matched > 0)) break;
      taken++;
    }
    if (matched === wanted.length) {
      while (run[taken] === KASHIDA) taken++;
      return [taken, shown, limits];
    }
  }
  return null;
}

class Writer {
  i = 0;
  out = "";
  private afterScript = false;
  constructor(
    private chars: string[],
    private indic: boolean,
  ) {}

  run(inGroup: boolean) {
    while (this.i < this.chars.length) {
      const c = this.chars[this.i];
      const script = this.afterScript;
      this.afterScript = false;
      if (c === "}" && inGroup) return;
      if (c === "{") {
        this.i++;
        this.out += "{";
        this.run(true);
        this.out += "}";
        this.i++;
      } else if (c === "\\") this.command();
      else if (c === "^" || c === "_") {
        this.out += c;
        this.i++;
        this.afterScript = true;
      } else if (isDigit(c)) this.number(script);
      else if (isArabicLetter(c)) this.arabic(script);
      else if (isAsciiLetter(c)) {
        this.out += `{${back(c)}}`;
        this.i++;
      } else if (c === "," || c === "،") this.punctuation("،");
      else if (c === ";" || c === "؛") this.punctuation("؛");
      else if (c === "?" || c === "؟") this.punctuation("؟");
      else if (c === "/") {
        this.out += `{${back("/")}}`;
        this.i++;
      } else {
        if (/\s/.test(c)) this.afterScript = script;
        this.out += c;
        this.i++;
      }
    }
  }

  private punctuation(mark: string) {
    this.out += `\\mathpunct{${word(mark)}}`;
    this.i++;
  }

  private number(oneDigit: boolean) {
    let text = "";
    while (this.i < this.chars.length) {
      const c = this.chars[this.i];
      const next = this.chars[this.i + 1];
      if (isDigit(c)) text += this.indic && c >= "0" && c <= "9" ? String.fromCharCode(0x0660 + Number(c)) : c;
      else if ((c === "." || c === "٫") && next !== undefined && isDigit(next) && !oneDigit) text += this.indic ? "٫" : c;
      else break;
      this.i++;
      if (oneDigit) break;
    }
    this.out += `{${word(text)}}`;
  }

  private arabic(oneLetter: boolean) {
    const start = this.i;
    let end = start;
    while (end < this.chars.length && (isArabicLetter(this.chars[end]) || isArabicMark(this.chars[end]))) end++;
    const run = this.chars.slice(start, end);
    let k = 0;
    while (k < run.length) {
      const rest = run.slice(k);
      const name = arabicName(rest);
      if (name) {
        this.operator(word(name[1]), name[2]);
        k += name[0];
      } else {
        let n = 1;
        while (n < rest.length && isArabicMark(rest[n])) n++;
        if (rest[0] === KASHIDA || rest[n] === KASHIDA) {
          // A kashida after the letter: the rest of the run is one word.
          this.out += `{${word(rest.join(""))}}`;
          k = run.length;
        } else {
          this.out += variable(rest.slice(0, n).join(""));
          k += n;
        }
      }
      if (oneLetter) break;
    }
    this.i = start + k;
  }

  private operator(name: string, limits: boolean) {
    this.out += `\\mathop{${name}}`;
    if (!limits) this.out += "\\nolimits";
  }

  private command() {
    const start = this.i;
    this.i++;
    while (this.i < this.chars.length && isAsciiLetter(this.chars[this.i])) this.i++;
    if (this.i === start + 1) {
      // A control symbol: \{ \, \\ \%
      const symbol = this.chars[this.i];
      if (symbol === undefined) {
        this.out += "\\";
        return;
      }
      this.i++;
      this.out += "%$#&".includes(symbol) ? `{${back(`\\${symbol}`)}}` : `\\${symbol}`;
      return;
    }
    const name = this.chars.slice(start + 1, this.i).join("");
    if (name in FUNCTIONS) this.operator(word(FUNCTIONS[name][0]), FUNCTIONS[name][1]);
    else if (name in OPERATORS) this.operator(back(`\\${name}`), OPERATORS[name]);
    else if (TURNED_BACK.has(name)) this.out += `{${back(`\\${name}`)}}`;
    else if (TEXT.has(name)) {
      const text = this.group();
      if (text !== null) this.out += name === "text" || name === "mbox" ? `{${word(text)}}` : `{\\class{tl-w}{\\${name}{${text}}}}`;
    } else if (ALPHABETS.has(name) || name === "operatorname") {
      const inner = this.group();
      if (inner === null) return;
      const arabic = [...inner].some(isArabicLetter);
      if (name === "operatorname") {
        const limits = this.chars[this.i] === "*";
        if (limits) this.i++;
        this.operator(arabic ? word(inner) : back(`\\mathrm{${inner}}`), limits);
      } else if (arabic) {
        // Arabic letters in another alphabet: plain variables.
        const w = new Writer([...inner], this.indic);
        w.run(false);
        this.out += `{${w.out}}`;
      } else this.out += `{${back(`\\${name}{${inner}}`)}}`;
    } else if (RAW.has(name)) {
      this.out += `\\${name}`;
      const arg = this.group();
      if (arg === null) return;
      this.out += `{${arg}}`;
      if (name === "begin" && WITH_COLUMNS.has(arg)) {
        const columns = this.group();
        if (columns !== null) this.out += `{${columns}}`;
      }
    } else {
      this.out += `\\${name}`;
      // A letter after it would join the name: `\cdot x`.
      if (this.i < this.chars.length && isAsciiLetter(this.chars[this.i])) this.out += " ";
    }
  }

  /** The `{…}` that follows, as it is (without its braces); null when there isn't one. */
  private group(): string | null {
    let j = this.i;
    while (j < this.chars.length && /\s/.test(this.chars[j])) j++;
    if (this.chars[j] !== "{") return null;
    const open = j + 1;
    let depth = 1;
    j = open;
    while (j < this.chars.length) {
      const c = this.chars[j];
      if (c === "\\") j++;
      else if (c === "{") depth++;
      else if (c === "}" && --depth === 0) break;
      j++;
    }
    const end = Math.min(j, this.chars.length);
    this.i = Math.min(end + 1, this.chars.length);
    return this.chars.slice(open, end).join("");
  }
}

/** The formula with every letter, word and number marked to be mirrored back, as
 *  LaTeX for MathLive. `indic`: numbers in Arabic-Indic digits. */
export function arabicMathLatex(latex: string, indic: boolean): string {
  const w = new Writer([...latex], indic);
  w.run(false);
  return w.out;
}

// ---- how the open document writes its formulas

export interface MathSettings {
  /** An Arabic document: only there can formulas be written the Arabic way. */
  arabicDocument: boolean;
  /** Formulas the Arabic way (the template's setting, in an Arabic document). */
  arabic: boolean;
  /** Numbers in Arabic-Indic digits. */
  indic: boolean;
}

let settings: MathSettings = { arabicDocument: false, arabic: false, indic: false };
const listeners = new Set<() => void>();

export const mathSettings = () => settings;

/** The template changed: formulas on the page are drawn again when it matters. */
export function setMathSettings(next: MathSettings) {
  if (next.arabicDocument === settings.arabicDocument && next.arabic === settings.arabic && next.indic === settings.indic) return;
  settings = next;
  listeners.forEach((f) => f());
}

/** Calls `f` when the settings change; returns how to stop. */
export function onMathSettings(f: () => void): () => void {
  listeners.add(f);
  return () => void listeners.delete(f);
}

// ---- Latin formulas into Arabic notation

/** The Arabic letter for each Latin variable, as Arabic schoolbooks write them:
 *  x y z are س ص ع, a b c are ا ب ج, n m k are ن م ك, a function f is د (as the
 *  differential d is). The others take a letter of their own, so two variables
 *  never become one. Arabic has no capitals: X is س too. */
const LETTERS: Record<string, string> = {
  a: "ا", b: "ب", c: "ج", d: "د", e: "ه", f: "د", g: "غ", h: "ح", i: "ت", j: "ي", k: "ك", l: "ل", m: "م",
  n: "ن", o: "ذ", p: "ف", q: "ق", r: "ر", s: "ش", t: "ط", u: "ث", v: "ز", w: "و", x: "س", y: "ص", z: "ع",
};

/** Commands whose argument stays as it is: text, names, lengths, other alphabets. */
const KEPT = new Set([...TEXT, ...RAW, ...ALPHABETS, "operatorname"]);

/** The formula with its Latin variables as Arabic letters (x → س). Commands, text,
 *  Greek letters and digits are untouched; so are letters in `\mathbb{R}` and the like. */
export function toArabicNotation(latex: string): string {
  const chars = [...latex];
  let out = "";
  let i = 0;
  // Copies the `{…}` at `i` (after spaces) as it is.
  const copyGroup = () => {
    let j = i;
    while (j < chars.length && /\s/.test(chars[j])) j++;
    if (chars[j] !== "{") return "";
    let depth = 0;
    let k = j;
    for (; k < chars.length; k++) {
      if (chars[k] === "\\") k++;
      else if (chars[k] === "{") depth++;
      else if (chars[k] === "}" && --depth === 0) break;
    }
    const group = chars.slice(i, k + 1).join("");
    i = Math.min(k + 1, chars.length);
    return group;
  };
  while (i < chars.length) {
    const c = chars[i];
    if (c === "\\") {
      const start = i++;
      while (i < chars.length && isAsciiLetter(chars[i])) i++;
      if (i === start + 1 && i < chars.length) i++; // a control symbol: \{ \,
      const name = chars.slice(start + 1, i).join("");
      out += chars.slice(start, i).join("");
      if (KEPT.has(name)) {
        const group = copyGroup();
        out += group;
        if (name === "begin" && WITH_COLUMNS.has(group.trim().slice(1, -1))) out += copyGroup();
      }
    } else if (isAsciiLetter(c)) {
      out += LETTERS[c.toLowerCase()] ?? c;
      i++;
    } else {
      out += c;
      i++;
    }
  }
  return out;
}

/** Whether a formula with its own `style` (or none) is written the Arabic way. */
export const isArabicMath = (style: string | null | undefined) => settings.arabicDocument && (style ? style === "arabic" : settings.arabic);
