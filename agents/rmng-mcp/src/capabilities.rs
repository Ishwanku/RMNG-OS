//! Capability dropping for MCP subprocesses (Sprint 21).

#[cfg(target_os = "linux")]
pub fn drop_all_capabilities() -> Result<(), String> {
    for set in [
        caps::CapSet::Effective,
        caps::CapSet::Permitted,
        caps::CapSet::Inheritable,
        caps::CapSet::Bounding,
        caps::CapSet::Ambient,
    ] {
        caps::clear(None, set).map_err(|e| format!("cap_drop({set:?}): {e}"))?;
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn drop_all_capabilities() -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drop_capabilities_no_panic() {
        let _ = drop_all_capabilities();
    }
}
