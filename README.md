# astro-compress-rs 🚀

A blazingly fast post-build compressor, CSS inliner, and `<head>` optimizer written in Rust for Astro and static sites.

It unifies and replaces multiple Node.js post-build tools:
1. **Capo `<head>` Optimizer & Auditor** (replaces `rviscomi/capo.js`):
   - Mathematically analyzes `<head>` ordering using Kendall Tau rank distance.
   - Reorders `<head>` elements strictly according to browser performance priorities (Origin Trials, charset, CSP, viewport, title, preconnects, critical styles, async/defer scripts).
2. **Critical CSS Inliner** (replaces `@playform/inline`):
   - Inlines render-blocking `<link rel="stylesheet">` tags directly into `<style>` tags with configurable threshold caps.
   - Eliminates extra round-trip requests for critical stylesheets.
3. **Rust Multi-Threaded Minifier** (replaces `@playform/compress`):
   - **CSS**: Minifies inline and standalone styles via `lightningcss`.
   - **HTML**: Strips redundant whitespace, HTML comments, and collapses tags.
   - **SVG**: Fast comment, doctype, and whitespace stripping.
   - Processes thousands of files concurrently across all CPU cores with `rayon`.

---

## Installation & Build

```bash
cargo build --release
```

Binary is output to `target/release/astro-compress-rs`.

---

## Usage

```bash
# Run full compression, CSS inlining, and Capo head reordering on Astro's dist/
astro-compress-rs dist/

# Run in audit-only mode (read-only diagnostics, prints average Capo score)
astro-compress-rs dist/ --audit-only

# Dry run simulation
astro-compress-rs dist/ --dry-run

# Custom inline size threshold (e.g. 20KB)
astro-compress-rs dist/ --max-inline-css 20480
```

---

## Performance

Processes hundreds of static pages in **< 150ms** on Apple Silicon, achieving 100% Capo head optimization while eliminating render-blocking CSS network roundtrips.
