use super::types::LlmUsage;

/// Heuristic $/1M token rates for cost estimates when providers don't return billing.
struct ModelRate {
    input_per_m: f64,
    output_per_m: f64,
}

fn rate_for_model(model: &str) -> ModelRate {
    let m = model.to_lowercase();
    if m.contains("gpt-4o") && !m.contains("mini") {
        return ModelRate {
            input_per_m: 2.50,
            output_per_m: 10.0,
        };
    }
    if m.contains("gpt-4o-mini") || m.contains("gpt-4.1-mini") {
        return ModelRate {
            input_per_m: 0.15,
            output_per_m: 0.60,
        };
    }
    if m.contains("claude-3-5-haiku") || m.contains("haiku") {
        return ModelRate {
            input_per_m: 0.80,
            output_per_m: 4.0,
        };
    }
    if m.contains("claude") && m.contains("sonnet") {
        return ModelRate {
            input_per_m: 3.0,
            output_per_m: 15.0,
        };
    }
    if m.contains("gemini-3.5-flash") || m.contains("gemini-2.5-flash-lite") {
        return ModelRate {
            input_per_m: 0.10,
            output_per_m: 0.40,
        };
    }
    if m.contains("gemini") && m.contains("pro") {
        return ModelRate {
            input_per_m: 1.25,
            output_per_m: 5.0,
        };
    }
    if m.contains("grok") {
        return ModelRate {
            input_per_m: 2.0,
            output_per_m: 10.0,
        };
    }
    if m.contains("llama") || m.contains("groq") {
        return ModelRate {
            input_per_m: 0.05,
            output_per_m: 0.08,
        };
    }
    if m.contains("deepseek") {
        return ModelRate {
            input_per_m: 0.14,
            output_per_m: 0.28,
        };
    }
    // Local / unknown — zero cost estimate
    ModelRate {
        input_per_m: 0.0,
        output_per_m: 0.0,
    }
}

/// Attach estimated USD cost when provider did not supply billing data.
pub fn enrich_usage_cost(provider: &str, model: &str, usage: &mut LlmUsage) {
    if usage.estimated_cost_usd.is_some() {
        return;
    }
    let (prompt, completion) = match (usage.prompt_tokens, usage.completion_tokens) {
        (Some(p), Some(c)) => (p, c),
        _ => return,
    };
    let rate = rate_for_model(model);
    if rate.input_per_m == 0.0 && rate.output_per_m == 0.0 && provider != "ollama" {
        // Still attach zero for explicit local models
        if provider == "ollama" {
            usage.estimated_cost_usd = Some(0.0);
            usage.cost_source = Some("local".into());
        }
        return;
    }
    let cost = (prompt as f64 / 1_000_000.0) * rate.input_per_m
        + (completion as f64 / 1_000_000.0) * rate.output_per_m;
    usage.estimated_cost_usd = Some(cost);
    usage.cost_source = Some("estimate".into());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimates_groq_llama_cost() {
        let mut usage = LlmUsage::from_counts(Some(1000), Some(500));
        enrich_usage_cost("groq", "llama-3.3-70b-versatile", &mut usage);
        assert!(usage.estimated_cost_usd.unwrap() > 0.0);
        assert_eq!(usage.cost_source.as_deref(), Some("estimate"));
    }
}