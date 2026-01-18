//! Metalink file parser for extracting checksums
//!
//! Metalink files (RFC 5854) contain file metadata including MD5/SHA checksums.
//! UniProt provides RELEASE.metalink with MD5 hashes for all files.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metalink root structure
#[derive(Debug, Deserialize, Serialize)]
struct Metalink {
    #[serde(rename = "file", default)]
    files: Vec<MetalinkFile>,
}

/// Individual file entry in metalink
#[derive(Debug, Deserialize, Serialize)]
struct MetalinkFile {
    name: String,
    #[serde(rename = "verification", default)]
    verification: Option<Verification>,
}

/// Verification block containing hash
#[derive(Debug, Deserialize, Serialize)]
struct Verification {
    #[serde(rename = "hash", default)]
    hashes: Vec<Hash>,
}

/// Hash entry (MD5, SHA1, SHA256, etc.)
#[derive(Debug, Deserialize, Serialize)]
struct Hash {
    #[serde(rename = "type")]
    hash_type: String,
    #[serde(rename = "$value")]
    value: String,
}

/// Parsed metalink with file-to-MD5 mapping
#[derive(Debug, Clone)]
pub struct MetalinkInfo {
    /// Map of filename to MD5 hash
    pub file_md5s: HashMap<String, String>,
}

impl MetalinkInfo {
    /// Parse metalink XML content
    pub fn parse(content: &str) -> Result<Self> {
        let metalink: Metalink = quick_xml::de::from_str(content)
            .context("Failed to parse metalink XML")?;

        let mut file_md5s = HashMap::new();

        for file in metalink.files {
            if let Some(verification) = file.verification {
                // Find MD5 hash
                for hash in verification.hashes {
                    if hash.hash_type.eq_ignore_ascii_case("md5") {
                        file_md5s.insert(file.name.clone(), hash.value.clone());
                        break;
                    }
                }
            }
        }

        Ok(Self { file_md5s })
    }

    /// Get MD5 for a specific file
    pub fn get_md5(&self, filename: &str) -> Option<&str> {
        self.file_md5s.get(filename).map(|s| s.as_str())
    }

    /// Get MD5 for file matching pattern (e.g., "uniprot_sprot.dat.gz")
    pub fn find_md5(&self, pattern: &str) -> Option<&str> {
        self.file_md5s
            .iter()
            .find(|(name, _)| name.contains(pattern))
            .map(|(_, md5)| md5.as_str())
    }

    /// List all files with their MD5s
    pub fn list_files(&self) -> Vec<(&str, &str)> {
        self.file_md5s
            .iter()
            .map(|(name, md5)| (name.as_str(), md5.as_str()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_metalink() {
        let xml = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <metalink xmlns="urn:ietf:params:xml:ns:metalink">
            <file name="uniprot_sprot.dat.gz">
                <verification>
                    <hash type="md5">e3cd39d0c48231aa5abb3eca81b3c62a</hash>
                </verification>
            </file>
            <file name="uniprot_sprot.fasta.gz">
                <verification>
                    <hash type="md5">1234567890abcdef1234567890abcdef</hash>
                </verification>
            </file>
        </metalink>
        "#;

        let info = MetalinkInfo::parse(xml).unwrap();

        assert_eq!(info.file_md5s.len(), 2);
        assert_eq!(
            info.get_md5("uniprot_sprot.dat.gz"),
            Some("e3cd39d0c48231aa5abb3eca81b3c62a")
        );
        assert_eq!(
            info.find_md5("fasta"),
            Some("1234567890abcdef1234567890abcdef")
        );
    }

    #[test]
    fn test_find_md5_pattern() {
        let xml = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <metalink xmlns="urn:ietf:params:xml:ns:metalink">
            <file name="knowledgebase/complete/uniprot_sprot.dat.gz">
                <verification>
                    <hash type="md5">abc123</hash>
                </verification>
            </file>
        </metalink>
        "#;

        let info = MetalinkInfo::parse(xml).unwrap();

        // Find by pattern
        assert_eq!(info.find_md5("dat.gz"), Some("abc123"));
        assert_eq!(info.find_md5("sprot"), Some("abc123"));
        assert_eq!(info.find_md5("trembl"), None);
    }

    #[test]
    fn test_list_files() {
        let xml = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <metalink xmlns="urn:ietf:params:xml:ns:metalink">
            <file name="file1.dat.gz">
                <verification>
                    <hash type="md5">md5_1</hash>
                </verification>
            </file>
            <file name="file2.fasta.gz">
                <verification>
                    <hash type="md5">md5_2</hash>
                </verification>
            </file>
        </metalink>
        "#;

        let info = MetalinkInfo::parse(xml).unwrap();
        let files = info.list_files();

        assert_eq!(files.len(), 2);
        assert!(files.contains(&("file1.dat.gz", "md5_1")));
        assert!(files.contains(&("file2.fasta.gz", "md5_2")));
    }
}
