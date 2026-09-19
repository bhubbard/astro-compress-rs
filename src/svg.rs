use regex::Regex;

/// Minify an SVG string by stripping comments, doctypes, metadata, and redundant whitespace.
pub fn minify_svg(svg: &str) -> String {
    let comment_re = Regex::new(r"(?s)<!--.*?-->").unwrap();
    let xml_decl_re = Regex::new(r"(?s)<\?xml.*?\?>").unwrap();
    let doctype_re = Regex::new(r"(?s)<!DOCTYPE.*?>").unwrap();
    let metadata_re = Regex::new(r"(?s)<metadata.*?>.*?</metadata>").unwrap();
    let ws_between_tags_re = Regex::new(r">\s+<").unwrap();
    let multi_ws_re = Regex::new(r"\s{2,}").unwrap();

    let s = comment_re.replace_all(svg, "");
    let s = xml_decl_re.replace_all(&s, "");
    let s = doctype_re.replace_all(&s, "");
    let s = metadata_re.replace_all(&s, "");
    let s = ws_between_tags_re.replace_all(&s, "><");
    let s = multi_ws_re.replace_all(&s, " ");

    s.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minify_svg() {
        let raw = r#"
        <?xml version="1.0" encoding="utf-8"?>
        <!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd">
        <!-- Generator: Adobe Illustrator -->
        <svg version="1.1" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <metadata>
                <rdf:RDF>Test</rdf:RDF>
            </metadata>
            <circle cx="50" cy="50" r="40" fill="red" />
        </svg>
        "#;
        let min = minify_svg(raw);
        assert!(!min.contains("<?xml"));
        assert!(!min.contains("<!DOCTYPE"));
        assert!(!min.contains("<!--"));
        assert!(!min.contains("<metadata"));
        assert!(min.starts_with("<svg"));
        assert!(min.ends_with("</svg>"));
    }
}
