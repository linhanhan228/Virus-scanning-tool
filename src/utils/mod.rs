pub mod logging;

use path_absolutize::Absolutize;
use std::path::{Path, PathBuf};
use std::time::Duration;
use users::{get_user_by_uid, get_group_by_gid};

pub fn get_current_user() -> Result<String, anyhow::Error> {
    let uid = users::get_current_uid();
    let user = get_user_by_uid(uid)
        .ok_or_else(|| anyhow::anyhow!("无法获取当前用户信息"))?;
    Ok(user.name().to_string_lossy().to_string())
}

pub fn get_current_group() -> Result<String, anyhow::Error> {
    let gid = users::get_current_gid();
    let group = get_group_by_gid(gid)
        .ok_or_else(|| anyhow::anyhow!("无法获取当前用户组信息"))?;
    Ok(group.name().to_string_lossy().to_string())
}

pub fn drop_privileges() -> Result<(), anyhow::Error> {
    if users::get_current_uid() == 0 {
        let nobody = users::get_user_by_name("nobody")
            .ok_or_else(|| anyhow::anyhow!("无法找到nobody用户"))?;

        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        {
            use nix::unistd::{Gid, Group};
            let gid = Gid::from_raw(nobody.primary_group_id().as_raw());
            let _ = nix::unistd::setgroups(&[]);
            nix::unistd::setgid(gid)?;
        }
        
        nix::unistd::setuid(nobody.uid().into())?;
    }
    Ok(())
}

pub fn format_bytes(size: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < units.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, units[unit_index])
}

pub fn get_file_hash(path: &Path) -> Result<String, anyhow::Error> {
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open(path)?;
    let mut hasher = crc32fast::Hasher::new();
    let mut buffer = vec![0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:08x}", hasher.finalize()))
}

pub fn get_file_size(path: &Path) -> Result<u64, anyhow::Error> {
    let metadata = std::fs::metadata(path)?;
    Ok(metadata.len())
}

pub fn get_file_permissions(path: &Path) -> String {
    if let Ok(metadata) = std::fs::metadata(path) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = metadata.permissions().mode();
            format!("{:o}", mode & 0o777)
        }
        #[cfg(not(unix))]
        {
            "unknown".to_string()
        }
    } else {
        "unknown".to_string()
    }
}

pub fn is_executable(path: &Path) -> bool {
    if let Ok(metadata) = std::fs::metadata(path) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = metadata.permissions().mode();
            mode & 0o111 != 0
        }
        #[cfg(not(unix))]
        {
            false
        }
    } else {
        false
    }
}

pub fn is_symlink(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

pub fn normalize_path(path: &Path) -> Result<PathBuf, anyhow::Error> {
    Ok(path.absolutize()?.to_path_buf())
}

pub fn create_directory(path: &Path, recursive: bool) -> Result<(), anyhow::Error> {
    if recursive {
        std::fs::create_dir_all(path)?;
    } else {
        std::fs::create_dir(path)?;
    }
    Ok(())
}

pub fn copy_file(src: &Path, dst: &Path) -> Result<(), anyhow::Error> {
    std::fs::copy(src, dst)?;
    Ok(())
}

pub fn move_file(src: &Path, dst: &Path) -> Result<(), anyhow::Error> {
    std::fs::rename(src, dst)?;
    Ok(())
}

pub fn delete_file(path: &Path) -> Result<(), anyhow::Error> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

pub fn quarantine_file(path: &Path, quarantine_dir: &Path) -> Result<PathBuf, anyhow::Error> {
    let file_name = path.file_name()
        .ok_or_else(|| anyhow::anyhow!("无效的文件名"))?;

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let quarantine_path = quarantine_dir.join(format!("{}_{}", timestamp, file_name.to_string_lossy()));

    std::fs::create_dir_all(quarantine_dir)?;
    copy_file(path, &quarantine_path)?;
    delete_file(path)?;

    Ok(quarantine_path)
}
