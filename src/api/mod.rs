use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use warp::{Filter, Rejection, Reply};
use rand::Rng;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanRequest {
    pub scan_type: String,
    pub paths: Vec<String>,
    pub exclude_paths: Vec<String>,
    pub thread_count: Option<usize>,
    pub generate_report: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResponse {
    pub scan_id: String,
    pub status: String,
    pub threats_found: usize,
    pub files_scanned: usize,
    pub scan_speed_mb_s: f64,
    pub duration_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRequest {
    pub force: Option<bool>,
    pub check_only: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateResponse {
    pub success: bool,
    pub version: String,
    pub signatures_added: u32,
    pub signatures_removed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub scanner_status: String,
    pub database_version: String,
    pub signature_count: usize,
    pub memory_usage_mb: f64,
    pub last_scan: Option<String>,
    pub last_update: Option<String>,
    pub active_scans: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatInfo {
    pub id: String,
    pub file_path: String,
    pub threat_type: String,
    pub risk_level: String,
    pub signature_id: String,
}

pub struct ApiServer {
    addr: SocketAddr,
    api_key: String,
}

impl ApiServer {
    pub fn new(addr: SocketAddr, api_key: String) -> Self {
        Self { addr, api_key }
    }

    pub async fn start<T>(&self, state: Arc<T>) -> Result<(), anyhow::Error>
    where
        T: Clone + Send + Sync + 'static,
    {
        let api_key = self.api_key.clone();
        let state = Arc::clone(&state);

        let log = warp::log("virus_scanner::api");

        let routes = Self::routes(state, api_key)
            .or(Self::health_routes())
            .with(log);

        log::info!("API服务器启动，监听: {}", self.addr);
        warp::serve(routes).run(self.addr).await;

        Ok(())
    }

    fn routes<T>(
        state: Arc<T>,
        api_key: String,
    ) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
    where
        T: Clone + Send + Sync + 'static,
    {
        let state_filter = warp::any().map(move || state.clone());
        let auth_filter = warp::header::optional("X-API-Key")
            .and(warp::any().map(move || api_key.clone()))
            .and_then(|key: Option<String>, expected_key: String| async move {
                if key.as_ref() == Some(&expected_key) {
                    Ok::<_, Rejection>(())
                } else {
                    Err(warp::reject::custom(ApiError::Unauthorized))
                }
            });

        let scan_routes = warp::path!("api" / "v1" / "scan")
            .and(warp::post())
            .and(warp::body::json())
            .and(state_filter.clone())
            .and(auth_filter.clone())
            .and_then(Self::handle_scan);

        let update_routes = warp::path!("api" / "v1" / "update")
            .and(warp::post())
            .and(warp::body::json())
            .and(state_filter.clone())
            .and(auth_filter.clone())
            .and_then(Self::handle_update);

        let status_routes = warp::path!("api" / "v1" / "status")
            .and(warp::get())
            .and(state_filter.clone())
            .and(auth_filter.clone())
            .and_then(Self::handle_status);

        let threats_routes = warp::path!("api" / "v1" / "threats")
            .and(warp::get())
            .and(state_filter.clone())
            .and(auth_filter.clone())
            .and_then(Self::handle_threats);

        scan_routes
            .or(update_routes)
            .or(status_routes)
            .or(threats_routes)
    }

    fn health_routes() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        warp::path!("health")
            .and(warp::get())
            .map(|| {
                let response = ApiResponse::<()> {
                    success: true,
                    data: None,
                    error: None,
                    timestamp: chrono::Utc::now(),
                };
                warp::reply::json(&response)
            })
    }

    async fn handle_scan<T>(
        request: ScanRequest,
        _state: Arc<T>,
        _auth: (),
    ) -> Result<impl Reply, Rejection> {
        let scan_id = format!("SCN{:08}", rand::thread_rng().gen::<u32>());
        Ok(warp::reply::json(&ApiResponse {
            success: true,
            data: Some(ScanResponse {
                scan_id,
                status: "started".to_string(),
                threats_found: 0,
                files_scanned: 0,
                scan_speed_mb_s: 0.0,
                duration_seconds: 0.0,
            }),
            error: None,
            timestamp: chrono::Utc::now(),
        }))
    }

    async fn handle_update<T>(
        request: UpdateRequest,
        _state: Arc<T>,
        _auth: (),
    ) -> Result<impl Reply, Rejection> {
        Ok(warp::reply::json(&ApiResponse {
            success: true,
            data: Some(UpdateResponse {
                success: true,
                version: "1.0.0".to_string(),
                signatures_added: 0,
                signatures_removed: 0,
            }),
            error: None,
            timestamp: chrono::Utc::now(),
        }))
    }

    async fn handle_status<T>(
        _state: Arc<T>,
        _auth: (),
    ) -> Result<impl Reply, Rejection> {
        Ok(warp::reply::json(&ApiResponse {
            success: true,
            data: Some(StatusResponse {
                scanner_status: "running".to_string(),
                database_version: "1.0.0".to_string(),
                signature_count: 0,
                memory_usage_mb: 0.0,
                last_scan: None,
                last_update: None,
                active_scans: 0,
            }),
            error: None,
            timestamp: chrono::Utc::now(),
        }))
    }

    async fn handle_threats<T>(
        _state: Arc<T>,
        _auth: (),
    ) -> Result<impl Reply, Rejection> {
        Ok(warp::reply::json(&ApiResponse {
            success: true,
            data: Some(vec![] as Vec<ThreatInfo>),
            error: None,
            timestamp: chrono::Utc::now(),
        }))
    }
}

#[derive(Debug)]
pub enum ApiError {
    Unauthorized,
    NotFound,
    InternalError(String),
    ValidationError(String),
    None,
}

impl warp::reject::Reject for ApiError {}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Unauthorized => write!(f, "未授权访问"),
            ApiError::NotFound => write!(f, "资源不存在"),
            ApiError::InternalError(e) => write!(f, "内部错误: {}", e),
            ApiError::ValidationError(e) => write!(f, "验证错误: {}", e),
            ApiError::None => write!(f, "无错误"),
        }
    }
}
