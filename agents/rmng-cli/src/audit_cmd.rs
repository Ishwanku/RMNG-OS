use rmng_core::{rollup_llm_costs, AuditLog, ChainVerifyResult, CostRollupReport};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct AuditVerifyOutput {
    valid: bool,
    entries: u64,
    first_break_seq: Option<u64>,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cost_rollup: Option<CostRollupReport>,
}

pub fn run_verify(json: bool, stats: bool) -> i32 {
    let log = AuditLog::new(AuditLog::default_path());
    let verify = match log.verify_chain() {
        Ok(v) => v,
        Err(e) => {
            if json {
                let out = serde_json::json!({
                    "valid": false,
                    "error": e.to_string(),
                });
                println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
            } else {
                eprintln!("verify error: {e}");
            }
            return 2;
        }
    };

    let cost_rollup = if stats {
        log.read_all()
            .ok()
            .map(|entries| rollup_llm_costs(&entries))
    } else {
        None
    };

    if json {
        let out = AuditVerifyOutput {
            valid: verify.valid,
            entries: verify.entries,
            first_break_seq: verify.first_break_seq,
            message: verify.message.clone(),
            cost_rollup,
        };
        println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
    } else {
        print_text_report(&verify, cost_rollup.as_ref());
    }

    if verify.valid {
        0
    } else {
        1
    }
}

fn print_text_report(verify: &ChainVerifyResult, rollup: Option<&CostRollupReport>) {
    println!("=== RMNG audit verify ===");
    println!();
    println!("path:     {}", AuditLog::default_path().display());
    println!("entries:  {}", verify.entries);
    println!(
        "integrity: {}",
        if verify.valid {
            "VALID"
        } else {
            "BROKEN"
        }
    );
    if let Some(seq) = verify.first_break_seq {
        println!("break at: seq #{seq}");
    }
    println!("detail:   {}", verify.message);
    if let Some(r) = rollup {
        println!();
        println!("-- cost stats (LLM audit entries) --");
        println!("  total: ${:.4} ({} calls)", r.total_cost_usd, r.total_llm_calls);
        if !r.daily.is_empty() {
            println!("  daily (recent):");
            for d in r.daily.iter().take(7) {
                println!("    {}  ${:.4}  {} calls", d.period, d.cost_usd, d.llm_calls);
            }
        }
        if !r.by_session.is_empty() {
            println!("  top sessions:");
            for (sid, v) in r.by_session.iter().take(5) {
                println!("    {sid}  ${:.4}  {} calls", v.cost_usd, v.llm_calls);
            }
        }
    }
}