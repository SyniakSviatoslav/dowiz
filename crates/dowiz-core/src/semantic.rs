//! semantic.rs — minimal clean semantic HTML emitter ("no div soup").
//!
//! Item #3 (Instatic): the *reusable* principle is "clean semantic output,
//! no editor machinery" — a document rendered to plain `<h1>/<p>/<a>/<ul>`
//! with no framework runtime, no builder attributes, no `div` soup. This is
//! the same "clean output" law the kernel's glyph layer embodies, applied to
//! markup.
//!
//! Zero-dep, deterministic: a tiny document model + a renderer that emits
//! minimal, escaped, correctly-indented HTML.

use alloc::string::String;
use alloc::vec::Vec;

/// A semantic document node.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// Heading level 1..6 + text.
    Heading(u8, String),
    /// Paragraph.
    Paragraph(String),
    /// Hyperlink (href, text).
    Link(String, String),
    /// Unordered list of items (each a `Vec<Node>` of inline/block children).
    List(Vec<Vec<Node>>),
    /// Blockquote.
    Quote(String),
    /// Code block (preformatted, not HTML-escaped beyond entities).
    Code(String),
    /// Raw HTML passthrough (caller guarantees well-formedness).
    Raw(String),
}

/// Escape text for HTML: `&`, `<`, `>`, `"`, `'`.
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Render a single node to HTML (no trailing newline).
fn render_node(n: &Node, indent: usize) -> String {
    let pad = "  ".repeat(indent);
    match n {
        Node::Heading(level, text) => {
            let lvl = (*level).clamp(1, 6);
            format!("{pad}<h{lvl}>{}</h{lvl}>", escape(text))
        }
        Node::Paragraph(text) => format!("{pad}<p>{}</p>", escape(text)),
        Node::Link(href, text) => {
            format!("{pad}<a href=\"{}\">{}</a>", escape(href), escape(text))
        }
        Node::List(items) => {
            let mut out = String::from(&format!("{pad}<ul>\n"));
            for item in items {
                out.push_str(&format!("{pad}  <li>"));
                if item.len() == 1 {
                    out.push_str(&render_node(&item[0], 0));
                } else {
                    for child in item {
                        out.push_str(&render_node(child, 0));
                    }
                }
                out.push_str("</li>\n");
            }
            out.push_str(&format!("{pad}</ul>"));
            out
        }
        Node::Quote(text) => format!("{pad}<blockquote>{}</blockquote>", escape(text)),
        Node::Code(text) => format!("{pad}<pre><code>{}</code></pre>", escape(text)),
        Node::Raw(html) => format!("{pad}{html}"),
    }
}

/// Render a document (list of nodes) to a clean HTML fragment.
pub fn render(nodes: &[Node]) -> String {
    nodes
        .iter()
        .map(|n| render_node(n, 0))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Render a full minimal HTML page (doctype + semantic head/body).
pub fn render_page(title: &str, body: &[Node]) -> String {
    let mut out = String::new();
    out.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
    out.push_str(&format!("  <meta charset=\"utf-8\">\n  <title>{}</title>\n", escape(title)));
    out.push_str("</head>\n<body>\n");
    for n in body {
        out.push_str(&render_node(n, 1));
        out.push('\n');
    }
    out.push_str("</body>\n</html>");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_special_chars() {
        assert_eq!(escape("a<b>&\"c\""), "a&lt;b&gt;&amp;&quot;c&quot;");
        assert_eq!(escape("it's"), "it&#39;s");
    }

    #[test]
    fn renders_paragraph_and_heading() {
        let doc = vec![Node::Heading(1, "Title".into()), Node::Paragraph("Hello & hi".into())];
        let html = render(&doc);
        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<p>Hello &amp; hi</p>"));
    }

    #[test]
    fn renders_link_with_escaped_href() {
        let html = render(&[Node::Link("https://x/?a=1&b=2".into(), "go".into())]);
        assert!(html.contains("href=\"https://x/?a=1&amp;b=2\""));
        assert!(html.contains(">go</a>"));
    }

    #[test]
    fn renders_list() {
        let doc = vec![Node::List(vec![
            vec![Node::Paragraph("one".into())],
            vec![Node::Paragraph("two".into())],
        ])];
        let html = render(&doc);
        assert!(html.contains("<ul>"));
        assert!(html.contains("<li><p>one</p></li>"));
        assert!(html.contains("<li><p>two</p></li>"));
    }

    #[test]
    fn renders_full_page() {
        let page = render_page("Test", &[Node::Paragraph("body".into())]);
        assert!(page.starts_with("<!DOCTYPE html>"));
        assert!(page.contains("<title>Test</title>"));
        assert!(page.contains("<p>body</p>"));
        assert!(page.ends_with("</html>"));
    }

    #[test]
    fn no_div_soup() {
        // The whole point: semantic tags, no <div>.
        let doc = vec![
            Node::Heading(2, "H".into()),
            Node::Paragraph("P".into()),
            Node::Link("/x".into(), "L".into()),
        ];
        let html = render(&doc);
        assert!(!html.contains("<div"));
    }
}
