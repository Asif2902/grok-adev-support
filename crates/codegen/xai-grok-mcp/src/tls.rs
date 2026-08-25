//! Android-safe TLS for the quarantined `reqwest` 0.13 clients.
//!
//! reqwest 0.13's rustls backend defaults to `rustls-platform-verifier`,
//! which resolves the system trust store through the platform verifier. On
//! `target_os = "android"` that verifier needs a JVM plus an
//! `android.content.Context` handed over JNI
//! (`rustls_platform_verifier::android::init_hosted`) before the first
//! client is built. This CLI runs as a plain shell process (Termux) with no
//! JVM attached, so every default-built reqwest 0.13 client fails at
//! `.build()` time with a platform-verifier error — breaking MCP HTTP
//! transports and the MCP OAuth flow (device-code polling and normal API
//! traffic are unaffected: they run on workspace reqwest 0.12, whose
//! `rustls-tls` feature ships webpki roots and never touches JNI).
//!
//! On Android, [`client_builder`] therefore pins every reqwest 0.13 client
//! to an explicit rustls [`ClientConfig`] rooted at Mozilla's webpki roots
//! (the same root set workspace reqwest 0.12 uses), bypassing
//! rustls-platform-verifier entirely. Other platforms keep reqwest's
//! defaults so user- and enterprise-installed trust anchors keep working.
//!
//! [`authorization_manager`] builds rmcp's `AuthorizationManager` over the
//! same hardened clients (mirroring rmcp's own reqwest OAuth HTTP adapter,
//! including its redirect-policy split and 1 MiB response cap), because
//! `AuthorizationManager::new` internally builds a default reqwest client
//! that would fail on Android for the same reason.

/// Timeout applied to every outbound OAuth HTTP request. Mirrors rmcp's
/// private `DEFAULT_HTTP_TIMEOUT`.
const OAUTH_HTTP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Maximum buffered OAuth response body. Mirrors rmcp's private
/// `MAX_OAUTH_HTTP_RESPONSE_BODY_BYTES`.
const MAX_OAUTH_HTTP_RESPONSE_BODY_BYTES: usize = 1024 * 1024;

/// Apply the target-appropriate TLS configuration to a reqwest 0.13 builder.
///
/// On Android this swaps in a preconfigured rustls backend rooted at webpki
/// roots; elsewhere it is a passthrough.
pub(crate) fn client_builder(base: reqwest::ClientBuilder) -> reqwest::ClientBuilder {
    #[cfg(target_os = "android")]
    {
        base.tls_backend_preconfigured(webpki_roots_client_config())
    }
    #[cfg(not(target_os = "android"))]
    {
        base
    }
}

/// Build a hardened `reqwest::Client` with the given extra configuration.
pub(crate) fn build_client(
    configure: impl FnOnce(reqwest::ClientBuilder) -> reqwest::ClientBuilder,
) -> Result<reqwest::Client, reqwest::Error> {
    configure(client_builder(reqwest::Client::builder())).build()
}

#[cfg(target_os = "android")]
fn webpki_roots_client_config() -> rustls::ClientConfig {
    let provider = rustls::crypto::CryptoProvider::get_default().map(std::sync::Arc::clone);
    let provider =
        provider.unwrap_or_else(|| std::sync::Arc::new(rustls::crypto::aws_lc_rs::default_provider()));
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    rustls::ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13, &rustls::version::TLS12])
        .expect("provider supports TLS 1.2/1.3")
        .with_root_certificates(roots)
        .with_no_client_auth()
}

/// rmcp's `AuthorizationManager::new` internally builds a default-configured
/// reqwest 0.13 client, which fails on Android (see module docs). This
/// mirrors rmcp's own reqwest-based [`OAuthHttpClient`] adapter — two clients
/// sharing one TLS policy, split only by redirect behavior — but routes both
/// through [`client_builder`], then hands them to rmcp via
/// `new_with_oauth_http_client` so the OAuth state machine keeps its exact
/// Follow/Stop redirect semantics.
pub(crate) async fn authorization_manager(
    base_url: &str,
) -> Result<rmcp::transport::auth::AuthorizationManager, rmcp::transport::auth::AuthError> {
    let follow_redirects = build_client(|b| b.timeout(OAUTH_HTTP_TIMEOUT))
        .map_err(|e| rmcp::transport::auth::AuthError::InternalError(e.to_string()))?;
    let stop_redirects = build_client(|b| {
        b.timeout(OAUTH_HTTP_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
    })
    .map_err(|e| rmcp::transport::auth::AuthError::InternalError(e.to_string()))?;

    rmcp::transport::auth::AuthorizationManager::new_with_oauth_http_client(
        base_url,
        std::sync::Arc::new(HardenedOAuthHttpClient {
            follow_redirects,
            stop_redirects,
        }),
    )
    .await
}

/// Hardened replacement for rmcp's private `ReqwestOAuthHttpClient`: same
/// request execution, body buffering, and 1 MiB response cap, over clients
/// built by [`client_builder`].
struct HardenedOAuthHttpClient {
    follow_redirects: reqwest::Client,
    stop_redirects: reqwest::Client,
}

impl rmcp::transport::auth::OAuthHttpClient for HardenedOAuthHttpClient {
    fn execute(
        &self,
        request: rmcp::transport::auth::OAuthHttpRequest,
    ) -> rmcp::transport::auth::OAuthHttpClientFuture<'_> {
        Box::pin(async move {
            let rmcp::transport::auth::OAuthHttpRequest {
                request,
                redirect_policy,
                ..
            } = request;
            let client = match redirect_policy {
                rmcp::transport::auth::OAuthHttpRedirectPolicy::Follow => &self.follow_redirects,
                // `#[non_exhaustive]` upstream; Stop is the conservative default
                // (mirrors rmcp's own no-redirect token handling).
                _ => &self.stop_redirects,
            };
            let request = reqwest::Request::try_from(request)
                .map_err(|e| rmcp::transport::auth::OAuthHttpClientError::new(e.to_string()))?;
            let response = client.execute(request).await.map_err(|e| {
                rmcp::transport::auth::OAuthHttpClientError::new(e.to_string())
            })?;

            let mut builder = http::Response::builder()
                .status(response.status())
                .version(response.version());
            for (name, value) in response.headers() {
                builder = builder.header(name, value);
            }
            let mut body = Vec::new();
            let mut body_stream = response.bytes_stream();
            use futures::StreamExt as _;
            while let Some(chunk) = body_stream.next().await {
                let chunk = chunk.map_err(|e| {
                    rmcp::transport::auth::OAuthHttpClientError::new(e.to_string())
                })?;
                if chunk.len() > MAX_OAUTH_HTTP_RESPONSE_BODY_BYTES - body.len() {
                    return Err(rmcp::transport::auth::OAuthHttpClientError::new(format!(
                        "OAuth HTTP response body exceeds {MAX_OAUTH_HTTP_RESPONSE_BODY_BYTES} bytes"
                    )));
                }
                body.extend_from_slice(&chunk);
            }
            builder.body(body).map_err(|e| {
                rmcp::transport::auth::OAuthHttpClientError::new(e.to_string())
            })
        })
    }
}
