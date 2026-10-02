//! `worker::Headers`, native (see `wire.rs`).

use worker::Result;

/// Header names are case-insensitive; stored lower-case, set replaces.
#[derive(Clone, Debug, Default)]
pub struct Fields(Vec<(String, String)>);

impl Fields {
    pub fn new() -> Self {
        Self::default()
    }
    #[cfg(test)]
    pub fn has(&self, name: &str) -> Result<bool> {
        Ok(self.get(name)?.is_some())
    }
    pub fn append(&mut self, name: &str, value: &str) -> Result<()> {
        self.0.push((name.to_ascii_lowercase(), value.to_string()));
        Ok(())
    }
    #[cfg(test)]
    pub fn delete(&mut self, name: &str) -> Result<()> {
        let n = name.to_ascii_lowercase();
        self.0.retain(|(k, _)| *k != n);
        Ok(())
    }
    pub fn get(&self, name: &str) -> Result<Option<String>> {
        let n = name.to_ascii_lowercase();
        Ok(self.0.iter().find(|(k, _)| *k == n).map(|(_, v)| v.clone()))
    }
    pub fn set(&mut self, name: &str, value: &str) -> Result<()> {
        let n = name.to_ascii_lowercase();
        self.0.retain(|(k, _)| *k != n);
        self.0.push((n, value.to_string()));
        Ok(())
    }
    /// The platform's headers, for a response that leaves as a `worker::Response`.
    pub fn into_worker(&self) -> Result<worker::Headers> {
        let h = worker::Headers::new();
        for (k, v) in &self.0 {
            h.append(k, v)?;
        }
        Ok(h)
    }
    pub fn entries(&self) -> impl Iterator<Item = &(String, String)> {
        self.0.iter()
    }
}
