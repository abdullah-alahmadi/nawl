# Contributing to Nawl

**English** · [العربية](#المساهمة-في-نول)

Thank you for helping. Nawl is young, so bug reports, ideas and fixes all make a real
difference. Arabic support comes first in this project, and reports about Arabic
documents are especially welcome, in Arabic or in English.

## Reporting a bug

Open an [issue](https://github.com/abdullah-alahmadi/nawl/issues/new/choose) and pick
**Bug report**. The most useful reports have:

- what you did, step by step, and what happened instead of what you expected;
- your Nawl version (shown in the installer's name) and your system (Windows or Ubuntu
  version);
- the TeX engine (Settings → Preview): the built-in Tectonic or an installed TeX;
- the document, if you can share it (**File → Save as single file…** gives one `.nawl`
  with its images), or a small one that shows the problem;
- for wrong output, a screenshot of the editor and the PDF next to each other.

Found a security problem? Please report it privately as [SECURITY.md](SECURITY.md)
explains, not in a public issue.

## Asking for a feature or a template

Pick **Feature request** for something Nawl should do, or **Template request** for a
journal, conference, university or official document layout you need. For a template,
link the publisher's author guidelines or attach the institution's rules; Nawl's
templates are made from those.

## Working on the code

The [README](README.md#building-from-source) explains how to build Nawl. In short, on
Ubuntu or in Git Bash on Windows:

```bash
npm install
scripts/fetch-tectonic.sh            # add x86_64-pc-windows-msvc on Windows
npm run tauri dev
```

How the code is laid out:

| Folder | What it holds |
|---|---|
| `crates/core` | The document and template model, validation; TypeScript types are generated into `src/bindings/` by `cargo test` |
| `crates/codegen` | Template → `.cls`, document → `.tex` |
| `crates/compile` | Runs Tectonic or an installed TeX |
| `crates/docx` | Word export |
| `crates/cli` | The command-line tool |
| `src-tauri` | The app's backend (commands in `lib.rs`) |
| `src` | The frontend: `App.tsx`, `editor/` (TipTap nodes), `designer/`, dialogs |
| `templates`, `examples` | Built-in templates and sample documents |

### Before sending a pull request

```bash
cargo test --workspace      # Rust tests; also regenerates src/bindings
npx tsc --noEmit -p .       # frontend type check
```

The same checks run on GitHub for every pull request. If you changed generated LaTeX,
also run the real compiles (they need the bundled engine):

```bash
NAWL_TECTONIC=$PWD/src-tauri/binaries/tectonic-x86_64-unknown-linux-gnu cargo test --workspace -- --ignored
```

### Guidelines

- **Arabic first.** Check how a change behaves in a right-to-left Arabic document as well
  as an English one: direction, fonts, digits and numbering.
- **Generated LaTeX must compile** with Tectonic (XeTeX) and with pdfLaTeX. Add a test,
  and an ignored compile test for a new kind of document.
- **Old files keep opening.** Documents are `.nawl` and projects `.nawlproj`; files from
  Texloom (`.loom`, `.loomproj`, `"format": "texloom"`) must still open.
- **Never commit a publisher's kit.** Kits often contain licensed fonts;
  `src-tauri/assets/` is ignored for that reason.
- **Rust:** errors as `AppError`, blocking work in `spawn_blocking`, and comments that say
  why. On Windows, external programs start through `nawl_compile::tool()` so no console
  window appears.
- **Linux:** the app runs on WebKitGTK there, so check focus, spacing and spell checking
  if you change the editor.
- Keep pull requests focused on one change, and describe what you checked and what you
  couldn't.

## License

Nawl is licensed under the [Apache License 2.0](LICENSE). By contributing, you agree
that your contribution is licensed under the same terms.

---

<div dir="rtl">

## المساهمة في نول

شكرًا لمساعدتك. نول برنامج ناشئ، ولبلاغات الأخطاء والأفكار والإصلاحات أثر حقيقي فيه.
العربية هي الأولوية الأولى في هذا المشروع، ونرحب خاصة بالبلاغات عن المستندات العربية،
بالعربية أو بالإنجليزية.

### الإبلاغ عن خطأ

افتح [بلاغًا](https://github.com/abdullah-alahmadi/nawl/issues/new/choose) واختر
**Bug report**. أنفع البلاغات ما يتضمن:

- ما فعلته خطوة بخطوة، وما حدث بدلًا مما توقعته؛
- إصدار نول (يظهر في اسم ملف التثبيت) ونظامك (إصدار ويندوز أو أوبونتو)؛
- محرك TeX (الإعدادات ← المعاينة): Tectonic المدمج أو TeX مثبت؛
- المستند إن أمكن مشاركته (**ملف ← حفظ كملف واحد…** يعطي ملف `.nawl` واحدًا بصوره)،
  أو مستندًا صغيرًا يُظهر المشكلة؛
- للمخرجات الخاطئة: صورة للمحرر وملف PDF متجاورين.

وإن وجدت مشكلة أمنية فأبلغ عنها سرًّا كما يشرح [SECURITY.md](SECURITY.md)، لا في بلاغ عام.

### طلب ميزة أو قالب

اختر **Feature request** لشيء ينبغي أن يفعله نول، أو **Template request** لتخطيط مجلة أو
مؤتمر أو جامعة أو خطاب رسمي تحتاجه. وللقالب، أرفق رابط إرشادات المؤلفين لدى الناشر أو
قواعد الجهة؛ فمنها تُصنع قوالب نول.

### العمل على الكود

يشرح [ملف README](README.md#building-from-source) طريقة البناء، والتفاصيل التقنية
أعلاه بالإنجليزية. قبل إرسال طلب دمج شغّل `cargo test --workspace` و
`npx tsc --noEmit -p .`، وتحقق من سلوك التعديل في مستند عربي من اليمين إلى اليسار وفي
مستند إنجليزي.

### الترخيص

نول مرخّص برخصة [Apache License 2.0](LICENSE)، وبمساهمتك توافق على ترخيص مساهمتك
بالشروط نفسها.

</div>
