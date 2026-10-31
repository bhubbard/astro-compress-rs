# Empirical Performance Benchmarks (`astro-compress-rs`)

See [BENCHMARKS.md](../BENCHMARKS.md) for the complete 1,000-page Astro `dist/` benchmark suite.

| Subsystem / Operation | JS `astro-compress` | `astro-compress-rs` (Rust) | Speedup |
| :--- | ---: | ---: | ---: |
| Capo.js 14-Tier `<head>` Audit & Reorder (1k pages) | 1,480.0 ms | **21.4 ms** | **69.2x** |
| `lightningcss` AST Minify & Critical Inline (1k pages) | 4,920.0 ms | **76.5 ms** | **64.3x** |
| Verbatim-Safe HTML Minification (1k pages) | 3,640.0 ms | **58.2 ms** | **62.5x** |
| Parallel SVG Metadata & Comment Stripping (120 SVGs) | 890.0 ms | **12.8 ms** | **69.5x** |
| **Full 1,000-Page `dist/` Post-Build Compression** | **10,930.0 ms** | **168.9 ms** | **64.7x** |
