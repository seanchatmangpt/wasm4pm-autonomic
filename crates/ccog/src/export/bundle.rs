//! Deterministic `.tar.zst` proof bundle (Phase 11).

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};

use crate::trace::BenchmarkTier;

/// In-memory proof bundle entries — keyed by file name, ordered.
#[derive(Clone, Debug, Default)]
pub struct ProofBundle {
    /// File-name → bytes.
    pub entries: BTreeMap<String, Vec<u8>>,
}

/// Errors produced by bundle write/read.
#[derive(Debug, thiserror::Error)]
pub enum BundleError {
    /// I/O failure underlying tar/zstd.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// Manifest hash did not match recomputed entry digest.
    #[error("manifest mismatch for entry '{name}': declared {declared}, actual {actual}")]
    ManifestMismatch {
        /// Entry name with mismatched hash.
        name: String,
        /// Hash recorded in `manifest.json`.
        declared: String,
        /// Hash recomputed over the entry bytes during read.
        actual: String,
    },
    /// Required entry was missing from the bundle.
    #[error("missing required entry: {0}")]
    MissingEntry(String),
    /// Manifest JSON was malformed.
    #[error("malformed manifest: {0}")]
    MalformedManifest(String),
}

impl ProofBundle {
    /// Build a fresh bundle from raw artifact bytes.
    pub fn build(
        trace_jsonld: Vec<u8>,
        receipt_jsonld: Vec<u8>,
        powl64_path: Vec<u8>,
        ontology_refs: Vec<String>,
        tier: BenchmarkTier,
    ) -> Self {
        let mut entries: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        let mut refs_blob = ontology_refs.join("\n");
        if !refs_blob.is_empty() {
            refs_blob.push('\n');
        }
        entries.insert("ontology-refs.txt".into(), refs_blob.into_bytes());
        entries.insert("powl64-path.bin".into(), powl64_path);
        entries.insert("receipt.jsonld".into(), receipt_jsonld);
        entries.insert("tier.txt".into(), format!("{:?}\n", tier).into_bytes());
        entries.insert("trace.jsonld".into(), trace_jsonld);

        let manifest = manifest_json(&entries);
        entries.insert("manifest.json".into(), manifest);

        Self { entries }
    }

    /// Serialize the bundle to deterministic `.tar.zst` bytes.
    ///
    /// # Errors
    ///
    /// Returns `Err(BundleError::Io)` if the tar writer or zstd encoder fails.
    pub fn write(&self) -> Result<Vec<u8>, BundleError> {
        let mut tar_bytes: Vec<u8> = Vec::new();
        {
            let mut builder = tar::Builder::new(&mut tar_bytes);
            builder.mode(tar::HeaderMode::Deterministic);
            for (name, data) in &self.entries {
                let mut header = tar::Header::new_gnu();
                header.set_size(data.len() as u64);
                header.set_mode(0o644);
                header.set_uid(0);
                header.set_gid(0);
                header.set_mtime(0);
                header.set_entry_type(tar::EntryType::Regular);
                header.set_cksum();
                builder.append_data(&mut header, name, Cursor::new(data))?;
            }
            builder.finish()?;
        }

        let mut compressed: Vec<u8> = Vec::new();
        {
            let mut enc = zstd::Encoder::new(&mut compressed, 19)?;
            enc.write_all(&tar_bytes)?;
            enc.finish()?;
        }
        Ok(compressed)
    }

    /// Read a `.tar.zst` byte slice into a [`ProofBundle`], validating manifest hashes.
    ///
    /// # Errors
    ///
    /// - [`BundleError::Io`] if the zstd / tar layers fail to decode.
    /// - [`BundleError::MissingEntry`] / [`BundleError::MalformedManifest`] /
    ///   [`BundleError::ManifestMismatch`] on corruption.
    pub fn read(bytes: &[u8]) -> Result<Self, BundleError> {
        let mut decoded: Vec<u8> = Vec::new();
        let mut dec = zstd::Decoder::new(bytes)?;
        dec.read_to_end(&mut decoded)?;

        let mut entries: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        let mut archive = tar::Archive::new(Cursor::new(&decoded));
        for entry in archive.entries()? {
            let mut e = entry?;
            let path = e.path()?.to_string_lossy().to_string();
            let mut buf = Vec::new();
            e.read_to_end(&mut buf)?;
            entries.insert(path, buf);
        }

        let manifest_bytes = entries
            .get("manifest.json")
            .ok_or_else(|| BundleError::MissingEntry("manifest.json".into()))?;
        let manifest: serde_json::Value = serde_json::from_slice(manifest_bytes)
            .map_err(|e| BundleError::MalformedManifest(e.to_string()))?;
        let map = manifest
            .as_object()
            .ok_or_else(|| BundleError::MalformedManifest("not a JSON object".into()))?;

        for (name, declared) in map {
            let declared = declared
                .as_str()
                .ok_or_else(|| BundleError::MalformedManifest(format!("{} is not string", name)))?;
            let data = entries
                .get(name)
                .ok_or_else(|| BundleError::MissingEntry(name.clone()))?;
            let actual = blake3::hash(data).to_hex().to_string();
            if actual != declared {
                return Err(BundleError::ManifestMismatch {
                    name: name.clone(),
                    declared: declared.to_string(),
                    actual,
                });
            }
        }

        Ok(Self { entries })
    }

    /// Get an entry by name, or `None` if absent.
    #[must_use]
    pub fn entry(&self, name: &str) -> Option<&[u8]> {
        self.entries.get(name).map(Vec::as_slice)
    }
}

fn manifest_json(entries: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let mut map = serde_json::Map::new();
    for (name, data) in entries {
        let h = blake3::hash(data).to_hex().to_string();
        map.insert(name.clone(), serde_json::Value::String(h));
    }
    let v = serde_json::Value::Object(map);
    serde_json::to_vec(&v).expect("manifest is serializable")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> ProofBundle {
        ProofBundle::build(
            b"{\"trace\":1}".to_vec(),
            b"{\"receipt\":1}".to_vec(),
            vec![0u8; 32],
            vec![
                "urn:blake3:abc".to_string(),
                "http://www.w3.org/ns/prov#Activity".to_string(),
            ],
            BenchmarkTier::ConformanceReplay,
        )
    }

    #[test]
    fn bundle_write_read_roundtrip() {
        let b = fixture();
        let bytes = b.write().expect("write");
        let b2 = ProofBundle::read(&bytes).expect("read");
        assert_eq!(b.entries, b2.entries);
    }

    #[test]
    fn bundle_deterministic_bytes() {
        let b = fixture();
        let x = b.write().expect("write 1");
        let y = b.write().expect("write 2");
        assert_eq!(x, y, "bundle bytes must be deterministic");
    }
}
