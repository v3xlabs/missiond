use std::{sync::Arc, time::Duration};

use anyhow::Result;
use poem::{
    endpoint::EmbeddedFilesEndpoint,
    get, handler,
    http::StatusCode,
    listener::TcpListener,
    middleware::Cors,
    web::{
        sse::{Event, SSE},
        Data, Path,
    },
    Body, Endpoint, EndpointExt as _, IntoResponse, Middleware, Response, Route, Server,
};
use rust_embed::RustEmbed;
use tracing::info;

use crate::{
    api::{self, auth::Access},
    events::{Catalogue, StateFrame},
    state::AppState,
};

#[derive(RustEmbed)]
#[folder = "web-dist"]
struct WebAssets;

/// Asset names carry a content hash and change with every build, but the html naming them does
/// not. A surface profile outlives an upgrade, so a cached page comes back pointing at assets that
/// are no longer there and renders nothing at all.
struct NoStore;

impl<E: Endpoint> Middleware<E> for NoStore {
    type Output = NoStoreEndpoint<E>;

    fn transform(&self, inner: E) -> Self::Output {
        NoStoreEndpoint { inner }
    }
}

struct NoStoreEndpoint<E> {
    inner: E,
}

impl<E: Endpoint> Endpoint for NoStoreEndpoint<E> {
    type Output = Response;

    async fn call(&self, request: poem::Request) -> poem::Result<Self::Output> {
        let mut response = self.inner.call(request).await?.into_response();

        response.headers_mut().insert(
            "cache-control",
            "no-store".parse().expect("a literal header value"),
        );

        Ok(response)
    }
}

pub async fn start_http(state: Arc<AppState>) -> Result<()> {
    let http = state.config.read().await.device.http;
    let address = format!("{}:{}", http.host, http.port);

    info!(%address, "starting http server");

    let api_service = api::create_api_service(state.clone());
    let ui = api_service.swagger_ui();
    let spec = api_service.spec_endpoint();

    let app = Route::new()
        .at("/api/preview/:tab_id", get(preview).data(state.clone()))
        .at(
            "/api/preview_live/:tab_id",
            get(preview_live).data(state.clone()),
        )
        .at("/api/screen", get(screen).data(state.clone()))
        .at("/api/media/:name", get(media).data(state.clone()))
        .at("/api/events", get(events).data(state.clone()))
        .at(
            "/api/notifications/stream",
            get(notification_stream).data(state.clone()),
        )
        .nest("/api", api_service)
        .nest("/docs", ui)
        .at("/docs/spec", spec)
        .nest("/", EmbeddedFilesEndpoint::<WebAssets>::new().with(NoStore))
        .with(Cors::new());

    Server::new(TcpListener::bind(address)).run(app).await?;

    Ok(())
}

#[handler]
async fn preview(state: Data<&Arc<AppState>>, tab_id: Path<String>) -> impl IntoResponse {
    let Some(receiver) = state.chrome.watch_preview(&tab_id.0).await else {
        return not_found("no such tab");
    };

    let frame = receiver.borrow().clone();

    match frame {
        Some(frame) => Response::builder()
            .content_type("image/jpeg")
            .body(Body::from_bytes(frame.into())),
        None => not_found("no frame captured yet"),
    }
}

fn not_found(message: &str) -> Response {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(message.to_string())
}

/// Captured through the compositor's screencopy protocol, so unlike a tab preview this includes
/// anything drawn over the page.
#[handler]
async fn screen(state: Data<&Arc<AppState>>) -> impl IntoResponse {
    let display = state.config.read().await.display;

    match state.capture.grab(&display).await {
        Ok(image) => Response::builder()
            .content_type("image/jpeg")
            .header("cache-control", "no-store")
            .body(Body::from_bytes(image.into())),
        Err(error) => Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .body(error.to_string()),
    }
}

const HEARTBEAT: Duration = Duration::from_secs(2);

#[handler]
async fn preview_live(state: Data<&Arc<AppState>>, tab_id: Path<String>) -> impl IntoResponse {
    const BOUNDARY: &str = "missiondframe";

    let Some(mut receiver) = state.chrome.watch_preview(&tab_id.0).await else {
        return not_found("no such tab");
    };

    let stream = async_stream::stream! {
        yield Ok::<_, std::io::Error>(format!("--{BOUNDARY}\r\n").into_bytes());

        loop {
            let frame = receiver.borrow_and_update().clone();

            if let Some(frame) = frame {
                let part = format!(
                    "Content-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n",
                    frame.len()
                );

                yield Ok::<_, std::io::Error>(part.into_bytes());
                yield Ok(frame);
                yield Ok(format!("\r\n--{BOUNDARY}\r\n").into_bytes());
            }

            // Chromium commits a part only once another one follows it, so a page that has
            // stopped repainting needs its last frame sent again to appear at all.
            if let Ok(Err(_)) = tokio::time::timeout(HEARTBEAT, receiver.changed()).await {
                break;
            }
        }
    };

    Response::builder()
        .content_type(format!("multipart/x-mixed-replace; boundary={BOUNDARY}"))
        .body(Body::from_bytes_stream(stream))
}

/// Two kinds of frame. `state` is what is on screen, sent on connect and whenever it changes.
/// `catalogue` is every playlist and its tabs, sent on connect and whenever an edit changes it.
#[handler]
fn events(state: Data<&Arc<AppState>>, request: &poem::Request) -> SSE {
    let state = state.0.clone();
    let requires_auth = state.admin_key.is_some();
    let access = Access::of(&state, request.header("authorization"));
    let mut display = state.events.subscribe();
    let mut config = state.config.subscribe();

    display.mark_changed();
    config.mark_changed();

    SSE::new(async_stream::stream! {
        let mut sent_catalogue = None;

        loop {
            let frame = tokio::select! {
                biased;

                changed = display.changed() => {
                    if changed.is_err() {
                        break;
                    }

                    let display = display.borrow_and_update().clone();

                    serde_json::to_string(&StateFrame {
                        display: &display,
                        requires_auth,
                        access,
                    })
                    .ok()
                    .map(|payload| Event::message(payload).event_type("state"))
                }
                changed = config.changed() => {
                    if changed.is_err() {
                        break;
                    }

                    config.borrow_and_update();

                    let catalogue = Catalogue::from(&state.config.read().await);

                    if sent_catalogue.as_ref() == Some(&catalogue) {
                        None
                    } else {
                        let frame = serde_json::to_string(&catalogue)
                            .ok()
                            .map(|payload| Event::message(payload).event_type("catalogue"));

                        sent_catalogue = Some(catalogue);

                        frame
                    }
                }
            };

            if let Some(frame) = frame {
                yield frame;
            }
        }
    })
    .keep_alive(Duration::from_secs(30))
}

/// Not a keep alive: `Notifications::active` drops an expired entry and announces it only when
/// somebody asks, so this timeout is the only thing that notices the last entry of the day ending.
const SWEEP: Duration = Duration::from_secs(5);

#[handler]
fn notification_stream(state: Data<&Arc<AppState>>) -> SSE {
    let state = state.0.clone();
    let mut changed = state.notifications.subscribe();

    SSE::new(async_stream::stream! {
        loop {
            let active = state.notifications.active().await;

            if let Ok(payload) = serde_json::to_string(&active) {
                yield Event::message(payload);
            }

            if let Ok(Err(_)) = tokio::time::timeout(SWEEP, changed.changed()).await {
                break;
            }
        }
    })
    .keep_alive(Duration::from_secs(30))
}

#[handler]
async fn media(state: Data<&Arc<AppState>>, name: Path<String>) -> impl IntoResponse {
    if !is_plain_file_name(&name.0) {
        return not_found("no such media file");
    }

    let file = state.config.dirs.config.join("media").join(&name.0);

    match tokio::fs::read(&file).await {
        Ok(bytes) => Response::builder()
            .content_type(content_type(&file))
            .body(Body::from_bytes(bytes.into())),
        Err(_) => not_found("no such media file"),
    }
}

/// Checked rather than resolved: canonicalising would reject the legitimate case, because a
/// Nix-generated config directory reaches its files through symlinks into other store paths.
fn is_plain_file_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains('\0')
}

fn content_type(file: &std::path::Path) -> &'static str {
    match file.extension().and_then(|extension| extension.to_str()) {
        Some("webm") => "video/webm",
        Some("mp4") => "video/mp4",
        Some("gif") => "image/gif",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::is_plain_file_name;

    #[test]
    fn a_plain_name_is_allowed() {
        assert!(is_plain_file_name("doorbell.webm"));
    }

    #[test]
    fn traversal_is_refused() {
        assert!(!is_plain_file_name("../device.toml"));
        assert!(!is_plain_file_name("nested/clip.webm"));
        assert!(!is_plain_file_name("..\\device.toml"));
    }

    #[test]
    fn hidden_and_empty_names_are_refused() {
        assert!(!is_plain_file_name(".."));
        assert!(!is_plain_file_name(".hidden"));
        assert!(!is_plain_file_name(""));
    }
}
