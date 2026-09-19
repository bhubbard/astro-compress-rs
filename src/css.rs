use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions, StyleSheet};
use anyhow::Result;

/// Minify a raw CSS string using lightningcss.
pub fn minify_css(css: &str) -> Result<String> {
    let mut stylesheet = StyleSheet::parse(css, ParserOptions::default())
        .map_err(|e| anyhow::anyhow!("CSS parse error: {:?}", e))?;

    stylesheet
        .minify(MinifyOptions::default())
        .map_err(|e| anyhow::anyhow!("CSS minify error: {:?}", e))?;

    let res = stylesheet
        .to_css(PrinterOptions {
            minify: true,
            ..Default::default()
        })
        .map_err(|e| anyhow::anyhow!("CSS print error: {:?}", e))?;

    Ok(res.code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minify_css() {
        let css = r#"
            body {
                background-color: #ffffff;
                color: #000000;
                margin: 0px 0px 0px 0px;
            }
            .nav > li {
                list-style-type: none;
            }
        "#;
        let min = minify_css(css).expect("CSS minification failed");
        assert!(!min.contains('\n'));
        assert!(min.contains("body{"));
        assert!(min.contains(".nav>li{list-style-type:none}"));
    }
}
