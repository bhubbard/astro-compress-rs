use std::path::Path;
use std::fs;
use anyhow::Result;
use regex::Regex;
use crate::capo::{calculate_capo_score, parse_head_elements, reorder_head_elements};
use crate::css::minify_css;

#[derive(Debug, Clone)]
pub struct ProcessOptions {
    pub reorder_head: bool,
    pub inline_css: bool,
    pub max_inline_css_bytes: usize,
    pub minify_inline_css: bool,
    pub minify_html: bool,
}

impl Default for ProcessOptions {
    fn default() -> Self {
        Self {
            reorder_head: true,
            inline_css: true,
            max_inline_css_bytes: 50 * 1024, // 50KB default inline cap
            minify_inline_css: true,
            minify_html: true,
        }
    }
}

pub struct HtmlAuditResult {
    pub initial_capo_score: f64,
    pub final_capo_score: f64,
    #[allow(dead_code)]
    pub elements_count: usize,
    pub inlined_css_count: usize,
    pub inlined_css_bytes: usize,
}

/// Process a single HTML string with the given options and base root directory (for resolving assets).
pub fn process_html(html: &str, base_dir: Option<&Path>, options: &ProcessOptions) -> Result<(String, HtmlAuditResult)> {
    let mut out = html.to_string();
    let mut inlined_css_count = 0;
    let mut inlined_css_bytes = 0;

    // 1. Inline local stylesheets if requested and base_dir is available
    if options.inline_css {
        if let Some(root) = base_dir {
            let link_re = Regex::new(r#"(?i)<link\b[^>]*\brel\s*=\s*["']stylesheet["'][^>]*>"#).unwrap();
            let href_re = Regex::new(r#"(?i)href\s*=\s*["']([^"']+)["']"#).unwrap();

            out = link_re.replace_all(&out, |caps: &regex::Captures| {
                let tag = &caps[0];
                if let Some(href_cap) = href_re.captures(tag) {
                    let href = &href_cap[1];
                    // Only inline local relative or root-relative paths, not http(s) or cdn
                    if !href.starts_with("http://") && !href.starts_with("https://") && !href.starts_with("//") {
                        let clean_href = href.split('?').next().unwrap_or(href);
                        let rel_path = clean_href.trim_start_matches('/');
                        let file_path = root.join(rel_path);

                        if file_path.is_file() {
                            if let Ok(metadata) = fs::metadata(&file_path) {
                                if metadata.len() as usize <= options.max_inline_css_bytes {
                                    if let Ok(content) = fs::read_to_string(&file_path) {
                                        let minified = if options.minify_inline_css {
                                            minify_css(&content).unwrap_or(content)
                                        } else {
                                            content
                                        };
                                        inlined_css_count += 1;
                                        inlined_css_bytes += minified.len();
                                        return format!("<style>{}</style>", minified);
                                    }
                                }
                            }
                        }
                    }
                }
                tag.to_string()
            }).to_string();
        }
    }

    // 2. Minify existing <style> tags
    if options.minify_inline_css {
        let style_re = Regex::new(r"(?is)<style\b([^>]*)>(.*?)</style>").unwrap();
        out = style_re.replace_all(&out, |caps: &regex::Captures| {
            let attrs = &caps[1];
            let css_body = &caps[2];
            let minified = minify_css(css_body).unwrap_or_else(|_| css_body.to_string());
            format!("<style{}>{}</style>", attrs, minified)
        }).to_string();
    }

    // 3. Capo Head Audit & Reordering
    let head_re = Regex::new(r"(?is)(<head\b[^>]*>)(.*?)(</head>)").unwrap();
    let mut initial_score = 100.0;
    let mut final_score = 100.0;
    let mut elements_count = 0;

    if let Some(caps) = head_re.captures(&out) {
        let head_open = &caps[1];
        let head_inner = &caps[2];
        let head_close = &caps[3];

        let parsed = parse_head_elements(head_inner);
        elements_count = parsed.len();
        initial_score = calculate_capo_score(&parsed);
        final_score = initial_score;

        if options.reorder_head && initial_score < 100.0 {
            let reordered = reorder_head_elements(parsed);
            final_score = calculate_capo_score(&reordered);

            let mut new_head_inner = String::new();
            for el in reordered {
                new_head_inner.push_str("\n    ");
                new_head_inner.push_str(&el.raw);
            }
            new_head_inner.push('\n');

            let replacement = format!("{}{}{}", head_open, new_head_inner, head_close);
            out = head_re.replace(&out, replacement.as_str()).to_string();
        }
    }

    // 4. Minify HTML (whitespace, comments)
    if options.minify_html {
        out = minify_html_string(&out);
    }

    let audit = HtmlAuditResult {
        initial_capo_score: initial_score,
        final_capo_score: final_score,
        elements_count,
        inlined_css_count,
        inlined_css_bytes,
    };

    Ok((out, audit))
}

/// Fast, safe HTML minification:
/// - Strip comments (except IE conditionals and SSI)
/// - Collapse whitespace between tags outside <pre>, <code>, <textarea>, <script>
pub fn minify_html_string(html: &str) -> String {
    // Strip standard HTML comments (preserving IE conditionals and SSI)
    let comment_re = Regex::new(r"(?s)<!--.*?-->").unwrap();
    let without_comments = comment_re.replace_all(html, |caps: &regex::Captures| {
        let comment = &caps[0];
        let lower = comment.to_lowercase();
        if lower.starts_with("<!--[if") || lower.starts_with("<!--#") {
            comment.to_string()
        } else {
            String::new()
        }
    });

    // Preserve verbatim blocks (<pre>, <code>, <textarea>, <script>) by tokenizing
    let verbatim_re = Regex::new(r"(?is)(<pre\b[^>]*>.*?</pre>|<code\b[^>]*>.*?</code>|<textarea\b[^>]*>.*?</textarea>|<script\b[^>]*>.*?</script>)").unwrap();
    let mut placeholders = Vec::new();

    let mut placeholder_index = 0;
    let masked = verbatim_re.replace_all(&without_comments, |caps: &regex::Captures| {
        let token = format!("@@VERBATIM_{}@@", placeholder_index);
        placeholder_index += 1;
        placeholders.push(caps[0].to_string());
        token
    });

    // Collapse whitespace between tags
    let ws_re = Regex::new(r">\s+<").unwrap();
    let collapsed = ws_re.replace_all(&masked, "><");

    // Collapse multi-spaces within text
    let multi_space_re = Regex::new(r"[ \t]{2,}").unwrap();
    let mut result = multi_space_re.replace_all(&collapsed, " ").to_string();

    // Restore verbatim blocks
    for (i, original) in placeholders.into_iter().enumerate() {
        let token = format!("@@VERBATIM_{}@@", i);
        result = result.replace(&token, &original);
    }

    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_html_capo_reorder() {
        let raw = r#"<!DOCTYPE html>
<html>
<head>
    <title>Test Page</title>
    <script defer src="/app.js"></script>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <!-- A comment in head -->
    <link rel="stylesheet" href="/styles.css">
</head>
<body>
    <h1>Hello World</h1>
</body>
</html>"#;

        let options = ProcessOptions {
            reorder_head: true,
            inline_css: false,
            max_inline_css_bytes: 0,
            minify_inline_css: true,
            minify_html: true,
        };

        let (processed, audit) = process_html(raw, None, &options).expect("Processing failed");
        assert!(audit.initial_capo_score < 100.0);
        assert_eq!(audit.final_capo_score, 100.0);

        let charset_idx = processed.find("charset=\"utf-8\"").unwrap();
        let viewport_idx = processed.find("name=\"viewport\"").unwrap();
        let title_idx = processed.find("<title>").unwrap();
        let script_idx = processed.find("<script defer").unwrap();

        assert!(charset_idx < viewport_idx);
        assert!(viewport_idx < title_idx);
        assert!(title_idx < script_idx);
    }
}
