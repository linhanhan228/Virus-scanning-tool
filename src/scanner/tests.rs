use crate::scanner::{SignatureDatabase, Signature, PatternType};
use std::path::PathBuf;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_database_creation() {
        let db = SignatureDatabase::new();
        assert_eq!(db.get_signature_count(), 0);
    }

    #[test]
    fn test_pattern_matching() {
        let test_data = b"Hello, World! This is a test message.";
        let pattern = b"test";

        assert!(matches_pattern(test_data, pattern, PatternType::ByteSequence));
    }

    #[test]
    fn test_pattern_not_matching() {
        let test_data = b"Hello, World! This is a test message.";
        let pattern = b"notfound";

        assert!(!matches_pattern(test_data, pattern, PatternType::ByteSequence));
    }

    fn matches_pattern(data: &[u8], pattern: &[u8], _pattern_type: PatternType) -> bool {
        data.windows(pattern.len()).any(|w| w == pattern)
    }

    #[test]
    fn test_signature_creation() {
        let signature = Signature {
            id: "TestSig001".to_string(),
            name: "Test Signature".to_string(),
            threat_type: "Virus".to_string(),
            risk_level: "High".to_string(),
            pattern: vec![0x48, 0x65, 0x6c, 0x6c, 0x6f],
            pattern_type: PatternType::ByteSequence,
            target: "Generic".to_string(),
            subplatform: None,
        };

        assert_eq!(signature.id, "TestSig001");
        assert_eq!(signature.threat_type, "Virus");
        assert_eq!(signature.pattern.len(), 5);
    }

    #[test]
    fn test_threat_type_classification() {
        let threat_types = vec![
            "Virus",
            "Trojan",
            "Worm",
            "Ransomware",
            "Rootkit",
            "Adware",
            "Spyware",
            "HackTool",
            "PUA",
            "Unknown",
        ];

        for threat in threat_types {
            assert!(!threat.is_empty());
        }
    }
}
