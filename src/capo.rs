use std::cmp::Ordering;
use regex::Regex;

/// Capo priority weights (lower number = higher priority).
/// Based on Rick Viscomi's Capo.js: https://github.com/rviscomi/capo.js
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CapoPriority {
    OriginTrials = 1,
    MetaCharset = 2,
    MetaCsp = 3,
    MetaViewport = 4,
    Title = 5,
    Preconnect = 6,
    AsyncScript = 7,
    ImportStyles = 8,
    SyncScript = 9,
    SyncStyles = 10,
    Preload = 11,
    DeferScript = 12,
    PrefetchPrerender = 13,
    OtherMeta = 14,
}

impl CapoPriority {
    #[allow(dead_code)]
    pub fn name(&self) -> &'static str {
        match self {
            CapoPriority::OriginTrials => "ORIGIN_TRIALS",
            CapoPriority::MetaCharset => "META_CHARSET",
            CapoPriority::MetaCsp => "META_CSP",
            CapoPriority::MetaViewport => "META_VIEWPORT",
            CapoPriority::Title => "TITLE",
            CapoPriority::Preconnect => "PRECONNECT",
            CapoPriority::AsyncScript => "ASYNC_SCRIPT",
            CapoPriority::ImportStyles => "IMPORT_STYLES",
            CapoPriority::SyncScript => "SYNC_SCRIPT",
            CapoPriority::SyncStyles => "SYNC_STYLES",
            CapoPriority::Preload => "PRELOAD",
            CapoPriority::DeferScript => "DEFER_SCRIPT",
            CapoPriority::PrefetchPrerender => "PREFETCH_PRERENDER",
            CapoPriority::OtherMeta => "OTHER_META",
        }
    }
}

#[derive(Debug, Clone)]
pub struct HeadElement {
    pub raw: String,
    pub priority: CapoPriority,
    pub original_index: usize,
}

/// Classify a single `<head>` HTML element string according to Capo rules.
pub fn classify_element(raw: &str, index: usize) -> HeadElement {
    let lower = raw.to_lowercase();
    let trimmed = lower.trim();

    let priority = if trimmed.starts_with("<meta") {
        if trimmed.contains("http-equiv=\"origin-trial\"") || trimmed.contains("http-equiv='origin-trial'") {
            CapoPriority::OriginTrials
        } else if trimmed.contains("charset=") || trimmed.contains("http-equiv=\"content-type\"") || trimmed.contains("http-equiv='content-type'") {
            CapoPriority::MetaCharset
        } else if trimmed.contains("http-equiv=\"content-security-policy\"") || trimmed.contains("http-equiv='content-security-policy'") {
            CapoPriority::MetaCsp
        } else if trimmed.contains("name=\"viewport\"") || trimmed.contains("name='viewport'") {
            CapoPriority::MetaViewport
        } else {
            CapoPriority::OtherMeta
        }
    } else if trimmed.starts_with("<title") || trimmed.starts_with("<base") {
        CapoPriority::Title
    } else if trimmed.starts_with("<link") {
        let rel = extract_rel(&lower);
        match rel.as_str() {
            "preconnect" => CapoPriority::Preconnect,
            "preload" | "modulepreload" => CapoPriority::Preload,
            "stylesheet" => {
                if trimmed.contains("@import") {
                    CapoPriority::ImportStyles
                } else {
                    CapoPriority::SyncStyles
                }
            }
            "prefetch" | "prerender" | "dns-prefetch" => CapoPriority::PrefetchPrerender,
            _ => CapoPriority::OtherMeta,
        }
    } else if trimmed.starts_with("<style") {
        if trimmed.contains("@import") {
            CapoPriority::ImportStyles
        } else {
            CapoPriority::SyncStyles
        }
    } else if trimmed.starts_with("<script") {
        if trimmed.contains("async") {
            CapoPriority::AsyncScript
        } else if trimmed.contains("defer") || trimmed.contains("type=\"module\"") || trimmed.contains("type='module'") {
            CapoPriority::DeferScript
        } else {
            CapoPriority::SyncScript
        }
    } else {
        CapoPriority::OtherMeta
    };

    HeadElement {
        raw: raw.trim().to_string(),
        priority,
        original_index: index,
    }
}

fn extract_rel(lower: &str) -> String {
    let re = Regex::new(r#"rel\s*=\s*["']([^"']+)["']"#).unwrap();
    if let Some(caps) = re.captures(lower) {
        if let Some(m) = caps.get(1) {
            return m.as_str().to_string();
        }
    }
    String::new()
}

/// Parse the raw inner HTML of `<head>` into a list of HeadElements.
pub fn parse_head_elements(head_inner: &str) -> Vec<HeadElement> {
    // Regex matching HTML comments, or top-level tags like <meta...>, <link...>, <title>...</title>, <style>...</style>, <script>...</script>
    let tag_re = Regex::new(r"(?is)(<!--.*?-->|<title\b[^>]*>.*?</title>|<style\b[^>]*>.*?</style>|<script\b[^>]*>.*?</script>|<(?:meta|link|base)\b[^>]*>)").unwrap();

    let mut elements = Vec::new();
    let mut index = 0;

    for mat in tag_re.find_iter(head_inner) {
        let snippet = mat.as_str().trim();
        if snippet.is_empty() || snippet.starts_with("<!--") {
            continue; // ignore comments for priority ranking
        }
        elements.push(classify_element(snippet, index));
        index += 1;
    }

    elements
}

/// Calculate Capo compliance score (0.0 to 100.0) based on Kendall Tau rank distance / inversions.
pub fn calculate_capo_score(elements: &[HeadElement]) -> f64 {
    if elements.len() <= 1 {
        return 100.0;
    }

    let mut inversions = 0;
    let total_pairs = (elements.len() * (elements.len() - 1)) / 2;

    for i in 0..elements.len() {
        for j in (i + 1)..elements.len() {
            if elements[i].priority > elements[j].priority {
                inversions += 1;
            }
        }
    }

    let ratio = 1.0 - (inversions as f64 / total_pairs as f64);
    (ratio * 100.0).max(0.0).min(100.0)
}

/// Reorder elements stably according to Capo priorities.
pub fn reorder_head_elements(mut elements: Vec<HeadElement>) -> Vec<HeadElement> {
    elements.sort_by(|a, b| {
        match a.priority.cmp(&b.priority) {
            Ordering::Equal => a.original_index.cmp(&b.original_index),
            other => other,
        }
    });
    elements
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capo_classification() {
        let el_charset = classify_element("<meta charset=\"utf-8\">", 0);
        assert_eq!(el_charset.priority, CapoPriority::MetaCharset);

        let el_viewport = classify_element("<meta name=\"viewport\" content=\"width=device-width\">", 1);
        assert_eq!(el_viewport.priority, CapoPriority::MetaViewport);

        let el_title = classify_element("<title>My Page</title>", 2);
        assert_eq!(el_title.priority, CapoPriority::Title);

        let el_preconnect = classify_element("<link rel=\"preconnect\" href=\"https://fonts.gstatic.com\">", 3);
        assert_eq!(el_preconnect.priority, CapoPriority::Preconnect);

        let el_defer = classify_element("<script defer src=\"/app.js\"></script>", 4);
        assert_eq!(el_defer.priority, CapoPriority::DeferScript);
    }

    #[test]
    fn test_capo_reorder() {
        let head = r#"
        <title>Bad Order</title>
        <script defer src="/app.js"></script>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width">
        <link rel="stylesheet" href="/style.css">
        "#;

        let parsed = parse_head_elements(head);
        let score = calculate_capo_score(&parsed);
        assert!(score < 100.0);

        let reordered = reorder_head_elements(parsed);
        let new_score = calculate_capo_score(&reordered);
        assert_eq!(new_score, 100.0);

        assert_eq!(reordered[0].priority, CapoPriority::MetaCharset);
        assert_eq!(reordered[1].priority, CapoPriority::MetaViewport);
        assert_eq!(reordered[2].priority, CapoPriority::Title);
    }
}
