use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

/// Convert Markdown to Discord-friendly markdown.
///
/// Discord renders most markdown natively, so non-table content passes through as-is.
/// Tables are rendered as aligned monospace blocks inside ``` fences, since Discord
/// does not support markdown table syntax.
pub fn to_discord_markdown(markdown: &str) -> String {
    let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES;
    let parser = Parser::new_ext(markdown, options).into_offset_iter();

    let mut output = String::with_capacity(markdown.len());
    let mut last_pos = 0;

    // Table buffering state
    let mut in_table = false;
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut current_cell = String::new();

    for (event, range) in parser {
        if in_table {
            match event {
                Event::End(TagEnd::Table) => {
                    in_table = false;
                    output.push_str(&render_table(&table_rows, "```\n", "```\n\n", false));
                    table_rows.clear();
                    last_pos = range.end;
                }
                Event::Start(Tag::TableHead) | Event::Start(Tag::TableRow) => {
                    current_row = Vec::new();
                }
                Event::End(TagEnd::TableHead) | Event::End(TagEnd::TableRow) => {
                    table_rows.push(std::mem::take(&mut current_row));
                }
                Event::Start(Tag::TableCell) => {
                    current_cell = String::new();
                }
                Event::End(TagEnd::TableCell) => {
                    current_row.push(std::mem::take(&mut current_cell));
                }
                Event::Text(text) => current_cell.push_str(&text),
                Event::Code(text) => current_cell.push_str(&text),
                Event::SoftBreak | Event::HardBreak => current_cell.push(' '),
                _ => {}
            }
            continue;
        }

        if let Event::Start(Tag::Table(_)) = event {
            // Flush raw markdown before the table
            output.push_str(&markdown[last_pos..range.start]);
            in_table = true;
            table_rows.clear();
        }
    }

    // Flush remaining raw markdown after last table (or entire input if no tables)
    output.push_str(&markdown[last_pos..]);
    output
}

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

    // Table buffering state
    let mut in_table = false;
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut current_cell = String::new();

    for event in parser {
        // When inside a table, buffer everything and render on End(Table)
        if in_table {
            match event {
                Event::End(TagEnd::Table) => {
                    in_table = false;
                    output.push_str(&render_table(&table_rows, "<pre>\n", "</pre>\n\n", true));
                    table_rows.clear();
                }
                Event::Start(Tag::TableHead) | Event::Start(Tag::TableRow) => {
                    current_row = Vec::new();
                }
                Event::End(TagEnd::TableHead) | Event::End(TagEnd::TableRow) => {
                    table_rows.push(std::mem::take(&mut current_row));
                }
                Event::Start(Tag::TableCell) => {
                    current_cell = String::new();
                }
                Event::End(TagEnd::TableCell) => {
                    current_row.push(std::mem::take(&mut current_cell));
                }
                Event::Text(text) => current_cell.push_str(&text),
                Event::Code(text) => current_cell.push_str(&text),
                Event::SoftBreak | Event::HardBreak => current_cell.push(' '),
                _ => {}
            }
            continue;
        }

        match event {
            // Tables
            Event::Start(Tag::Table(_)) => {
                in_table = true;
                table_rows.clear();
            }

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

/// Render buffered table rows as an aligned monospace block.
///
/// `open`/`close` wrap the block (e.g. `<pre>\n`/`</pre>\n\n` or `` ```\n ``/`` ```\n\n ``).
/// `html_escape` controls whether cell content is HTML-escaped (needed for Telegram, not Discord).
fn render_table(rows: &[Vec<String>], open: &str, close: &str, html_escape: bool) -> String {
    if rows.is_empty() {
        return String::new();
    }

    let num_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut col_widths = vec![0usize; num_cols];
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            col_widths[i] = col_widths[i].max(cell.len());
        }
    }

    let mut result = String::from(open);
    for (row_idx, row) in rows.iter().enumerate() {
        for (col_idx, cell) in row.iter().enumerate() {
            if col_idx > 0 {
                result.push_str(" | ");
            }
            let width = col_widths.get(col_idx).copied().unwrap_or(0);
            let padded = format!("{:<width$}", cell);
            if html_escape {
                result.push_str(&escape_html(&padded));
            } else {
                result.push_str(&padded);
            }
        }
        result.push('\n');

        // Separator after header row
        if row_idx == 0 {
            for (col_idx, &width) in col_widths.iter().enumerate() {
                if col_idx > 0 {
                    result.push_str("-+-");
                }
                result.push_str(&"-".repeat(width));
            }
            result.push('\n');
        }
    }
    result.push_str(close);
    result
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

    #[test]
    fn table_renders_as_pre_telegram() {
        let input = "\
| Metric | Before | After |
|--------|--------|-------|
| build  | 315s   | 173s  |
| deploy | 360s   | 302s  |";
        let result = to_telegram_html(input);
        assert!(result.contains("<pre>"));
        assert!(result.contains("</pre>"));
        assert!(result.contains("Metric"));
        assert!(result.contains("315s"));
        assert!(result.contains("---"));
        assert!(result.contains(" | "));
    }

    #[test]
    fn table_renders_as_code_block_discord() {
        let input = "\
| Metric | Before | After |
|--------|--------|-------|
| build  | 315s   | 173s  |
| deploy | 360s   | 302s  |";
        let result = to_discord_markdown(input);
        assert!(result.contains("```"));
        assert!(result.contains("Metric"));
        assert!(result.contains("315s"));
        assert!(result.contains("---"));
        assert!(result.contains(" | "));
        // Should NOT contain HTML tags
        assert!(!result.contains("<pre>"));
    }

    #[test]
    fn discord_preserves_non_table_content() {
        let input = "Hello **world**\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\nGoodbye";
        let result = to_discord_markdown(input);
        assert!(result.contains("Hello **world**"));
        assert!(result.contains("```"));
        assert!(result.contains("Goodbye"));
    }

    #[test]
    fn discord_no_tables_passthrough() {
        let input = "Just **bold** and `code`";
        let result = to_discord_markdown(input);
        assert_eq!(result, input);
    }
}
