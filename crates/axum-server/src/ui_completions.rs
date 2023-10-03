use axum::{
    body::Body,
    extract::Path,
    http::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension, RequestExt,
};
use http::HeaderName;
use hyper::client;

use db::{queries, Pool};
use hyper_rustls::ConfigBuilderExt;

use crate::{api_reverse_proxy::Completion, authentication::Authentication};

pub async fn handler(
    Path(chat_id): Path<i32>,
    current_user: Authentication,
    Extension(pool): Extension<Pool>,
    req: Request<hyper::Body>,
) -> Result<Response, StatusCode> {
    let mut db_client = pool.get().await.map_err(|_| StatusCode::BAD_REQUEST)?;
    let transaction = db_client
        .transaction()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    super::rls::set_row_level_security_user(&transaction, &current_user)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let model = queries::models::model_host_by_chat_id()
        .bind(&transaction, &chat_id)
        .one()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let body: String = req.extract().await.map_err(|_| StatusCode::BAD_REQUEST)?;
    let completion: Completion = serde_json::from_str(&body).map_err(|e| {
        tracing::error!("{}", e);
        StatusCode::BAD_REQUEST
    })?;

    let max_tokens = if model.base_url.starts_with("https://inference.gig") {
        Some(4000)
    } else {
        None
    };

    let completion = Completion {
        model: model.name,
        stream: Some(true),
        max_tokens,
        temperature: Some(0.7),
        ..completion
    };

    let completion_json =
        serde_json::to_string(&completion).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Create a new request
    let mut req = Request::post(format!("{}/chat/completions", model.base_url))
        .header("content-type", "application/json")
        .body(Body::from(completion_json))
        .map_err(|e| {
            tracing::error!("{}", e);
            StatusCode::BAD_REQUEST
        })?;

    // Do we have an API key, then add it.
    if let Some(api_key) = model.api_key {
        req.headers_mut().append(
            HeaderName::from_static("authorization"),
            format!("Bearer {}", api_key).parse().unwrap(),
        );
    }

    dbg!(&req);

    let tls = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_native_roots()
        .with_no_client_auth();

    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_tls_config(tls)
        .https_or_http()
        .enable_http1()
        .build();

    // Build the hyper client from the HTTPS connector.
    let client: client::Client<_, hyper::Body> = client::Client::builder().build(https);

    Ok(client
        .request(req)
        .await
        .map_err(|e| {
            tracing::error!("{}", e);
            StatusCode::BAD_REQUEST
        })?
        .into_response())
}