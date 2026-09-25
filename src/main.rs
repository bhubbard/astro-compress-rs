mod capo;
mod css;
mod html;
mod svg;

use anyhow::Result;
use clap::Parser;
use rayon::prelude::*;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use walkdir::WalkDir;

use css::minify_css;
use html::{ProcessOptions, process_html};
use svg::minify_svg;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "High-performance Rust post-build compressor, CSS inliner, and Capo head reorderer for Astro sites"
)]
struct Args {
    /// Directory containing built static site (e.g. dist/)
    #[arg(default_value = "dist")]
    dir: PathBuf,

    /// Reorder <head> elements according to Capo.js priorities
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    reorder_head: bool,

    /// Inline local stylesheets into <style> tags
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    inline_css: bool,

    /// Maximum CSS file size (in bytes) eligible for inlining (default 50KB)
    #[arg(long, default_value_t = 51200)]
    max_inline_css: usize,

    /// Minify inline and standalone CSS files with lightningcss
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    minify_css: bool,

    /// Minify HTML markup and whitespace
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    minify_html: bool,

    /// Minify standalone SVG files
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    minify_svg: bool,

    /// Run Capo audit only (do not write any files to disk)
    #[arg(long, default_value_t = false)]
    audit_only: bool,

    /// Dry run (simulate without writing files)
    #[arg(long, default_value_t = false)]
    dry_run: bool,

    /// Quiet mode (suppress verbose logs)
    #[arg(short, long, default_value_t = false)]
    quiet: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let start_time = Instant::now();

    if !args.dir.is_dir() {
        anyhow::bail!(
            "Directory '{}' does not exist or is not a directory",
            args.dir.display()
        );
    }

    println!("🚀 astro-compress-rs v{}", env!("CARGO_PKG_VERSION"));
    println!("📂 Target directory: {}", args.dir.display());
    println!(
        "⚙️  Config: Capo Head Reorder={}, Inline CSS={} (max {} B), Minify CSS={}, Minify SVG={}, Minify HTML={}",
        args.reorder_head,
        args.inline_css,
        args.max_inline_css,
        args.minify_css,
        args.minify_svg,
        args.minify_html
    );

    // Collect all files to process
    let mut html_files = Vec::new();
    let mut css_files = Vec::new();
    let mut svg_files = Vec::new();

    for entry in WalkDir::new(&args.dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                match ext.to_ascii_lowercase().as_str() {
                    "html" | "htm" => html_files.push(path.to_path_buf()),
                    "css" => css_files.push(path.to_path_buf()),
                    "svg" => svg_files.push(path.to_path_buf()),
                    _ => {}
                }
            }
        }
    }

    println!(
        "Found {} HTML files, {} CSS files, {} SVG files",
        html_files.len(),
        css_files.len(),
        svg_files.len()
    );

    let total_html_bytes_before = AtomicUsize::new(0);
    let total_html_bytes_after = AtomicUsize::new(0);
    let total_css_inlined_count = AtomicUsize::new(0);
    let total_css_inlined_bytes = AtomicUsize::new(0);
    let total_svg_bytes_saved = AtomicUsize::new(0);
    let total_css_bytes_saved = AtomicUsize::new(0);

    let sum_initial_capo_score = std::sync::atomic::AtomicU64::new(0);
    let sum_final_capo_score = std::sync::atomic::AtomicU64::new(0);

    let html_opts = ProcessOptions {
        reorder_head: args.reorder_head && !args.audit_only,
        inline_css: args.inline_css && !args.audit_only,
        max_inline_css_bytes: args.max_inline_css,
        minify_inline_css: args.minify_css && !args.audit_only,
        minify_html: args.minify_html && !args.audit_only,
    };

    // 1. Process HTML files in parallel
    html_files.par_iter().for_each(|path| {
        if let Ok(content) = fs::read_to_string(path) {
            let before_len = content.len();
            total_html_bytes_before.fetch_add(before_len, Ordering::Relaxed);

            if let Ok((processed, audit)) = process_html(&content, Some(&args.dir), &html_opts) {
                let after_len = processed.len();
                total_html_bytes_after.fetch_add(after_len, Ordering::Relaxed);
                total_css_inlined_count.fetch_add(audit.inlined_css_count, Ordering::Relaxed);
                total_css_inlined_bytes.fetch_add(audit.inlined_css_bytes, Ordering::Relaxed);

                sum_initial_capo_score
                    .fetch_add((audit.initial_capo_score * 100.0) as u64, Ordering::Relaxed);
                sum_final_capo_score
                    .fetch_add((audit.final_capo_score * 100.0) as u64, Ordering::Relaxed);

                if !args.audit_only && !args.dry_run {
                    let _ = fs::write(path, processed);
                }
            }
        }
    });

    // 2. Process standalone CSS files in parallel if requested
    if args.minify_css && !args.audit_only {
        css_files.par_iter().for_each(|path| {
            if let Ok(content) = fs::read_to_string(path) {
                let before_len = content.len();
                if let Ok(minified) = minify_css(&content) {
                    let after_len = minified.len();
                    if after_len < before_len {
                        total_css_bytes_saved.fetch_add(before_len - after_len, Ordering::Relaxed);
                        if !args.dry_run {
                            let _ = fs::write(path, minified);
                        }
                    }
                }
            }
        });
    }

    // 3. Process standalone SVG files in parallel if requested
    if args.minify_svg && !args.audit_only {
        svg_files.par_iter().for_each(|path| {
            if let Ok(content) = fs::read_to_string(path) {
                let before_len = content.len();
                let minified = minify_svg(&content);
                let after_len = minified.len();
                if after_len < before_len {
                    total_svg_bytes_saved.fetch_add(before_len - after_len, Ordering::Relaxed);
                    if !args.dry_run {
                        let _ = fs::write(path, minified);
                    }
                }
            }
        });
    }

    let elapsed = start_time.elapsed();
    let count_html = html_files.len().max(1) as f64;
    let avg_initial_capo =
        (sum_initial_capo_score.load(Ordering::Relaxed) as f64 / 100.0) / count_html;
    let avg_final_capo = (sum_final_capo_score.load(Ordering::Relaxed) as f64 / 100.0) / count_html;

    println!("\n✨ Compression & Audit Complete in {:.2?}", elapsed);
    println!("──────────────────────────────────────────────────");
    println!("📊 Capo.js Head Efficiency:");
    println!("   Initial Average Score: {:.1}%", avg_initial_capo);
    if !args.audit_only && args.reorder_head {
        println!(
            "   Optimized Average Score: {:.1}% (improved by +{:.1}%)",
            avg_final_capo,
            avg_final_capo - avg_initial_capo
        );
    }
    println!("📦 Critical CSS Inlining:");
    println!(
        "   Inlined stylesheets: {} instances ({} bytes total)",
        total_css_inlined_count.load(Ordering::Relaxed),
        total_css_inlined_bytes.load(Ordering::Relaxed)
    );
    println!("💾 Size Savings:");
    let html_before = total_html_bytes_before.load(Ordering::Relaxed);
    let html_after = total_html_bytes_after.load(Ordering::Relaxed);
    println!(
        "   HTML Size: {} KB → {} KB ({:+.1}%)",
        html_before / 1024,
        html_after / 1024,
        if html_before > 0 {
            ((html_after as f64 - html_before as f64) / html_before as f64) * 100.0
        } else {
            0.0
        }
    );
    println!(
        "   Standalone CSS saved: {} KB",
        total_css_bytes_saved.load(Ordering::Relaxed) / 1024
    );
    println!(
        "   Standalone SVG saved: {} KB",
        total_svg_bytes_saved.load(Ordering::Relaxed) / 1024
    );
    println!("──────────────────────────────────────────────────");

    Ok(())
}
