# Security policy

**English** · [العربية](#سياسة-الأمان)

## Supported versions

Nawl is in early preview. Security fixes go into the latest release only.

| Version | Supported |
|---|---|
| 0.1.x (latest) | Yes |
| Older builds, and Texloom | No |

## Reporting a vulnerability

**Please don't report security problems in a public issue.** Report them privately
through GitHub: on the repository page open the **Security** tab and choose
**Report a vulnerability**. Only the maintainer sees the report.

If that button isn't there, open an issue that says only that you have a security report
and asks how to send it, without any details.

Please include what an attacker could do, the steps or a sample file that shows it, and
the Nawl version and system you tested on.

Nawl is maintained by one person, in spare time. You can expect a first answer within
about a week. Once a fix is released, the report can be published, with credit to you if
you want it.

## What counts

Nawl is made to open documents, templates and publisher kits that other people share,
so problems in that path matter most:

- a `.nawl` or `.nawlproj` file, a template, a template package or a publisher's kit
  (.zip) that reads or writes files outside its own folder, or runs programs;
- LaTeX in a document or template that escapes the TeX run. Compiles never allow shell
  escape: the bundled Tectonic runs with `--untrusted`, and an installed TeX with
  `-no-shell-escape`. A way around that is a vulnerability;
- anything in the app (the editor, the preview, Word export) that runs code from a
  document.

Not in scope: problems in TeX engines, TeX packages or fonts themselves (please report
those to their projects), and setups where you changed your own TeX installation to
allow shell escape.

---

<div dir="rtl">

## سياسة الأمان

نول في مرحلة تجريبية مبكرة، وإصلاحات الأمان تدخل في أحدث إصدار فقط.

**لا تُبلغ عن مشكلات الأمان في بلاغ عام.** أبلغ عنها سرًّا عبر GitHub: افتح تبويب
**Security** في صفحة المستودع واختر **Report a vulnerability**، ولا يرى البلاغ إلا
المشرف على المشروع. وإن لم يظهر هذا الزر، فافتح بلاغًا عامًّا تقول فيه فقط إن لديك بلاغًا
أمنيًّا وتسأل عن طريقة إرساله، دون أي تفاصيل.

أرفق ما يستطيع المهاجم فعله، والخطوات أو ملفًا نموذجيًّا يُظهر المشكلة، وإصدار نول
ونظامك. يشرف على نول شخص واحد في وقت فراغه، فتوقّع الرد الأول خلال أسبوع تقريبًا.

أهم ما يدخل في النطاق: مستند أو قالب أو حزمة ناشر يقرأ ملفات أو يكتبها خارج مجلده أو
يشغّل برامج، أو كود LaTeX يتجاوز تشغيل TeX؛ فالبناء لا يسمح أبدًا بتشغيل أوامر النظام
(shell escape).

</div>
