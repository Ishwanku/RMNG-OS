use rmng_core::{
    compute_audit_stats, rollup_llm_costs, AuditLog, AuditStats, ChainVerifyResult, CostRollupReport,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct AuditVerifyOutput {
    valid: bool,
    entries: u64,
    first_break_seq: Option<u64>,
    message: String,
    exit_code: i32,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    stats: Option<AuditStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cost_rollup: Option<CostRollupReport>,
}

pub fn run_verify(json: bool, stats: bool) -> i32 {
    let path = AuditLog::default_path();
    let log = AuditLog::new(&path);
    let verify = match log.verify_chain() {
        Ok(v) => v,
        Err(e) => {
            if json {
                let out = serde_json::json!({
                    "valid": false,
                    "exit_code": 2,
                    "error": e.to_string(),
                    "path": path.display().to_string(),
                });
                println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
            } else {
                eprintln!("verify error: {e}");
            }
            return 2;
        }
    };

    let entries = log.read_all().unwrap_or_default();
    let audit_stats = if stats {
        Some(compute_audit_stats(&entries))
    } else {
        None
    };
    let cost_rollup = if stats {
        Some(rollup_llm_costs(&entries))
    } else {
        None
    };

    let exit_code = if verify.valid { 0 } else { 1 };

    if json {
        let out = AuditVerifyOutput {
            valid: verify.valid,
            entries: verify.entries,
            first_break_seq: verify.first_break_seq,
            message: verify.message.clone(),
            exit_code,
            path: path.display().to_string(),
            stats: audit_stats,
            cost_rollup,
        };
        println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
    } else {
        print_text_report(&verify, audit_stats.as_ref(), cost_rollup.as_ref());
    }

    exit_code
}

fn print_text_report(
    verify: &ChainVerifyResult,
    stats: Option<&AuditStats>,
    rollup: Option<&CostRollupReport>,
) {
    println!("=== RMNG audit verify ===");
    println!();
    println!("path:      {}", AuditLog::default_path().display());
    println!("entries:   {}", verify.entries);
    println!(
        "integrity: {}",
        if verify.valid { "VALID" } else { "BROKEN" }
    );
    if let Some(seq) = verify.first_break_seq {
        println!("break at:  seq #{seq}");
    }
    println!("detail:    {}", verify.message);
    if let Some(s) = stats {
        println!();
        println!("-- audit stats --");
        println!("  llm_calls:      {}", s.llm_calls);
        println!("  mcp_calls:      {}", s.mcp_calls);
        println!("  circuit_events: {}", s.circuit_events);
        println!("  spent_today:    ${:.4}", s.spent_today_usd);
        println!("  spent_total:    ${:.4}", s.spent_total_usd);
        if !s.by_category.is_empty() {
            println!("  by_category:");
            let mut cats: Vec<_> = s.by_category.iter().collect();
            cats.sort_by_key(|(k, _)| *k);
            for (k, v) in cats {
                println!("    {k}: {v}");
            }
        }
    }
    if let Some(r) = rollup {
        println!();
        println!("-- cost rollup --");
        println!("  total: ${:.4} ({} calls)", r.total_cost_usd, r.total_llm_calls);
        println!("  today: ${:.4} ({} calls)", r.spent_today_usd, r.llm_calls_today);
        if !r.by_agent_today_ranked.is_empty() {
            println!("  agents today:");
            for a in r.by_agent_today_ranked.iter().take(5) {
                println!("    {}  ${:.4}  {} calls", a.id, a.cost_usd, a.llm_calls);
            }
        }
        if !r.daily.is_empty() {
            println!("  daily (recent):");
            for d in r.daily.iter().take(7) {
                println!("    {}  ${:.4}  {} calls", d.period, d.cost_usd, d.llm_calls);
            }
        }
    }
}
