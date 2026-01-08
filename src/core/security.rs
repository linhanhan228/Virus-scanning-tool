use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use crate::utils::AuditLogger;

pub struct SecurityManager {
    audit_logger: AuditLogger,
    failed_attempts: Arc<Mutex<Vec<FailedLogin>>>,
    lockout_threshold: usize,
    lockout_duration: u64,
}

struct FailedLogin {
    timestamp: Instant,
    username: String,
    ip_address: String,
}

impl SecurityManager {
    pub fn new(
        audit_log_path: PathBuf,
        lockout_threshold: usize,
        lockout_duration: u64,
    ) -> Self {
        Self {
            audit_logger: AuditLogger::new(audit_log_path, true),
            failed_attempts: Arc::new(Mutex::new(Vec::new())),
            lockout_threshold,
            lockout_duration,
        }
    }

    pub fn is_locked_out(&self, username: &str, ip: &str) -> bool {
        let now = Instant::now();
        let mut attempts = self.failed_attempts.lock().unwrap();

        attempts.retain(|attempt| {
            now.duration_since(attempt.timestamp).as_secs() < self.lockout_duration
        });

        let user_attempts: Vec<_> = attempts
            .iter()
            .filter(|a| a.username == username && a.ip_address == ip)
            .collect();

        user_attempts.len() >= self.lockout_threshold
    }

    pub fn record_failed_attempt(&self, username: &str, ip: &str) {
        self.failed_attempts.lock().unwrap().push(FailedLogin {
            timestamp: Instant::now(),
            username: username.to_string(),
            ip_address: ip.to_string(),
        });

        self.audit_logger.log(
            "LOGIN_FAILED",
            username,
            &format!("IP: {}", ip),
        );
    }

    pub fn record_success(&self, username: &str, ip: &str) {
        self.audit_logger.log(
            "LOGIN_SUCCESS",
            username,
            &format!("IP: {}", ip),
        );
    }

    pub fn log_operation(&self, operation: &str, user: &str, details: &str) {
        self.audit_logger.log(operation, user, details);
    }
}

pub struct QuarantineManager {
    quarantine_dir: PathBuf,
    encryption_key: Option<Vec<u8>>,
}

impl QuarantineManager {
    pub fn new(quarantine_dir: PathBuf, encryption_key: Option<Vec<u8>>) -> Self {
        std::fs::create_dir_all(&quarantine_dir).ok();
        Self {
            quarantine_dir,
            encryption_key,
        }
    }

    pub async fn quarantine_file(
        &self,
        file_path: &PathBuf,
    ) -> Result<PathBuf, anyhow::Error> {
        let file_name = file_path.file_name()
            .ok_or_else(|| anyhow::anyhow!("无效的文件名"))?;

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let quarantine_name = format!("{}_{}", timestamp, file_name.to_string_lossy());
        let quarantine_path = self.quarantine_dir.join(&quarantine_name);

        if let Some(ref key) = self.encryption_key {
            self.encrypt_and_copy(file_path, &quarantine_path, key).await?;
        } else {
            std::fs::copy(file_path, &quarantine_path)?;
        }

        std::fs::remove_file(file_path)?;

        Ok(quarantine_path)
    }

    async fn encrypt_and_copy(
        &self,
        src: &PathBuf,
        dst: &PathBuf,
        key: &[u8],
    ) -> Result<(), anyhow::Error> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let mut src_file = tokio::fs::File::open(src).await?;
        let mut content = Vec::new();
        src_file.read_to_end(&mut content).await?;

        let encrypted = self.encrypt_aes_256_gcm(&content, key)?;

        let mut dst_file = tokio::fs::File::create(dst).await?;
        dst_file.write_all(&encrypted).await?;

        Ok(())
    }

    fn encrypt_aes_256_gcm(
        &self,
        data: &[u8],
        key: &[u8],
    ) -> Result<Vec<u8>, anyhow::Error> {
        use aes::Aes256;
        use ctr::Ctr128BE;
        use crypto_mac::Hmac;
        use crypto_mac::NewMac;

        let cipher = Aes256::new_from_slice(key)
            .map_err(|e| anyhow::anyhow!("密钥错误: {}", e))?;
        let mut cipher = Ctr128BE::new(cipher, &Default::default());

        let mut encrypted = vec![0u8; data.len()];
        cipher.encrypt(data, &mut encrypted);

        let mut hmac = Hmac::<sha2::Sha256>::new_from_slice(key)
            .map_err(|e| anyhow::anyhow!("HMAC错误: {}", e))?;
        hmac.update(&encrypted);
        let tag = hmac.finalize().into_bytes();

        let mut result = encrypted;
        result.extend_from_slice(&tag);

        Ok(result)
    }

    pub fn restore_file(&self, quarantine_path: &PathBuf) -> Result<PathBuf, anyhow::Error> {
        if !quarantine_path.exists() {
            return Err(anyhow::anyhow!("文件不存在"));
        }

        let file_name = quarantine_path.file_name()
            .ok_or_else(|| anyhow::anyhow!("无效的文件名"))?;

        let parts: Vec<&str> = file_name.to_string_lossy().splitn(2, '_').collect();
        if parts.len() < 2 {
            return Err(anyhow::anyhow!("文件名格式错误"));
        }

        let original_name = parts[1];
        let restore_path = std::env::current_dir()?.join(original_name);

        if let Some(ref key) = self.encryption_key {
            self.decrypt_and_copy(quarantine_path, &restore_path, key)?;
        } else {
            std::fs::copy(quarantine_path, &restore_path)?;
        }

        Ok(restore_path)
    }

    fn decrypt_and_copy(
        &self,
        src: &PathBuf,
        dst: &PathBuf,
        key: &[u8],
    ) -> Result<(), anyhow::Error> {
        let content = std::fs::read(src)?;

        if content.len() < 32 {
            return Err(anyhow::anyhow!("文件格式错误"));
        }

        let data_len = content.len() - 32;
        let (encrypted, tag) = content.split_at(data_len);

        let mut hmac = Hmac::<sha2::Sha256>::new_from_slice(key)
            .map_err(|e| anyhow::anyhow!("HMAC错误: {}", e))?;
        hmac.update(encrypted);
        hmac.verify(tag)
            .map_err(|e| anyhow::anyhow!("验证失败: {}", e))?;

        let cipher = Aes256::new_from_slice(key)
            .map_err(|e| anyhow::anyhow!("密钥错误: {}", e))?;
        let mut cipher = Ctr128BE::new(cipher, &Default::default());

        let mut decrypted = vec![0u8; data_len];
        cipher.decrypt(encrypted, &mut decrypted);

        std::fs::write(dst, &decrypted)?;

        Ok(())
    }

    pub fn delete_quarantined(&self, quarantine_path: &PathBuf) -> Result<(), anyhow::Error> {
        if quarantine_path.exists() {
            std::fs::remove_file(quarantine_path)?;
        }
        Ok(())
    }
}

pub struct PermissionManager {
    required_capabilities: Vec<&'static str>,
    running_as_root: bool,
}

impl PermissionManager {
    pub fn new() -> Self {
        let running_as_root = users::get_current_uid() == 0;

        Self {
            required_capabilities: vec![
                "CAP_DAC_READ_SEARCH",
                "CAP_NET_RAW",
            ],
            running_as_root,
        }
    }

    pub fn check_capabilities(&self) -> Result<(), anyhow::Error> {
        if !self.running_as_root {
            return Ok(());
        }

        Ok(())
    }

    pub fn drop_root_privileges(&self, run_as_user: &str) -> Result<(), anyhow::Error> {
        if !self.running_as_root {
            return Ok(());
        }

        let user = users::get_user_by_name(run_as_user)
            .ok_or_else(|| anyhow::anyhow!("用户不存在: {}", run_as_user))?;

        nix::unistd::setgroups(&[])?;
        nix::unistd::setgid(user.primary_gid())?;
        nix::unistd::setuid(user.uid())?;

        Ok(())
    }

    pub fn is_privileged(&self) -> bool {
        self.running_as_root
    }
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new()
    }
}
