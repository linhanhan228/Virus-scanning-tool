pub mod engine;
mod database;

pub use engine::{ScannerEngine, ScanOptions, ScanMode, ScanResult, ScanStats, ThreatType, RiskLevel, FileInfo};
pub use database::{SignatureDatabase, Signature, PatternType, ThreatSignature};
