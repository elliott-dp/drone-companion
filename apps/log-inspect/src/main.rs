//! `log-inspect` — open a mission dataset and report its integrity.
//!
//! ```text
//! log-inspect <mission_dir>            human summary + verdict (exit per verdict)
//! log-inspect <mission_dir> --json     one machine-readable JSON object
//! log-inspect --raw <raw_mavlink.bin>  frame count + torn-tail check for a raw capture
//! log-inspect <mission_dir> --lenient  exit 0 unless CORRUPT (read a crashed dataset)
//! ```
//!
//! Exit codes: 0 Clean, 1 Dirty, 2 Corrupt, 3 usage/IO. The Phase-5 crash
//! harness asserts exit 1 (Dirty); the 1 h mission asserts exit 0 (Clean).

use std::path::PathBuf;
use std::process::ExitCode;

use cc_mission_log::inspect::{inspect_mission, raw_summary, Report, Verdict};

struct Args {
    mission_dir: Option<PathBuf>,
    raw: Option<PathBuf>,
    json: bool,
    lenient: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args { mission_dir: None, raw: None, json: false, lenient: false };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--json" => a.json = true,
            "--lenient" => a.lenient = true,
            "--strict" => a.lenient = false, // documented default; explicit no-op
            "--raw" => a.raw = Some(PathBuf::from(it.next().ok_or("--raw needs a path")?)),
            "-h" | "--help" => return Err(usage()),
            s if s.starts_with('-') => return Err(format!("unknown flag {s}\n{}", usage())),
            s => a.mission_dir = Some(PathBuf::from(s)),
        }
    }
    Ok(a)
}

fn usage() -> String {
    "usage: log-inspect <mission_dir> [--json] [--lenient]\n       log-inspect --raw <raw_mavlink.bin>".into()
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(msg) => {
            eprintln!("{msg}");
            return ExitCode::from(3);
        }
    };

    // --raw mode: standalone capture summary.
    if let Some(raw) = args.raw {
        let (frames, torn) = raw_summary(&raw);
        if args.json {
            println!(
                "{}",
                serde_json::json!({ "raw": raw.display().to_string(), "frames": frames, "torn": torn })
            );
        } else {
            println!("raw_mavlink.bin: {frames} frames{}", if torn { " (TORN trailing frame)" } else { "" });
        }
        return ExitCode::from(if torn { 1 } else { 0 });
    }

    let Some(dir) = args.mission_dir else {
        eprintln!("{}", usage());
        return ExitCode::from(3);
    };
    if !dir.is_dir() {
        eprintln!("not a directory: {}", dir.display());
        return ExitCode::from(3);
    }

    let report = inspect_mission(&dir);
    if args.json {
        println!("{}", to_json(&report));
    } else {
        print_human(&report);
    }

    let code = if args.lenient {
        match report.verdict {
            Verdict::Corrupt(_) => 2,
            _ => 0,
        }
    } else {
        report.exit_code()
    };
    ExitCode::from(code as u8)
}

fn issues(v: &Verdict) -> &[String] {
    match v {
        Verdict::Clean => &[],
        Verdict::Dirty(m) | Verdict::Corrupt(m) => m,
    }
}

fn to_json(r: &Report) -> String {
    let segs: Vec<_> = r
        .segments
        .iter()
        .map(|s| {
            let streams: Vec<_> = s
                .streams
                .iter()
                .map(|st| {
                    serde_json::json!({
                        "name": st.name, "parts": st.parts, "rows": st.rows,
                        "first_cc_ns": st.first_cc_ns, "last_cc_ns": st.last_cc_ns,
                        "seq_gap_total": st.seq_gap_total,
                    })
                })
                .collect();
            serde_json::json!({
                "dir": s.dir, "cc_boot_id": s.cc_boot_id, "px4_boot_id": s.px4_boot_id,
                "closed": s.closed, "inprogress_parts": s.inprogress_parts,
                "raw_present": s.raw_present, "raw_frames": s.raw_frames, "raw_torn": s.raw_torn,
                "drops": s.drops, "streams": streams,
            })
        })
        .collect();
    serde_json::json!({
        "mission_dir": r.mission_dir.display().to_string(),
        "verdict": r.verdict.label(),
        "issues": issues(&r.verdict),
        "mission_id": r.mission_id,
        "vehicle_id": r.vehicle_id,
        "complete": r.complete,
        "dialect_hash": r.dialect_hash,
        "dialect_hash_ok": r.dialect_hash_ok,
        "schema_version_ok": r.schema_version_ok,
        "total_rows": r.total_rows(),
        "total_drops": r.total_drops(),
        "segments": segs,
    })
    .to_string()
}

fn print_human(r: &Report) {
    println!("mission {} (vehicle {})  [{}]", r.mission_id, r.vehicle_id, r.verdict.label());
    println!("  dir:            {}", r.mission_dir.display());
    println!("  complete:       {}", r.complete);
    println!("  dialect hash:   {} ({})", r.dialect_hash, if r.dialect_hash_ok { "match" } else { "MISMATCH" });
    println!("  schema version: {}", if r.schema_version_ok { "match" } else { "MISMATCH" });
    println!("  total rows:     {}", r.total_rows());
    println!("  total drops:    {}", r.total_drops());
    for s in &r.segments {
        println!(
            "  {} (cc_boot {}, px4_boot {}, {})",
            s.dir, s.cc_boot_id, s.px4_boot_id,
            if s.closed { "closed" } else { "OPEN/incomplete" }
        );
        if s.inprogress_parts > 0 {
            println!("    ! {} in-progress part(s)", s.inprogress_parts);
        }
        for st in &s.streams {
            if st.rows > 0 || st.parts > 0 {
                let span = match (st.first_cc_ns, st.last_cc_ns) {
                    (Some(a), Some(b)) => format!("{:.1}s", (b - a) as f64 / 1e9),
                    _ => "-".into(),
                };
                println!(
                    "    {:<14} {:>8} rows  {:>4} parts  span {:>7}  gaps {}",
                    st.name, st.rows, st.parts, span, st.seq_gap_total
                );
            }
        }
        if s.raw_present {
            println!("    raw_mavlink    {} frames{}", s.raw_frames, if s.raw_torn { " (torn tail)" } else { "" });
        }
    }
    for issue in issues(&r.verdict) {
        println!("  - {issue}");
    }
}
