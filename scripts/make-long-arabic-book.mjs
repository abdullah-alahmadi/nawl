// Writes a long Arabic book (about 1000 pages by default) to test how Nawl copes
// with large documents: the editor, LaTeX generation and compiling. The text is
// assembled from a pool of sentences, with sections, equations, tables, figures,
// lists, citations and English terms in realistic proportions; the same arguments
// always give the same book.
//
//   node scripts/make-long-arabic-book.mjs [chapters] [out]
//   (defaults: 85 chapters, about 1000 pages, examples/arabic-book-long.nawl, which git ignores)
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const chapters = Number(process.argv[2] ?? 85);
const out = process.argv[3] ?? join(root, "examples", "arabic-book-long.nawl");

// A small deterministic random generator (mulberry32).
let seed = 1447;
const random = () => {
  seed = (seed + 0x6d2b79f5) | 0;
  let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
  t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
  return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
};
const pick = (list) => list[Math.floor(random() * list.length)];
const between = (a, b) => a + Math.floor(random() * (b - a + 1));
const indic = (n) => String(n).replace(/\d/g, (d) => "٠١٢٣٤٥٦٧٨٩"[d]);

const SENTENCES = [
  "اهتمّ العلماء العرب منذ القرن الثالث الهجري بترجمة كتب الطب والفلك والرياضيات ونقدها والإضافة إليها.",
  "وكانت بيت الحكمة في بغداد مركزًا للترجمة والتأليف، اجتمع فيه المترجمون والنسّاخ من مختلف الأقاليم.",
  "ولم تقتصر جهودهم على النقل، بل صحّحوا أخطاء من سبقهم وأجروا التجارب وسجّلوا الملاحظات بدقة.",
  "ويُعدّ الخوارزمي من أوائل من وضعوا أسس علم الجبر، ونُسبت إليه كلمة الخوارزمية في لغات كثيرة.",
  "وقد ألّف ابن الهيثم في البصريات كتابًا غيّر فهم الناس لطبيعة الرؤية وانتقال الضوء.",
  "وتدلّ المخطوطات المحفوظة في المكتبات العامة على اتساع حركة التأليف وتنوّع موضوعاتها.",
  "ويواجه الباحث في المخطوطات صعوبات كثيرة، منها تفرّق النسخ واختلافها وضياع بعض أجزائها.",
  "ولذلك يلجأ المحققون إلى المقابلة بين النسخ المتعددة لإثبات النص الأقرب إلى ما كتبه المؤلف.",
  "ومن أهم ما يعين على ذلك دراسة الخطوط وأنواع الورق والأحبار وتواريخ النسخ المدوّنة في خواتيم الكتب.",
  "وتطوّرت صناعة الورق في العالم الإسلامي تطورًا كبيرًا، فانتشرت مصانعه من سمرقند إلى الأندلس.",
  "وأسهم انتشار الورق في خفض تكلفة الكتب، فاتسعت دائرة القرّاء وكثرت المكتبات الخاصة والعامة.",
  "وكان للوراقين دور مهم في نسخ الكتب وبيعها، وكانت أسواقهم ملتقى للعلماء وطلاب العلم.",
  "وتشير كتب التراجم إلى أن بعض العلماء كانوا ينسخون كتبهم بأيديهم ويهدونها إلى تلاميذهم.",
  "ويمكن اليوم الاستفادة من التقنيات الرقمية في حفظ المخطوطات وإتاحتها للباحثين في كل مكان.",
  "فالتصوير عالي الدقة يحفظ تفاصيل الخط والزخرفة، ويقلّل الحاجة إلى تداول النسخ الأصلية.",
  "كما تساعد أنظمة التعرّف الآلي على النصوص في البحث داخل المخطوطات المصوّرة وفهرستها.",
  "غير أن الخطوط العربية المتصلة وكثرة الزخارف تجعل التعرّف الآلي عليها أصعب منه في الخطوط اللاتينية.",
  "ويحتاج تدريب النماذج الحاسوبية على هذه الخطوط إلى مدوّنات كبيرة مصنّفة يدويًا بعناية.",
  "وقد بدأت بعض الجامعات والمراكز البحثية في بناء هذه المدوّنات وإتاحتها للباحثين مجانًا.",
  "وتبقى مهمة المحقق أساسية، إذ لا تغني الآلة عن خبرته في قراءة النص وفهم سياقه.",
  "ويُلاحظ أن المصطلحات العلمية تغيّرت دلالاتها عبر القرون، فلا يصحّ حملها على معانيها المعاصرة.",
  "ومن هنا تأتي أهمية المعاجم التاريخية التي تتتبّع تطوّر معاني الألفاظ عبر الزمن.",
  "وقد عُني اللغويون العرب بجمع اللغة وتدوينها في معاجم كبيرة رُتّبت بطرق مختلفة.",
  "فمنهم من رتّب الألفاظ على مخارج الحروف، ومنهم من رتّبها على أواخر الكلمات أو أوائلها.",
  "ويعكس هذا التنوّع في مناهج الترتيب تطوّر التفكير المعجمي وحاجات القرّاء في كل عصر.",
  "وفي ميدان الفلك، صنع العلماء الأسطرلابات وبنوا المراصد لرصد حركات الكواكب وتحديد المواقيت.",
  "وكانت معرفة اتجاه القبلة ومواقيت الصلاة من أهم الدوافع إلى العناية بعلم الفلك والهندسة.",
  "وأُلّفت الأزياج التي تتضمّن جداول لحركات الشمس والقمر والكواكب، واستُعملت قرونًا عديدة.",
  "ويُظهر تحليل هذه الجداول دقة كبيرة في الحسابات بالنظر إلى الأدوات المتاحة في ذلك الزمان.",
  "وفي الطب، جمع العلماء خبرات من سبقهم وأضافوا إليها ملاحظاتهم السريرية الدقيقة.",
  "وأُنشئت البيمارستانات التي جمعت بين العلاج والتعليم، وكان لها نظام إداري محكم.",
  "وكانت هذه المستشفيات تضم أقسامًا متخصصة، وصيدليات، ومكتبات يرجع إليها الأطباء والطلاب.",
  "ولا يزال كثير من هذا التراث مخطوطًا لم يُحقَّق، ينتظر الباحثين المتخصصين في مختلف العلوم.",
  "ويتطلّب ذلك تعاونًا بين المؤسسات العلمية لتبادل صور المخطوطات وتوحيد معايير الفهرسة.",
  "ومن المفيد أن تُدرَّس مناهج التحقيق في الجامعات بوصفها علمًا له قواعده وأدواته.",
]
const TERMS = ["OCR", "Unicode", "XML", "TEI", "metadata", "IIIF", "machine learning", "digital humanities"];
const SECTION_TITLES = ["تمهيد", "المفهوم والنشأة", "المصادر الأولى", "مناهج البحث", "نماذج تطبيقية", "التحديات والحلول", "الأدوات الرقمية", "خلاصة الفصل", "قضايا للنقاش", "مقارنات وتحليل"];
const CHAPTER_TOPICS = ["الترجمة والنقل", "صناعة الورق والكتاب", "المخطوطات وتحقيقها", "علوم اللغة والمعاجم", "الفلك والمواقيت", "الطب والبيمارستانات", "الرياضيات والجبر", "البصريات والضوء", "المكتبات والوراقون", "التراث الرقمي"];
const FIGURES = ["figures/thesis-pipeline.png", "figures/irrigation-system.png", "figures/thesis-f1.png", "figures/air-pm25.png"];
const EQUATIONS = [
  String.raw`x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}`,
  String.raw`\sum_{k=1}^{n} k = \frac{n(n+1)}{2}`,
  String.raw`\tan\theta = \frac{\sin\theta}{\cos\theta}`,
  String.raw`A = \pi r^2`,
  String.raw`\int_0^1 x^2\,dx = \frac{1}{3}`,
];
const FAMILIES = ["الأحمدي", "القحطاني", "العتيبي", "الزهراني", "الغامدي", "الحربي", "الشهري", "الدوسري", "المطيري", "العمري"];
const GIVEN = ["عبدالله", "سارة", "خالد", "نورة", "فهد", "منى", "سعد", "ريم", "محمد", "هند"];
const EN_FAMILIES = ["Smith", "Brown", "Garcia", "Müller", "Rossi", "Martin", "Kowalski", "Haddad", "Nakamura", "Silva"];
const EN_GIVEN = ["John", "Maria", "David", "Anna", "Peter", "Laila", "Omar", "Elena", "James", "Sofia"];

// A bibliography of 60 entries (two thirds Arabic), marked as test data.
const keys = [];
let bib = "";
for (let i = 1; i <= 60; i++) {
  const arabic = i % 3 !== 0;
  const key = `ref${i}`;
  keys.push(key);
  const year = 1980 + (i * 7) % 44;
  if (arabic) {
    bib += `@book{${key},\n  author = {${pick(FAMILIES)}, ${pick(GIVEN)}},\n  title = {${pick(CHAPTER_TOPICS)}: دراسة في المصادر (مرجع اختبار ${indic(i)})},\n  publisher = {دار النشر العلمي},\n  address = {الرياض},\n  year = {${year}}\n}\n\n`;
  } else {
    bib += `@article{${key},\n  author = {${pick(EN_FAMILIES)}, ${pick(EN_GIVEN)} and ${pick(EN_FAMILIES)}, ${pick(EN_GIVEN)}},\n  title = {Test Reference ${i} on Arabic Manuscripts},\n  journal = {Journal of Test Studies},\n  volume = {${between(1, 40)}},\n  pages = {${between(1, 200)}--${between(201, 400)}},\n  year = {${year}}\n}\n\n`;
  }
}

const text = (t, marks) => (marks ? { type: "text", text: t, marks } : { type: "text", text: t });
function paragraph() {
  const content = [];
  const n = between(4, 7);
  for (let s = 0; s < n; s++) {
    let sentence = pick(SENTENCES);
    if (random() < 0.12) sentence = sentence.replace(/\.$/, "") + ` (${pick(TERMS)}).`;
    content.push(text((s ? " " : "") + sentence));
    if (random() < 0.08) content.push(text(" "), { type: "inlineMath", attrs: { latex: "a^2 + b^2 = c^2" } }, text("."));
  }
  if (random() < 0.35) {
    content.push(text(" "), { type: "citation", attrs: { keys: [pick(keys)] } }, text("."));
  }
  return { type: "paragraph", content };
}
const cell = (kind, t) => ({ type: kind, attrs: { colspan: 1, rowspan: 1, colwidth: null }, content: [{ type: "paragraph", content: [text(t)] }] });

let figureNo = 0, tableNo = 0, equationNo = 0;
const blocks = [];
for (let c = 1; c <= chapters; c++) {
  const topic = CHAPTER_TOPICS[(c - 1) % CHAPTER_TOPICS.length];
  blocks.push({ type: "heading", attrs: { level: 1, id: `ch${c}`, numbered: true }, content: [text(`${topic} (${indic(Math.ceil(c / CHAPTER_TOPICS.length))})`)] });
  for (let p = 0; p < 2; p++) blocks.push(paragraph());
  for (let s = 0; s < 6; s++) {
    blocks.push({ type: "heading", attrs: { level: 2, id: `ch${c}s${s}`, numbered: true }, content: [text(SECTION_TITLES[(c + s) % SECTION_TITLES.length])] });
    const paragraphs = between(5, 8);
    for (let p = 0; p < paragraphs; p++) blocks.push(paragraph());
    const r = random();
    if (r < 0.25) {
      equationNo++;
      blocks.push({ type: "equation", attrs: { latex: pick(EQUATIONS), numbered: true, id: `eq${equationNo}` } });
      blocks.push({ type: "paragraph", content: [text("وتوضّح المعادلة "), { type: "ref", attrs: { target: `eq${equationNo}` } }, text(" العلاقة السابقة.")] });
    } else if (r < 0.45) {
      tableNo++;
      const rows = [["القرن", "عدد المخطوطات", "النسبة"]];
      for (let k = 0; k < between(3, 6); k++) rows.push([`القرن ${indic(3 + k)}هـ`, indic(between(50, 900)), `${indic(between(1, 40))}٫${indic(between(0, 9))}٪`]);
      blocks.push({ type: "tableFigure", attrs: { id: `tab${tableNo}` }, content: [
        { type: "caption", content: [text(`توزيع المخطوطات حسب القرون (${indic(tableNo)})`)] },
        { type: "table", content: rows.map((row, i) => ({ type: "tableRow", content: row.map((t) => cell(i === 0 ? "tableHeader" : "tableCell", t)) })) },
      ] });
    } else if (r < 0.55) {
      figureNo++;
      blocks.push({ type: "figure", attrs: { src: FIGURES[figureNo % FIGURES.length], widthPercent: 80, id: `fig${figureNo}` }, content: [{ type: "caption", content: [text(`شكل توضيحي رقم ${indic(figureNo)}`)] }] });
    } else if (r < 0.7) {
      blocks.push({ type: "bulletList", content: [1, 2, 3].map(() => ({ type: "listItem", content: [{ type: "paragraph", content: [text(pick(SENTENCES))] }] })) });
    }
  }
}

// Template and metadata from the built-in book template.
const template = JSON.parse(readFileSync(join(root, "templates", "arabic-book", "template.json"), "utf8"));
const doc = {
  format: "texloom",
  version: 1,
  template,
  templatePath: null,
  metadata: {
    title: "تاريخ العلوم والمخطوطات العربية",
    subtitle: "كتاب اختبار طويل لنول",
    author: "مؤلف تجريبي",
    publisher: "دار المعرفة",
    place: "الرياض",
    year: "١٤٤٧هـ",
    preface: "هذا كتاب طويل مُولَّد آليًا لاختبار قدرة نول على التعامل مع المستندات الكبيرة. نصّه مركّب من جمل مكررة، ومراجعه بيانات اختبار وليست مراجع حقيقية.",
  },
  bibliography: bib,
  doc: { type: "doc", content: blocks },
};
writeFileSync(out, JSON.stringify(doc));
console.log(`${out}: ${chapters} chapters, ${blocks.length} blocks, ${figureNo} figures, ${tableNo} tables, ${equationNo} equations, ${(JSON.stringify(doc).length / 1e6).toFixed(1)} MB`);
