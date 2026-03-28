pub mod schema;

pub fn load_config() -> anyhow::Result<schema::Config> {
    // Stub — real implementation in Wave 1 (Plan 02)
    Ok(Default::default())
}

#[cfg(test)]
mod tests {
    #[test]
    fn stub_returns_default() {
        let cfg = super::load_config().expect("stub must not fail");
        assert!(cfg.scan.exclude.is_empty());
    }
}
