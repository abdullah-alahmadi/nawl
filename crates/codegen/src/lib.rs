//! Phase 0 LaTeX generator: walks the editor's ProseMirror JSON directly.
//!
//! This is deliberately untyped and template-less. Phase 1 replaces it with typed
//! `core::Document` / `core::Template` models and a `.cls` generator; until then this
//! proves the editor → LaTeX → PDF pipeline end to end.

use serde_json::Value;

/// Converts a ProseMirror `doc` node into a complete, compilable LaTeX document.
pub fn document_to_latex(doc: &Value) -> String {
    let mut body = String::new();
    blocks(&mut body, doc);
    format!("{PREAMBLE}\\begin{{document}}\n\n{body}\\end{{document}}\n")
}

const PREAMBLE: &str = r"\documentclass[11pt]{article}
\usepackage{fontspec}
\usepackage{amsmath,amssymb}
\usepackage[normalem]{ulem}
\usepackage{enumitem}
\usepackage{hyperref}
";

fn children(node: &Value) -> &[Value] {
    node.get("content").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

fn attr<'a>(node: &'a Value, name: &str) -> Option<&'a Value> {
    node.get("attrs").and_then(|a| a.get(name))
}

fn blocks(out: &mut String, parent: &Value) {
    for node in children(parent) {
        block(out, node);
    }
}

fn block(out: &mut String, node: &Value) {
    match node.get("type").and_then(Value::as_str).unwrap_or("") {
        "paragraph" => {
            let text = inlines(node);
            if !text.trim().is_empty() {
                out.push_str(&text);
                out.push_str("\n\n");
            }
        }
        "heading" => {
            let level = attr(node, "level").and_then(Value::as_u64).unwrap_or(1);
            let cmd = match level {
                1 => "section",
                2 => "subsection",
                3 => "subsubsection",
                _ => "paragraph",
            };
            out.push_str(&format!("\\{cmd}{{{}}}\n\n", inlines(node)));
        }
        "bulletList" => list(out, node, "itemize", None),
        "orderedList" => {
            let start = attr(node, "start").and_then(Value::as_u64).filter(|s| *s != 1);
            list(out, node, "enumerate", start);
        }
        "blockquote" => {
            out.push_str("\\begin{quote}\n");
            blocks(out, node);
            out.push_str("\\end{quote}\n\n");
        }
        "codeBlock" => {
            // `verbatim` can't contain its own end marker; break it if present.
            let code = plain_text(node).replace("\\end{verbatim}", "\\end {verbatim}");
            out.push_str(&format!("\\begin{{verbatim}}\n{code}\n\\end{{verbatim}}\n\n"));
        }
        "horizontalRule" => out.push_str("\\noindent\\rule{\\linewidth}{0.4pt}\n\n"),
        "equation" => {
            let latex = attr(node, "latex").and_then(Value::as_str).unwrap_or("").trim();
            if !latex.is_empty() {
                out.push_str(&format!("\\begin{{equation}}\n{latex}\n\\end{{equation}}\n\n"));
            }
        }
        // Unknown block: keep its content rather than dropping text.
        _ => blocks(out, node),
    }
}

fn list(out: &mut String, node: &Value, env: &str, start: Option<u64>) {
    match start {
        Some(s) => out.push_str(&format!("\\begin{{{env}}}[start={s}]\n")),
        None => out.push_str(&format!("\\begin{{{env}}}\n")),
    }
    for item in children(node) {
        out.push_str("\\item ");
        let mut inner = String::new();
        blocks(&mut inner, item);
        out.push_str(inner.trim_end());
        out.push('\n');
    }
    out.push_str(&format!("\\end{{{env}}}\n\n"));
}

fn inlines(node: &Value) -> String {
    let mut s = String::new();
    for child in children(node) {
        match child.get("type").and_then(Value::as_str).unwrap_or("") {
            "text" => {
                let text = child.get("text").and_then(Value::as_str).unwrap_or("");
                s.push_str(&apply_marks(escape(text), child));
            }
            "hardBreak" => s.push_str("\\newline\n"),
            "inlineMath" => {
                let latex = attr(child, "latex").and_then(Value::as_str).unwrap_or("");
                s.push_str(&format!("${latex}$"));
            }
            _ => s.push_str(&inlines(child)),
        }
    }
    s
}

fn apply_marks(mut text: String, node: &Value) -> String {
    let marks = node.get("marks").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]);
    for mark in marks {
        text = match mark.get("type").and_then(Value::as_str).unwrap_or("") {
            "bold" => format!("\\textbf{{{text}}}"),
            "italic" => format!("\\emph{{{text}}}"),
            "underline" => format!("\\uline{{{text}}}"),
            "strike" => format!("\\sout{{{text}}}"),
            "code" => format!("\\texttt{{{text}}}"),
            "link" => {
                let href = attr(mark, "href").and_then(Value::as_str).unwrap_or("");
                format!("\\href{{{}}}{{{text}}}", escape_url(href))
            }
            _ => text,
        };
    }
    text
}

fn plain_text(node: &Value) -> String {
    let mut s = String::new();
    for child in children(node) {
        if let Some(t) = child.get("text").and_then(Value::as_str) {
            s.push_str(t);
        } else {
            s.push_str(&plain_text(child));
        }
    }
    s
}

/// Escapes characters that are special in LaTeX text mode.
pub fn escape(text: &str) -> String {
    let mut s = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => s.push_str("\\textbackslash{}"),
            '{' => s.push_str("\\{"),
            '}' => s.push_str("\\}"),
            '$' => s.push_str("\\$"),
            '&' => s.push_str("\\&"),
            '%' => s.push_str("\\%"),
            '#' => s.push_str("\\#"),
            '_' => s.push_str("\\_"),
            '~' => s.push_str("\\textasciitilde{}"),
            '^' => s.push_str("\\textasciicircum{}"),
            _ => s.push(c),
        }
    }
    s
}

fn escape_url(url: &str) -> String {
    url.replace('\\', "").replace('%', "\\%").replace('#', "\\#")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn escapes_specials() {
        assert_eq!(escape(r"50% & $5 #1 a_b {x} ~ ^ \"), r"50\% \& \$5 \#1 a\_b \{x\} \textasciitilde{} \textasciicircum{} \textbackslash{}");
    }

    #[test]
    fn converts_basic_document() {
        let doc = json!({"type": "doc", "content": [
            {"type": "heading", "attrs": {"level": 1}, "content": [{"type": "text", "text": "Intro"}]},
            {"type": "paragraph", "content": [
                {"type": "text", "text": "Hello "},
                {"type": "text", "text": "world", "marks": [{"type": "bold"}]}
            ]},
            {"type": "bulletList", "content": [
                {"type": "listItem", "content": [{"type": "paragraph", "content": [{"type": "text", "text": "one"}]}]}
            ]},
            {"type": "equation", "attrs": {"latex": "E = mc^2"}}
        ]});
        let tex = document_to_latex(&doc);
        assert!(tex.contains("\\section{Intro}"));
        assert!(tex.contains("Hello \\textbf{world}"));
        assert!(tex.contains("\\begin{itemize}\n\\item one\n\\end{itemize}"));
        assert!(tex.contains("\\begin{equation}\nE = mc^2\n\\end{equation}"));
    }
}
