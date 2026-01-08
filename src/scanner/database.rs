use anyhow::{Context, Result};
use lru::LruCache;
use rayon::prelude::*;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct Signature {
    pub id: String,
    pub name: String,
    pub threat_type: String,
    pub risk_level: String,
    pub pattern: Vec<u8>,
    pub pattern_type: PatternType,
    pub target: String,
    pub subplatform: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PatternType {
    ByteSequence,
    ExtendedByteSequence,
    LogicalExpression,
    Regex,
    PEHeader,
    Hash,
}

#[derive(Debug)]
pub struct ThreatSignature {
    pub id: String,
    pub name: String,
    pub threat_type: String,
    pub risk_level: String,
    pub encrypted_pattern: Vec<u8>,
    pub pattern_type: PatternType,
    pub decompressed_size: u64,
    pub offset: u64,
    pub target: String,
}

pub struct SignatureDatabase {
    signatures: Arc<RwLock<HashMap<String, Signature>>>,
    signatures_by_type: Arc<RwLock<HashMap<String, Vec<String>>>>,
    hash_cache: Arc<Mutex<LruCache<String, String>>>,
    memory_usage: Arc<Mutex<u64>>,
    last_update: Arc<Mutex<Option<Instant>>>,
    version: Arc<Mutex<String>>,
}

impl SignatureDatabase {
    pub fn new() -> Self {
        Self {
            signatures: Arc::new(RwLock::new(HashMap::new())),
            signatures_by_type: Arc::new(RwLock::new(HashMap::new())),
            hash_cache: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(10000).unwrap()))),
            memory_usage: Arc::new(Mutex::new(0)),
            last_update: Arc::new(Mutex::new(None)),
            version: Arc::new(Mutex::new(String::from("0.0.0"))),
        }
    }

    pub async fn load_from_cvd<P: AsRef<Path>>(&self, path: P) -> Result<(), anyhow::Error> {
        log::info!("正在加载病毒库: {:?}", path.as_ref());

        let file = std::fs::File::open(path).context("无法打开病毒库文件")?;
        let reader = std::io::BufReader::new(file);

        let mut archive = zip::ZipArchive::new(reader).context("无法解析ZIP格式")?;

        let main_cvd = archive.by_name("main.cvd")?;

        let mut signatures = Vec::new();
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(main_cvd);

        for result in reader.records() {
            let record = result.context("无法读取CSV记录")?;
            let signature = Signature {
                id: record[0].to_string(),
                name: record[1].to_string(),
                threat_type: record[2].to_string(),
                risk_level: record[3].to_string(),
                pattern: hex::decode(&record[4]).context("无法解码特征码")?,
                pattern_type: Self::parse_pattern_type(&record[5]),
                target: record[6].to_string(),
                subplatform: record.get(7).map(|s| s.to_string()),
            };
            signatures.push(signature);
        }

        let mut sig_map = self.signatures.write().await;
        let mut type_map = self.signatures_by_type.write().await;

        for sig in signatures {
            sig_map.insert(sig.id.clone(), sig.clone());
            type_map
                .entry(sig.threat_type.clone())
                .or_insert_with(Vec::new)
                .push(sig.id.clone());
        }

        *self.memory_usage.lock().unwrap() = self.calculate_memory_usage();

        log::info!("已加载 {} 条病毒特征码", sig_map.len());

        Ok(())
    }

    pub async fn load_from_directory<P: AsRef<Path>>(
        &self,
        dir: P,
    ) -> Result<(), anyhow::Error> {
        log::info!("正在从目录加载病毒库: {:?}", dir.as_ref());

        let mut loaded_count = 0;

        for entry in WalkDir::new(dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".cvd"))
        {
            if self.load_from_cvd(entry.path()).await.is_ok() {
                loaded_count += 1;
            }
        }

        log::info!("已从 {} 个文件加载病毒库", loaded_count);

        Ok(())
    }

    pub async fn scan_file<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<Option<ThreatSignature>, anyhow::Error> {
        let path_str = path.as_ref().to_string_lossy().to_string();

        let mut cache = self.hash_cache.lock().unwrap();
        if let Some(cached) = cache.get(&path_str) {
            if let Some(sig_id) = self.signatures.read().await.get(cached) {
                return Ok(Some(ThreatSignature {
                    id: sig_id.id.clone(),
                    name: sig_id.id.clone(),
                    threat_type: sig_id.threat_type.clone(),
                    risk_level: sig_id.risk_level.clone(),
                    encrypted_pattern: sig_id.pattern.clone(),
                    pattern_type: sig_id.pattern_type,
                    decompressed_size: sig_id.pattern.len() as u64,
                    offset: 0,
                    target: sig_id.target.clone(),
                }));
            }
        }
        drop(cache);

        let file_data = match std::fs::read(path) {
            Ok(data) => data,
            Err(_) => return Ok(None),
        };

        let file_hash = Self::calculate_hash(&file_data);

        let mut signatures = self.signatures.write().await;
        if let Some(sig_id) = signatures.get(&file_hash) {
            let mut cache = self.hash_cache.lock().unwrap();
            cache.put(path_str, sig_id.id.clone());
            return Ok(Some(ThreatSignature {
                id: sig_id.id.clone(),
                name: sig_id.id.clone(),
                threat_type: sig_id.threat_type.clone(),
                risk_level: sig_id.risk_level.clone(),
                encrypted_pattern: sig_id.pattern.clone(),
                pattern_type: sig_id.pattern_type,
                decompressed_size: sig_id.pattern.len() as u64,
                offset: 0,
                target: sig_id.target.clone(),
            }));
        }

        drop(signatures);
        drop(file_data);

        Ok(None)
    }

    pub fn scan_file_sync<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Option<ThreatSignature> {
        let path_str = path.as_ref().to_string_lossy().to_string();

        let mut cache = self.hash_cache.lock().unwrap();
        if let Some(cached) = cache.get(&path_str) {
            let signatures = self.signatures.blocking_read();
            if let Some(sig_id) = signatures.get(cached) {
                return Some(ThreatSignature {
                    id: sig_id.id.clone(),
                    name: sig_id.name.clone(),
                    threat_type: sig_id.threat_type.clone(),
                    risk_level: sig_id.risk_level.clone(),
                    encrypted_pattern: sig_id.pattern.clone(),
                    pattern_type: sig_id.pattern_type,
                    decompressed_size: sig_id.pattern.len() as u64,
                    offset: 0,
                    target: sig_id.target.clone(),
                });
            }
        }
        drop(cache);

        let file_data = match std::fs::read(path.as_ref()) {
            Ok(data) => data,
            Err(_) => return None,
        };

        let file_hash = Self::calculate_hash(&file_data);

        let mut signatures = self.signatures.blocking_write();
        if let Some(sig_id) = signatures.get(&file_hash) {
            let mut cache = self.hash_cache.lock().unwrap();
            cache.put(path_str, sig_id.id.clone());
            return Some(ThreatSignature {
                id: sig_id.id.clone(),
                name: sig_id.name.clone(),
                threat_type: sig_id.threat_type.clone(),
                risk_level: sig_id.risk_level.clone(),
                encrypted_pattern: sig_id.pattern.clone(),
                pattern_type: sig_id.pattern_type,
                decompressed_size: sig_id.pattern.len() as u64,
                offset: 0,
                target: sig_id.target.clone(),
            });
        }

        None
    }

    fn match_pattern(
        data: &[u8],
        pattern: &[u8],
        pattern_type: PatternType,
    ) -> bool {
        match pattern_type {
            PatternType::ByteSequence => data.windows(pattern.len()).any(|w| w == pattern),
            PatternType::ExtendedByteSequence => {
                Self::match_extended_pattern(data, pattern)
            }
            _ => false,
        }
    }

    fn match_extended_pattern(data: &[u8], pattern: &[u8]) -> bool {
        let mut i = 0;
        let mut j = 0;

        while i < data.len() && j < pattern.len() {
            if pattern[j] == b'*' {
                return true;
            } else if pattern[j] == b'?' {
                i += 1;
                j += 1;
            } else {
                let mut k = 0;
                while k < pattern.len() - j && pattern[j + k] != b'*' && pattern[j + k] != b'?' {
                    k += 1;
                }
                if data[i..].starts_with(&pattern[j..j + k]) {
                    i += k;
                    j += k;
                } else {
                    return false;
                }
            }
        }

        j >= pattern.len()
    }

    fn calculate_hash(data: &[u8]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    fn parse_pattern_type(s: &str) -> PatternType {
        match s {
            "bytecode" => PatternType::ByteSequence,
            "extended" => PatternType::ExtendedByteSequence,
            "logical" => PatternType::LogicalExpression,
            "regex" => PatternType::Regex,
            "pe" => PatternType::PEHeader,
            "hash" => PatternType::Hash,
            _ => PatternType::ByteSequence,
        }
    }

    fn calculate_memory_usage(&self) -> u64 {
        self.signatures.blocking_read().values().map(|s| s.pattern.len() as u64).sum()
    }

    pub fn get_memory_usage(&self) -> u64 {
        *self.memory_usage.lock().unwrap()
    }

    pub fn get_signature_count(&self) -> usize {
        self.signatures.blocking_read().len()
    }

    pub fn get_last_update(&self) -> Option<Instant> {
        *self.last_update.lock().unwrap()
    }

    pub fn set_last_update(&self, time: Instant) {
        *self.last_update.lock().unwrap() = Some(time);
    }

    pub fn get_version(&self) -> String {
        self.version.lock().unwrap().clone()
    }

    pub fn set_version(&self, version: String) {
        *self.version.lock().unwrap() = version;
    }

    pub async fn update_signatures(
        &self,
        new_signatures: Vec<Signature>,
    ) -> Result<(), anyhow::Error> {
        let mut sig_map = self.signatures.write().await;
        let mut type_map = self.signatures_by_type.write().await;

        for sig in new_signatures {
            sig_map.insert(sig.id.clone(), sig.clone());
            type_map
                .entry(sig.threat_type.clone())
                .or_insert_with(Vec::new)
                .push(sig.id.clone());
        }

        *self.memory_usage.lock().unwrap() = self.calculate_memory_usage();

        Ok(())
    }
}

impl Default for SignatureDatabase {
    fn default() -> Self {
        Self::new()
    }
}
