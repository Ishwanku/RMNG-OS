use rmng_core::{parse_provider_str, LlmProviderKind, RmngConfig};
use rmng_nervous::{
    catalog_path, compare_models, default_model, install_user_catalog, list_all_providers,
    list_catalog_models, load_catalog, resolve_api_key,
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

pub fn print_models(provider: Option<&str>, include_specialized: bool, live: bool) {
    let prov = match provider {
        Some(s) => parse_provider_str(s).unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(1);
        }),
        None => RmngConfig::load().resolved_llm().llm_provider,
    };

    if live {
        print_models_live(prov, include_specialized);
        return;
    }

    let models = list_catalog_models(prov, include_specialized);
    if models.is_empty() {
        println!("No catalog models for {prov:?}");
        return;
    }
    println!("Models for {:?} (catalog):", prov);
    for m in models {
        print_catalog_entry(&m);
    }
}

fn print_catalog_entry(m: &rmng_nervous::ModelEntry) {
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

fn print_models_live(prov: LlmProviderKind, include_specialized: bool) {
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
            for m in list_catalog_models(prov, include_specialized) {
                print_catalog_entry(&m);
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

/// Compare live provider APIs against local catalog for all wired providers (Sprint 8).
pub fn run_sync_catalog(include_specialized: bool) -> i32 {
    let providers = [
        "grok", "openai", "groq", "google", "anthropic", "together", "fireworks", "deepseek",
        "nvidia_nim", "ollama",
    ];
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let mut warnings = 0u32;
    println!("Catalog sync — live API vs ~/.rmng/llm-catalog.toml");
    println!();
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
                for m in report.live_only.iter().take(5) {
                    println!("             WARN live-only: {m}");
                }
            }
            Err(e) => println!("{id:<12} ERROR: {e}"),
        }
    }
    println!();
    if warnings > 0 {
        println!("{warnings} provider(s) with catalog drift — edit ~/.rmng/llm-catalog.toml");
        1
    } else {
        println!("No drift detected (or no live API access).");
        0
    }
}