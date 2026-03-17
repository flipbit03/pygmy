use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

/// Convert Markdown to Telegram-compatible HTML.
///
/// Telegram supports: `<b>`, `<i>`, `<u>`, `<s>`, `<code>`, `<pre>`, `<a href>`, `<blockquote>`.
/// Unsupported elements are degraded gracefully to plain text.
pub fn to_telegram_html(markdown: &str) -> String {
    let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES;
    let parser = Parser::new_ext(markdown, options);

    let mut output = String::with_capacity(markdown.len());
    let mut list_depth: usize = 0;
    let mut ordered_indices: Vec<u64> = Vec::new();

    for event in parser {
        match event {
            // Inline formatting
            Event::Start(Tag::Strong) => output.push_str("<b>"),
            Event::End(TagEnd::Strong) => output.push_str("</b>"),
            Event::Start(Tag::Emphasis) => output.push_str("<i>"),
            Event::End(TagEnd::Emphasis) => output.push_str("</i>"),
            Event::Start(Tag::Strikethrough) => output.push_str("<s>"),
            Event::End(TagEnd::Strikethrough) => output.push_str("</s>"),

            // Code
            Event::Code(text) => {
                output.push_str("<code>");
                output.push_str(&escape_html(&text));
                output.push_str("</code>");
            }
            Event::Start(Tag::CodeBlock(_)) => output.push_str("<pre>"),
            Event::End(TagEnd::CodeBlock) => output.push_str("</pre>"),

            // Links
            Event::Start(Tag::Link { dest_url, .. }) => {
                output.push_str("<a href=\"");
                output.push_str(&escape_html(&dest_url));
                output.push_str("\">");
            }
            Event::End(TagEnd::Link) => output.push_str("</a>"),

            // Blockquotes
            Event::Start(Tag::BlockQuote(_)) => output.push_str("<blockquote>"),
            Event::End(TagEnd::BlockQuote(_)) => output.push_str("</blockquote>"),

            // Headings → bold text
            Event::Start(Tag::Heading { .. }) => output.push_str("<b>"),
            Event::End(TagEnd::Heading(_)) => {
                output.push_str("</b>\n");
            }

            // Lists → unicode bullets / numbers
            Event::Start(Tag::List(start)) => {
                list_depth += 1;
                ordered_indices.push(start.unwrap_or(0));
            }
            Event::End(TagEnd::List(_)) => {
                list_depth = list_depth.saturating_sub(1);
                ordered_indices.pop();
            }
            Event::Start(Tag::Item) => {
                let indent = "  ".repeat(list_depth.saturating_sub(1));
                if let Some(idx) = ordered_indices.last_mut() {
                    if *idx > 0 {
                        output.push_str(&format!("{indent}{}. ", idx));
                        *idx += 1;
                    } else {
                        output.push_str(&format!("{indent}• "));
                    }
                } else {
                    output.push_str(&format!("{indent}• "));
                }
            }
            Event::End(TagEnd::Item) => {
                if !output.ends_with('\n') {
                    output.push('\n');
                }
            }

            // Paragraphs
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => output.push_str("\n\n"),

            // Text and whitespace
            Event::Text(text) => output.push_str(&escape_html(&text)),
            Event::SoftBreak => output.push('\n'),
            Event::HardBreak => output.push('\n'),

            // Horizontal rule
            Event::Rule => output.push_str("---\n"),

            // Everything else: ignore
            _ => {}
        }
    }

    // Trim trailing whitespace
    let trimmed = output.trim_end();
    trimmed.to_string()
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bold_and_italic() {
        assert_eq!(to_telegram_html("**bold**"), "<b>bold</b>");
        assert_eq!(to_telegram_html("*italic*"), "<i>italic</i>");
    }

    #[test]
    fn inline_code() {
        assert_eq!(to_telegram_html("`some code`"), "<code>some code</code>");
    }

    #[test]
    fn code_block() {
        let input = "```\nfn main() {}\n```";
        let result = to_telegram_html(input);
        assert!(result.contains("<pre>"));
        assert!(result.contains("fn main() {}"));
        assert!(result.contains("</pre>"));
    }

    #[test]
    fn link() {
        assert_eq!(
            to_telegram_html("[click](https://example.com)"),
            "<a href=\"https://example.com\">click</a>"
        );
    }

    #[test]
    fn heading_becomes_bold() {
        assert_eq!(to_telegram_html("# Title"), "<b>Title</b>");
    }

    #[test]
    fn unordered_list() {
        let input = "- one\n- two\n- three";
        let result = to_telegram_html(input);
        assert!(result.contains("• one"));
        assert!(result.contains("• two"));
        assert!(result.contains("• three"));
    }

    #[test]
    fn ordered_list() {
        let input = "1. first\n2. second";
        let result = to_telegram_html(input);
        assert!(result.contains("1. first"));
        assert!(result.contains("2. second"));
    }

    #[test]
    fn html_escaping() {
        assert_eq!(to_telegram_html("a < b & c > d"), "a &lt; b &amp; c &gt; d");
    }

    #[test]
    fn strikethrough() {
        assert_eq!(to_telegram_html("~~deleted~~"), "<s>deleted</s>");
    }

    #[test]
    fn blockquote() {
        let result = to_telegram_html("> quoted text");
        assert!(result.contains("<blockquote>"));
        assert!(result.contains("quoted text"));
    }

    #[test]
    fn plain_text_passthrough() {
        assert_eq!(to_telegram_html("hello world"), "hello world");
    }
}
