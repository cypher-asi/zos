use std::path::PathBuf;

use axum::http::HeaderValue;
use axum::middleware;
use axum::routing::{get, post};
use axum::Json;
use axum::Router;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;

use crate::handlers::{auth, projects};
use crate::state::AppState;

const LOCAL_CORS_HOSTS: &[&str] = &["localhost", "127.0.0.1"];

fn is_local_origin(origin: &HeaderValue) -> bool {
    let Ok(s) = origin.to_str() else {
        return false;
    };
    let Some((scheme, rest)) = s.split_once("://") else {
        return false;
    };
    let host = rest.split('/').next().unwrap_or(rest);
    matches!(scheme, "http" | "https")
        && LOCAL_CORS_HOSTS
            .iter()
            .any(|h| host == *h || host.starts_with(&format!("{h}:")))
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/logout", post(auth::logout))
        .route(
            "/api/auth/request-password-reset",
            post(auth::request_password_reset),
        )
}

fn protected_auth_routes() -> Router<AppState> {
    Router::new()
        .route("/api/auth/session", get(auth::get_session))
        .route("/api/auth/validate", post(auth::validate))
}

fn project_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/projects",
            get(projects::list_projects).post(projects::create_project),
        )
        .route(
            "/api/projects/{id}",
            get(projects::get_project)
                .put(projects::update_project)
                .delete(projects::delete_project),
        )
}

pub fn create_router(state: AppState, interface_dir: Option<PathBuf>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin, _| is_local_origin(origin)))
        .allow_credentials(true)
        .allow_methods(AllowMethods::mirror_request())
        .allow_headers(AllowHeaders::mirror_request());

    let protected = Router::new()
        .merge(protected_auth_routes())
        .merge(project_routes())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::auth_guard::require_verified_session,
        ));

    let api = Router::new()
        .route("/api/health", get(health))
        .merge(auth_routes())
        .merge(protected)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    match interface_dir {
        Some(dir) => {
            let index = dir.join("index.html");
            let serve = tower::ServiceBuilder::new()
                .layer(SetResponseHeaderLayer::overriding(
                    axum::http::header::CACHE_CONTROL,
                    HeaderValue::from_static("no-cache"),
                ))
                .service(ServeDir::new(&dir).not_found_service(ServeFile::new(index)));
            api.fallback_service(serve)
        }
        None => api,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_local_origin_localhost() {
        let hv = HeaderValue::from_static("http://localhost:3000");
        assert!(is_local_origin(&hv));
    }

    #[test]
    fn is_local_origin_127() {
        let hv = HeaderValue::from_static("http://127.0.0.1:5173");
        assert!(is_local_origin(&hv));
    }

    #[test]
    fn is_local_origin_https() {
        let hv = HeaderValue::from_static("https://localhost:8443");
        assert!(is_local_origin(&hv));
    }

    #[test]
    fn is_local_origin_rejects_remote() {
        let hv = HeaderValue::from_static("https://example.com");
        assert!(!is_local_origin(&hv));
    }

    #[test]
    fn is_local_origin_rejects_ftp() {
        let hv = HeaderValue::from_static("ftp://localhost");
        assert!(!is_local_origin(&hv));
    }

    #[test]
    fn is_local_origin_rejects_invalid() {
        let hv = HeaderValue::from_static("not-a-url");
        assert!(!is_local_origin(&hv));
    }

    #[test]
    fn is_local_origin_bare_localhost_no_port() {
        let hv = HeaderValue::from_static("http://localhost");
        assert!(is_local_origin(&hv));
    }

    #[test]
    fn is_local_origin_bare_127_no_port() {
        let hv = HeaderValue::from_static("http://127.0.0.1");
        assert!(is_local_origin(&hv));
    }
}
