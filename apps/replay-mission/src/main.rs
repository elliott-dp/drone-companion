//! `replay-mission` — CLI over `cc-replay`.
//!
//! ```text
//!   replay-mission run   <mission_dir> [--json] [--findings-only]
//!   replay-mission diff  <mission_dir_a> <mission_dir_b>
//!   replay-mission audit <mission_dir>...
//! ```
//!
//! Exit codes: `0` success / clean / identical; `1` diff mismatch or audit over
//! threshold; `2` read error; `3` usage.

use std::path::Path;
use std::process::ExitCode;

use cc_replay::{audit, diff, run_mission, Timeline};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("run") => cmd_run(&args[1..]),
        Some("diff") => cmd_diff(&args[1..]),
        Some("audit") => cmd_audit(&args[1..]),
        _ => {
            eprintln!(
                "usage:\n  replay-mission run   <mission_dir> [--json] [--findings-only]\n  \
                 replay-mission diff  <dir_a> <dir_b>\n  replay-mission audit <mission_dir>..."
            );
            ExitCode::from(3)
        }
    }
}

fn cmd_run(args: &[String]) -> ExitCode {
    let json = args.iter().any(|a| a == "--json");
    let findings_only = args.iter().any(|a| a == "--findings-only");
    let Some(dir) = args.iter().find(|a| !a.starts_with("--")) else {
        eprintln!("run: missing <mission_dir>");
        return ExitCode::from(3);
    };
    let tl = match run_mission(Path::new(dir)) {
        Ok(tl) => tl,
        Err(e) => {
            eprintln!("run: {e}");
            return ExitCode::from(2);
        }
    };
    let shown: Vec<_> = if findings_only {
        tl.findings().cloned().collect()
    } else {
        tl.rows.clone()
    };
    if json {
        let out = serde_json::json!({
            "hash": tl.hash(),
            "ticks": tl.rows.len(),
            "findings": tl.findings().count(),
            "rows": shown,
        });
        println!("{}", serde_json::to_string_pretty(&out).unwrap());
    } else {
        println!("hash   {}", tl.hash());
        println!("ticks  {}", tl.rows.len());
        println!("findings {}", tl.findings().count());
        for r in &shown {
            println!(
                "  t={:>12}ns {:<8} {:<14} flags={:#06x} detail={} value={} limit={} conf={}",
                r.tick_ns, r.severity, r.action, r.health_flags, r.detail_code, r.value, r.limit,
                r.confidence
            );
        }
    }
    ExitCode::SUCCESS
}

fn cmd_diff(args: &[String]) -> ExitCode {
    if args.len() < 2 {
        eprintln!("diff: need <dir_a> <dir_b>");
        return ExitCode::from(3);
    }
    let load = |d: &str| -> Option<Timeline> {
        match run_mission(Path::new(d)) {
            Ok(tl) => Some(tl),
            Err(e) => {
                eprintln!("diff: {d}: {e}");
                None
            }
        }
    };
    let (Some(a), Some(b)) = (load(&args[0]), load(&args[1])) else {
        return ExitCode::from(2);
    };
    let diffs = diff(&a, &b);
    if diffs.is_empty() {
        println!("IDENTICAL  hash={}  ({} ticks)", a.hash(), a.rows.len());
        ExitCode::SUCCESS
    } else {
        println!("DIFFERENT  a={}  b={}", a.hash(), b.hash());
        for d in &diffs {
            println!("  {d}");
        }
        ExitCode::from(1)
    }
}

fn cmd_audit(args: &[String]) -> ExitCode {
    if args.is_empty() {
        eprintln!("audit: need at least one <mission_dir>");
        return ExitCode::from(3);
    }
    let mut timelines = Vec::new();
    for d in args {
        match run_mission(Path::new(d)) {
            Ok(tl) => timelines.push(tl),
            Err(e) => {
                eprintln!("audit: {d}: {e}");
                return ExitCode::from(2);
            }
        }
    }
    let st = audit(&timelines);
    println!(
        "missions={} ticks={} warn={} critical={} warn_rate={:.4}%",
        st.missions,
        st.ticks,
        st.warn_ticks,
        st.critical_ticks,
        st.warn_rate() * 100.0
    );
    for (code, n) in &st.by_detail {
        println!("  detail {code}: {n}");
    }
    // benign-corpus gate: any CRITICAL, or > 0.5% WARN, is a failure
    if st.critical_ticks > 0 || st.warn_rate() > 0.005 {
        eprintln!("AUDIT FAIL: findings exceed the benign-corpus threshold");
        ExitCode::from(1)
    } else {
        println!("AUDIT PASS");
        ExitCode::SUCCESS
    }
}
