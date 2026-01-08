use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct MonitorEvent {
    pub watch_path: PathBuf,
    pub event_type: EventType,
    pub file_path: PathBuf,
    pub cookie: u32,
    pub timestamp: u64,
    pub process_info: Option<ProcessInfo>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EventType {
    Created,
    Modified,
    Deleted,
    MovedFrom,
    MovedTo,
    Accessed,
}

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub command: String,
    pub user_id: u32,
    pub user_name: String,
}

#[cfg(target_os = "linux")]
mod linux_monitor {
    use super::*;
    use inotify::{Inotify, WatchMask};
    use std::thread;
    use std::time::Duration;
    use tokio::sync::mpsc;

    pub struct FileMonitor {
        inotify: Arc<Mutex<Option<Inotify>>>,
        running: Arc<AtomicBool>,
        watches: Arc<Mutex<HashMap<PathBuf, WatchMask>>>,
        event_callback: Arc<Mutex<Option<Arc<dyn Fn(MonitorEvent) + Send + Sync>>>>,
    }

    impl FileMonitor {
        pub fn new() -> Self {
            Self {
                inotify: Arc::new(Mutex::new(None)),
                running: Arc::new(AtomicBool::new(false)),
                watches: Arc::new(Mutex::new(HashMap::new())),
                event_callback: Arc::new(Mutex::new(None)),
            }
        }

        pub fn add_watch(&self, path: &PathBuf, mask: WatchMask) -> Result<(), anyhow::Error> {
            let mut inotify_guard = self.inotify.lock().unwrap();
            let inotify = inotify_guard
                .as_mut()
                .expect("监控器未初始化，请先调用start()");

            inotify
                .watches()
                .add(path.clone(), mask)
                .with_context(|| format!("无法监控路径: {:?}", path))?;

            let mut watches = self.watches.lock().unwrap();
            watches.insert(path.clone(), mask);

            log::info!("已添加监控: {:?}", path);
            Ok(())
        }

        pub fn remove_watch(&self, path: &PathBuf) -> Result<(), anyhow::Error> {
            let mut inotify_guard = self.inotify.lock().unwrap();
            let inotify = inotify_guard
                .as_mut()
                .expect("监控器未初始化，请先调用start()");

            if let Some(wd) = inotify.watches().find(path) {
                inotify.watches().remove(wd)?;
            }

            let mut watches = self.watches.lock().unwrap();
            watches.remove(path);

            log::info!("已移除监控: {:?}", path);
            Ok(())
        }

        pub fn add_default_watches(&self) -> Result<(), anyhow::Error> {
            let mask = WatchMask::CREATE | WatchMask::MODIFY;

            let default_paths = vec![
                PathBuf::from("/tmp"),
            ];

            for path in default_paths {
                if path.exists() {
                    self.add_watch(&path, mask)?;
                }
            }

            Ok(())
        }

        pub fn start(&mut self) -> Result<(), anyhow::Error> {
            if self.running.load(Ordering::Relaxed) {
                return Err(anyhow::anyhow!("监控器已在运行中"));
            }

            let inotify = Inotify::init()
                .context("无法初始化inotify")?;

            {
                let mut guard = self.inotify.lock().unwrap();
                *guard = Some(inotify);
            }

            self.running.store(true, Ordering::Relaxed);

            let inotify = Arc::clone(&self.inotify);
            let running = Arc::clone(&self.running);
            let watches = Arc::clone(&self.watches);
            let event_callback = Arc::clone(&self.event_callback);

            thread::spawn(move || {
                log::info!("文件监控线程已启动");

                while running.load(Ordering::Relaxed) {
                    let mut buffer = [0u8; 1024];
                    let mut inotify_guard = inotify.lock().unwrap();

                    if let Some(ref inotify) = *inotify_guard {
                        match inotify.read_events(&mut buffer) {
                            Ok(events) => {
                                for event in events {
                                    let watch_path = PathBuf::from("/tmp");
                                    let (event_type, file_name) = Self::parse_event(
                                        event.mask,
                                        event.name,
                                    );

                                    if let Some(name) = file_name {
                                        let file_path = watch_path.join(&name);
                                        let timestamp = std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_secs();

                                        let monitor_event = MonitorEvent {
                                            watch_path,
                                            event_type,
                                            file_path,
                                            cookie: event.cookie,
                                            timestamp,
                                            process_info: None,
                                        };

                                        if let Some(ref callback) = *event_callback.lock().unwrap() {
                                            callback(monitor_event);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("读取inotify事件失败: {}", e);
                            }
                        }
                    }

                    drop(inotify_guard);
                    thread::sleep(Duration::from_millis(500));
                }

                log::info!("文件监控线程已停止");
            });

            log::info!("文件监控服务已启动");
            Ok(())
        }

        fn parse_event(mask: inotify::EventMask, name: Option<&std::ffi::OsStr>) -> (EventType, Option<String>) {
            let file_name = name.and_then(|n| n.to_str().map(|s| s.to_string()));
            
            if mask.contains(inotify::EventMask::CREATE) {
                return (EventType::Created, file_name);
            }
            if mask.contains(inotify::EventMask::MODIFY) {
                return (EventType::Modified, file_name);
            }
            if mask.contains(inotify::EventMask::DELETE) {
                return (EventType::Deleted, file_name);
            }
            if mask.contains(inotify::EventMask::MOVED_FROM) {
                return (EventType::MovedFrom, file_name);
            }
            if mask.contains(inotify::EventMask::MOVED_TO) {
                return (EventType::MovedTo, file_name);
            }
            if mask.contains(inotify::EventMask::ACCESS) {
                return (EventType::Accessed, file_name);
            }
            (EventType::Modified, file_name)
        }

        pub fn stop(&mut self) {
            self.running.store(false, Ordering::Relaxed);

            let mut guard = self.inotify.lock().unwrap();
            if let Some(ref mut inotify) = *guard {
                let watch_paths: Vec<PathBuf> = self.watches.lock().unwrap().keys().cloned().collect();
                for path in &watch_paths {
                    if let Ok(wd) = inotify.watches().add(path.clone(), WatchMask::empty()) {
                        let _ = inotify.watches().remove(wd);
                    }
                }
                self.watches.lock().unwrap().clear();
            }

            log::info!("文件监控服务已停止");
        }

        pub fn set_event_callback(&mut self, callback: Arc<dyn Fn(MonitorEvent) + Send + Sync>) {
            let mut cb = self.event_callback.lock().unwrap();
            *cb = Some(callback);
        }

        pub fn is_running(&self) -> bool {
            self.running.load(Ordering::Relaxed)
        }

        pub fn get_watched_paths(&self) -> Vec<PathBuf> {
            self.watches.lock().unwrap().keys().cloned().collect()
        }
    }

    pub use FileMonitor;
}

#[cfg(not(target_os = "linux"))]
mod stub_monitor {
    use super::*;

    pub struct FileMonitor;

    impl FileMonitor {
        pub fn new() -> Self {
            Self
        }

        pub fn add_watch(&self, _path: &PathBuf, _mask: u32) -> Result<(), anyhow::Error> {
            Err(anyhow::anyhow!("文件监控仅在Linux系统上可用"))
        }

        pub fn remove_watch(&self, _path: &PathBuf) -> Result<(), anyhow::Error> {
            Err(anyhow::anyhow!("文件监控仅在Linux系统上可用"))
        }

        pub fn add_default_watches(&self) -> Result<(), anyhow::Error> {
            Err(anyhow::anyhow!("文件监控仅在Linux系统上可用"))
        }

        pub fn start(&mut self) -> Result<(), anyhow::Error> {
            Err(anyhow::anyhow!("文件监控仅在Linux系统上可用"))
        }

        pub fn stop(&mut self) {
            log::warn!("文件监控仅在Linux系统上可用");
        }

        pub fn set_event_callback(&mut self, _callback: Arc<dyn Fn(MonitorEvent) + Send + Sync>) {
        }

        pub fn is_running(&self) -> bool {
            false
        }

        pub fn get_watched_paths(&self) -> Vec<PathBuf> {
            Vec::new()
        }
    }
}

#[cfg(target_os = "linux")]
pub use linux_monitor::FileMonitor;

#[cfg(not(target_os = "linux"))]
pub use stub_monitor::FileMonitor;
