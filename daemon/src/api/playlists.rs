use std::sync::Arc;

use poem_openapi::{param::Path, payload::Json, OpenApi};

use crate::{
    chrome::{tell, ChromeMessage},
    config::Document,
    state::AppState,
};

use super::{
    auth::{authorize, authorize_control, Authorization},
    ApiError, ApiResult, CreatePlaylistRequest, MutationResult, PlaylistInfo, ReorderRequest,
    SetEnabledRequest, TabInfo,
};

pub struct PlaylistApi {
    pub state: Arc<AppState>,
}

#[OpenApi]
impl PlaylistApi {
    /// Every playlist, in configuration order.
    #[oai(path = "/playlists", method = "get")]
    async fn list(&self) -> ApiResult<Json<Vec<PlaylistInfo>>> {
        let active = self
            .state
            .chrome
            .state
            .lock()
            .await
            .current_playlist_id
            .clone();

        let playlists = self.state.config.playlists().await;
        let mut out = Vec::with_capacity(playlists.len());

        for playlist in &playlists {
            let tab_count = self
                .state
                .config
                .playlist_tabs(&playlist.playlist_id)
                .await
                .len();

            out.push(PlaylistInfo::new(playlist, tab_count, active.as_deref()));
        }

        Ok(Json(out))
    }

    /// A playlist's tabs, in play order.
    #[oai(path = "/playlists/:playlist_id/tabs", method = "get")]
    async fn tabs(&self, playlist_id: Path<String>) -> ApiResult<Json<Vec<TabInfo>>> {
        let playlist = self
            .state
            .config
            .playlist(&playlist_id.0)
            .await
            .ok_or_else(|| ApiError::not_found(&playlist_id.0))?;

        let mut out = Vec::with_capacity(playlist.tabs.len());

        for (index, tab_id) in playlist.tabs.iter().enumerate() {
            let Some(tab) = self.state.config.tab(tab_id).await else {
                continue;
            };

            let enabled = !playlist.disabled_tabs.contains(tab_id);

            out.push(TabInfo::new(&tab, index, enabled, &self.state.chrome).await);
        }

        Ok(Json(out))
    }

    /// Create a playlist.
    #[oai(path = "/playlists", method = "post")]
    async fn create(
        &self,
        request: Json<CreatePlaylistRequest>,
        authorization: Authorization,
    ) -> ApiResult<Json<MutationResult>> {
        authorize(&self.state, &authorization)?;

        let playlist = request.0.into_playlist()?;

        if self
            .state
            .config
            .playlist(&playlist.playlist_id)
            .await
            .is_some()
        {
            return Err(ApiError::bad_request(format!(
                "playlist {} already exists",
                playlist.playlist_id
            )));
        }

        let persisted = self
            .state
            .config
            .mutate(Document::Playlists, |documents| {
                documents.playlists.playlists.push(playlist.clone());
            })
            .await
            .map_err(ApiError::internal)?;

        Ok(Json(persisted.into()))
    }

    /// Delete a playlist.
    #[oai(path = "/playlists/:playlist_id", method = "delete")]
    async fn delete(
        &self,
        playlist_id: Path<String>,
        authorization: Authorization,
    ) -> ApiResult<Json<MutationResult>> {
        authorize(&self.state, &authorization)?;
        self.require(&playlist_id.0).await?;

        let persisted = self
            .state
            .config
            .mutate(Document::Playlists, |documents| {
                documents
                    .playlists
                    .playlists
                    .retain(|playlist| playlist.playlist_id != playlist_id.0);
            })
            .await
            .map_err(ApiError::internal)?;

        Ok(Json(persisted.into()))
    }

    /// Put a playlist on screen.
    #[oai(path = "/playlists/:playlist_id/activate", method = "post")]
    async fn activate(
        &self,
        playlist_id: Path<String>,
        authorization: Authorization,
    ) -> ApiResult<Json<MutationResult>> {
        authorize_control(&self.state, &authorization)?;

        tell(
            &self.state.chrome,
            ChromeMessage::ActivatePlaylist {
                playlist_id: playlist_id.0,
            },
        )
        .await
        .map_err(ApiError::internal)?;

        Ok(Json(MutationResult::applied()))
    }

    /// Put a tab on screen, and hold it there for the playlist's hold duration.
    #[oai(
        path = "/playlists/:playlist_id/tabs/:tab_id/activate",
        method = "post"
    )]
    async fn activate_tab(
        &self,
        playlist_id: Path<String>,
        tab_id: Path<String>,
        authorization: Authorization,
    ) -> ApiResult<Json<MutationResult>> {
        authorize_control(&self.state, &authorization)?;

        tell(
            &self.state.chrome,
            ChromeMessage::ActivateTab {
                tab_id: tab_id.0,
                playlist_id: playlist_id.0,
            },
        )
        .await
        .map_err(ApiError::internal)?;

        Ok(Json(MutationResult::applied()))
    }

    /// Reorder a playlist. The list order is the play order, so this rewrites `playlists.toml`.
    #[oai(path = "/playlists/:playlist_id/reorder", method = "put")]
    async fn reorder(
        &self,
        playlist_id: Path<String>,
        request: Json<ReorderRequest>,
        authorization: Authorization,
    ) -> ApiResult<Json<MutationResult>> {
        authorize(&self.state, &authorization)?;

        let playlist = self.require(&playlist_id.0).await?;

        let mut requested = request.0.tab_ids.clone();
        let mut existing = playlist.tabs.clone();

        requested.sort();
        existing.sort();

        if requested != existing {
            return Err(ApiError::bad_request(
                "the reordered list must contain exactly the playlist's current tabs",
            ));
        }

        let persisted = self
            .state
            .config
            .mutate(Document::Playlists, |documents| {
                if let Some(playlist) = find(documents, &playlist_id.0) {
                    playlist.tabs = request.0.tab_ids.clone();
                }
            })
            .await
            .map_err(ApiError::internal)?;

        Ok(Json(persisted.into()))
    }

    /// Include or exclude a tab from a playlist without removing it.
    #[oai(path = "/playlists/:playlist_id/tabs/:tab_id/enabled", method = "put")]
    async fn set_tab_enabled(
        &self,
        playlist_id: Path<String>,
        tab_id: Path<String>,
        request: Json<SetEnabledRequest>,
        authorization: Authorization,
    ) -> ApiResult<Json<MutationResult>> {
        authorize(&self.state, &authorization)?;
        self.require(&playlist_id.0).await?;

        let persisted = self
            .state
            .config
            .mutate(Document::Playlists, |documents| {
                if let Some(playlist) = find(documents, &playlist_id.0) {
                    playlist.disabled_tabs.retain(|id| id != &tab_id.0);

                    if !request.0.enabled {
                        playlist.disabled_tabs.push(tab_id.0.clone());
                    }
                }
            })
            .await
            .map_err(ApiError::internal)?;

        Ok(Json(persisted.into()))
    }

    /// Add an existing tab to the end of a playlist.
    #[oai(path = "/playlists/:playlist_id/tabs/:tab_id", method = "put")]
    async fn add_tab(
        &self,
        playlist_id: Path<String>,
        tab_id: Path<String>,
        authorization: Authorization,
    ) -> ApiResult<Json<MutationResult>> {
        authorize(&self.state, &authorization)?;
        self.require(&playlist_id.0).await?;

        self.state
            .config
            .tab(&tab_id.0)
            .await
            .ok_or_else(|| ApiError::not_found(&tab_id.0))?;

        let persisted = self
            .state
            .config
            .mutate(Document::Playlists, |documents| {
                if let Some(playlist) = find(documents, &playlist_id.0) {
                    if !playlist.tabs.contains(&tab_id.0) {
                        playlist.tabs.push(tab_id.0.clone());
                    }
                }
            })
            .await
            .map_err(ApiError::internal)?;

        Ok(Json(persisted.into()))
    }

    /// Remove a tab from one playlist, leaving the tab itself alone.
    #[oai(path = "/playlists/:playlist_id/tabs/:tab_id", method = "delete")]
    async fn remove_tab(
        &self,
        playlist_id: Path<String>,
        tab_id: Path<String>,
        authorization: Authorization,
    ) -> ApiResult<Json<MutationResult>> {
        authorize(&self.state, &authorization)?;
        self.require(&playlist_id.0).await?;

        let persisted = self
            .state
            .config
            .mutate(Document::Playlists, |documents| {
                if let Some(playlist) = find(documents, &playlist_id.0) {
                    playlist.tabs.retain(|id| id != &tab_id.0);
                    playlist.disabled_tabs.retain(|id| id != &tab_id.0);
                }
            })
            .await
            .map_err(ApiError::internal)?;

        Ok(Json(persisted.into()))
    }
}

impl PlaylistApi {
    async fn require(&self, playlist_id: &str) -> ApiResult<crate::config::Playlist> {
        self.state
            .config
            .playlist(playlist_id)
            .await
            .ok_or_else(|| ApiError::not_found(playlist_id))
    }
}

fn find<'a>(
    documents: &'a mut crate::config::Documents,
    playlist_id: &str,
) -> Option<&'a mut crate::config::Playlist> {
    documents
        .playlists
        .playlists
        .iter_mut()
        .find(|playlist| playlist.playlist_id == playlist_id)
}
