use rmng_core::{parse_provider_str, LlmProviderKind, RmngConfig};
use rmng_nervous::{
    apply_live_models, catalog_path, compare_models, default_model, install_user_catalog,
    list_all_providers, list_catalog_models, load_catalog, provider_id, resolve_api_key,
    resolve_model_pricing, user_catalog_path,
};

pub fn print_show() {
    let cfg = RmngConfig::load();
    let llm = cfg.resolved_llm();
    let cat = load_catalog();
    println!("=== RMNG LLM configuration ===");
    println!();
    println!("config:   {}", RmngConfig::config_path().display());
    println!("catalog:  {} (v{})", cat.path.display(), cat.file.catalog.version);
    if let Some(p) = &cfg.profile {
        println!("profile:  {p}");
    }
    println!();
    println!("active provider: {:?}", llm.llm_provider);
    println!(
        "model:           {}",
        llm.model.as_deref().unwrap_or(&default_model(llm.llm_provider))
    );
    if let Some(ep) = &llm.endpoint_url {
        println!("endpoint:        {ep}");
    }
    if let Some(env) = &llm.api_key_env_var {
        let set = resolve_api_key(&llm).ok().flatten().is_some();
        println!("api_key_env:     {env} (set={set})");
    }
    println!("max_retries:     {}", llm.max_retries);
    println!("timeout_secs:    {}", llm.timeout_secs);
    if !cfg.profiles.is_empty() {
        println!();
        println!("-- profiles ({}) --", cfg.profiles.len());
        for p in &cfg.profiles {
            let active = cfg.profile.as_deref() == Some(p.name.as_str());
            let mark = if active { "*" } else { " " };
            let prov = p
                .llm_provider
                .map(|x| format!("{x:?}"))
                .unwrap_or_else(|| "-".into());
            let model = p.model.as_deref().unwrap_or("-");
            println!("  {mark} {} — provider={prov} model={model}", p.name);
        }
    }
}

pub fn print_providers() {
    let cat = load_catalog();
    println!("Catalog: {} (v{})", cat.path.display(), cat.file.catalog.version);
    println!();
    for (id, p) in list_all_providers() {
        let def_model = list_catalog_models(parse_id(&id), false)
            .into_iter()
            .find(|m| m.default)
            .map(|m| m.id)
            .unwrap_or_else(|| "-".into());
        let env = p.api_key_env.clone().unwrap_or_else(|| "-".into());
        println!(
            "{id:<12} {:<22} api={:<14} env={env:<18} default={def_model}",
            p.label, p.api_style
        );
        if let Some(url) = &p.docs_url {
            println!("             docs: {url}");
        }
    }
}

pub fn print_models(
    provider: Option<&str>,
    include_specialized: bool,
    live: bool,
    pricing: bool,
) {
    let prov = match provider {
        Some(s) => parse_provider_str(s).unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(1);
        }),
        None => RmngConfig::load().resolved_llm().llm_provider,
    };

    if live {
        print_models_live(prov, include_specialized, pricing);
        return;
    }

    let models = list_catalog_models(prov, include_specialized);
    if models.is_empty() {
        println!("No catalog models for {prov:?}");
        return;
    }
    let prov_key = provider_id(prov);
    println!("Models for {:?} (catalog):", prov);
    if pricing {
        println!("  {:<36} {:>12} {:>12}  source", "model", "in$/1M", "out$/1M");
    }
    for m in models {
        print_catalog_entry(&m, prov_key, pricing);
    }
}

fn print_catalog_entry(m: &rmng_nervous::ModelEntry, prov_key: &str, pricing: bool) {
    if pricing {
        let (in_rate, out_rate, source) = pricing_line(prov_key, m);
        println!(
            "  {:<36} ${:>10.4} ${:>10.4}  {source}",
            m.id, in_rate, out_rate
        );
        return;
    }
    let tags = [
        if m.default { Some("default") } else { None },
        if m.specialized { Some("specialized") } else { None },
        m.tier.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(", ");
    let desc = m.description.as_deref().unwrap_or("");
    println!("  {:<36} [{tags}] {desc}", m.id);
}

fn pricing_line(prov_key: &str, m: &rmng_nervous::ModelEntry) -> (f64, f64, &'static str) {
    if let (Some(i), Some(o)) = (m.input_cost_per_m, m.output_cost_per_m) {
        return (i, o, "catalog");
    }
    resolve_model_pricing(prov_key, &m.id)
}

fn print_models_live(prov: LlmProviderKind, include_specialized: bool, pricing: bool) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    match rt.block_on(compare_models(prov, include_specialized)) {
        Ok(report) => {
            println!("Models for {:?} (live vs catalog):", prov);
            if let Some(note) = &report.detail {
                println!("  note: {note}");
            }
            if !report.live_models.is_empty() {
                println!();
                println!("-- live API ({} models) --", report.live_models.len());
                for id in &report.live_models {
                    let in_catalog = list_catalog_models(prov, include_specialized)
                        .iter()
                        .any(|m| m.id == *id);
                    let tag = if in_catalog { "catalog" } else { "NEW" };
                    println!("  {id:<36} [{tag}]");
                }
            }
            println!();
            println!("-- catalog --");
            let prov_key = provider_id(prov);
            if pricing {
                println!("  {:<36} {:>12} {:>12}  source", "model", "in$/1M", "out$/1M");
            }
            for m in list_catalog_models(prov, include_specialized) {
                print_catalog_entry(&m, prov_key, pricing);
            }
            if !report.catalog_only.is_empty() {
                println!();
                println!("WARN catalog-only (not in live API — may be deprecated/renamed):");
                for id in &report.catalog_only {
                    println!("  {id}");
                }
            }
            if !report.live_only.is_empty() {
                println!();
                println!("WARN live-only (add to ~/.rmng/llm-catalog.toml):");
                for id in &report.live_only {
                    println!("  {id}");
                }
            }
        }
        Err(e) => {
            eprintln!("live model discovery failed: {e}");
            std::process::exit(1);
        }
    }
}

pub fn run_setup() -> i32 {
    let src = catalog_path();
    if !src.is_file() {
        eprintln!("catalog not found at {}", src.display());
        return 1;
    }
    match install_user_catalog(&src) {
        Ok(dest) => {
            println!("Installed catalog → {}", dest.display());
            println!();
            println!("Next steps:");
            println!("  1. Edit ~/.rmng/config.toml — set llm_provider, model, api_key_env_var");
            println!("  2. Or add [[llm.profiles]] blocks and: rmng llm use <name>");
            println!("  3. Keys in ~/.rmng/secrets.env (never commit keys)");
            println!("  4. When models change: edit ~/.rmng/llm-catalog.toml (no rebuild)");
            0
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

pub fn run_use(profile_name: &str) -> i32 {
    let path = RmngConfig::config_path();
    let mut cfg: RmngConfig = if path.exists() {
        let raw = std::fs::read_to_string(&path).unwrap_or_default();
        toml::from_str(&raw).unwrap_or_default()
    } else {
        RmngConfig::default()
    };
    if !cfg.profiles.iter().any(|p| p.name == profile_name) {
        eprintln!(
            "profile '{profile_name}' not in config — add [[profiles]] with name = \"{profile_name}\""
        );
        return 1;
    }
    cfg.profile = Some(profile_name.to_string());
    let out = toml::to_string_pretty(&cfg).unwrap_or_default();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(e) = std::fs::write(&path, out) {
        eprintln!("write {}: {e}", path.display());
        return 1;
    }
    println!("Active profile: {profile_name}");
    print_show();
    0
}

fn parse_id(id: &str) -> LlmProviderKind {
    parse_provider_str(id).unwrap_or(LlmProviderKind::None)
}

/// Compare live provider APIs against local catalog; optionally merge new models (Sprint 9).
pub fn run_sync_catalog(include_specialized: bool, apply: bool) -> i32 {
    let providers = [
        "grok", "openai", "groq", "google", "anthropic", "together", "fireworks", "deepseek",
        "nvidia_nim", "ollama",
    ];
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let mut warnings = 0u32;
    let mode = if apply { "apply" } else { "dry-run" };
    println!("Catalog sync ({mode}) — live API vs {}", user_catalog_path().display());
    if !apply {
        println!("(use --apply to merge live-only models into the user catalog)");
    }
    println!();
    let mut total_added = 0u32;
    for id in providers {
        let prov = match parse_provider_str(id) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("  {id}: skip ({e})");
                continue;
            }
        };
        match rt.block_on(compare_models(prov, include_specialized)) {
            Ok(report) => {
                let drift = report.catalog_only.len() + report.live_only.len();
                if drift > 0 {
                    warnings += 1;
                }
                println!(
                    "{id:<12} live={} catalog_only={} live_only={}",
                    report.live_models.len(),
                    report.catalog_only.len(),
                    report.live_only.len()
                );
                if let Some(note) = &report.detail {
                    println!("             note: {note}");
                }
                for m in &report.catalog_only {
                    println!("             WARN catalog-only: {m}");
                }
                for m in &report.live_only {
                    println!("             WARN live-only: {m}");
                }
                if apply && !report.live_only.is_empty() {
                    match apply_live_models(id, &report.live_only) {
                        Ok((path, added)) => {
                            total_added += added.len() as u32;
                            if !added.is_empty() {
                                println!("             APPLY added {} model(s) → {}", added.len(), path.display());
                                for m in &added {
                                    println!("             + {m}");
                                }
                            }
                        }
                        Err(e) => println!("             APPLY ERROR: {e}"),
                    }
                }
            }
            Err(e) => println!("{id:<12} ERROR: {e}"),
        }
    }
    println!();
    if apply {
        if total_added > 0 {
            println!("Applied {total_added} new model(s) to {}", user_catalog_path().display());
        } else {
            println!("No new models to apply.");
        }
        0
    } else if warnings > 0 {
        println!("{warnings} provider(s) with catalog drift — run with --apply to merge live-only models");
        1
    } else {
        println!("No drift detected (or no live API access).");
        0
    }
}

pub async fn run_health(json: bool) -> i32 {
    use rmng_core::{budget_governance_report, check_budget_from_audit, AuditLog};
    use rmng_nervous::{
        circuit_state_path, health_check_detailed, list_circuit_statuses, reload_from_disk,
        NervousConnector,
    };
    use serde::Serialize;

    #[derive(Serialize)]
    struct HealthJson {
        provider_id: String,
        healthy: bool,
        model: String,
        api_key_set: bool,
        endpoint: Option<String>,
        detail: String,
        rmngd_running: bool,
        socket_path: String,
        circuit_state_path: String,
        circuits_open: u32,
        circuit_breakers: Vec<rmng_nervous::CircuitStatus>,
        budget: Option<rmng_core::BudgetCheckResult>,
        budgets: rmng_core::BudgetGovernanceReport,
    }

    reload_from_disk();
    let connector = NervousConnector::load();
    let cfg = connector.config().clone();
    match health_check_detailed(connector.config()).await {
        Ok(r) => {
            let circuits = list_circuit_statuses();
            let circuits_open = circuits.iter().filter(|c| c.open).count() as u32;
            let entries = AuditLog::new(AuditLog::default_path())
                .read_all()
                .unwrap_or_default();
            let agent_caps: Vec<(String, Option<f64>)> = rmng_nervous::AgentRegistry::load()
                .map(|reg| {
                    reg.agent_ids()
                        .into_iter()
                        .filter_map(|id| {
                            reg.get(&id)
                                .ok()
                                .map(|a| (a.id.clone(), a.daily_budget_usd))
                        })
                        .collect()
                })
                .unwrap_or_default();
            let budgets = budget_governance_report(&cfg, &entries, &agent_caps);
            let budget = check_budget_from_audit(&cfg);
            if json {
                let out = HealthJson {
                    provider_id: r.provider_id,
                    healthy: r.healthy,
                    model: r.model,
                    api_key_set: r.api_key_set,
                    endpoint: r.endpoint,
                    detail: r.detail,
                    rmngd_running: rmng_core::daemon_running(),
                    socket_path: rmng_core::socket_path().display().to_string(),
                    circuit_state_path: circuit_state_path().display().to_string(),
                    circuits_open,
                    circuit_breakers: circuits,
                    budget,
                    budgets,
                };
                println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
            } else {
                let status = if r.healthy { "healthy" } else { "unreachable" };
                println!("provider:  {}", r.provider_id);
                println!("status:    {status}");
                println!("model:     {}", r.model);
                println!("key_set:   {}", r.api_key_set);
                if let Some(ep) = &r.endpoint {
                    println!("endpoint:  {ep}");
                }
                println!("detail:    {}", r.detail);
                println!(
                    "rmngd:     {} ({})",
                    if rmng_core::daemon_running() {
                        "running"
                    } else {
                        "stopped"
                    },
                    rmng_core::socket_path().display()
                );
                if circuits.is_empty() {
                    println!("circuits:  none ({})", circuit_state_path().display());
                } else {
                    println!(
                        "circuits:  {} tracked ({})",
                        circuits.len(),
                        circuit_state_path().display()
                    );
                    for c in circuits.iter().filter(|c| c.open) {
                        println!(
                            "  OPEN {} failures={} cooldown={:?}s",
                            c.provider_id, c.failures, c.cooldown_secs_remaining
                        );
                    }
                }
                if let Some(b) = budget {
                    let tag = match b.level {
                        rmng_core::BudgetLevel::Ok => "ok",
                        rmng_core::BudgetLevel::Warn => "WARN",
                        rmng_core::BudgetLevel::Deny => "DENY",
                    };
                    println!("budget:    [{tag}] {}", b.message);
                }
                if let Some(p) = budgets.active_profile {
                    println!("profile:   {} — {}", p.id, p.check.message);
                }
                if circuits_open > 0 {
                    println!("alert:     {circuits_open} circuit breaker(s) OPEN — see circuits above");
                }
                println!();
                println!("tip: rmng health — full readiness + audit; rmng observe — deep metrics");
            }
            if r.healthy && circuits_open == 0 { 0 } else { 1 }
        }
        Err(e) => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "healthy": false,
                        "error": e.to_string(),
                    }))
                    .unwrap_or_default()
                );
            } else {
                eprintln!("{e}");
            }
            1
        }
    }
}
