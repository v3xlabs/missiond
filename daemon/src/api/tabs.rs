use std::sync::Arc;

use poem_openapi::{param::Path, payload::Json, OpenApi};

use crate::{
    chrome::{tell, ChromeMessage},
    config::{Document, Playlist},
    state::AppState,
};

use super::{
    auth::{authorize, authorize_control, Authorization},
    ApiError, ApiResult, MutationResult, TabInfo, UpsertTabRequest,
};

pub struct TabApi {
    pub state: Arc<AppState>,
}

#[OpenApi]
impl TabApi {
    /// Every configured tab, whether or not a playlist uses it.
    #[oai(path = "/tabs", method = "get")]
    async fn list(&self) -> ApiResult<Json<Vec<TabInfo>>> {
        let tabs = self.state.config.tabs().await;
        let mut out = Vec::with_capacity(tabs.len());

        for (index, tab) in tabs.iter().enumerate() {
            out.push(TabInfo::new(tab, index, true, &self.state.chrome).await);
        }

        Ok(Json(out))
    }

    /// Create a tab, or replace one with the same id.
    #[oai(path = "/tabs/:tab_id", method = "put")]
    async fn upsert(
        &self,
        tab_id: Path<String>,
        request: Json<UpsertTabRequest>,
        authorization: Authorization,
    ) -> ApiResult<Json<MutationResult>> {
        authorize(&self.state, &authorization)?;

        let url = request.0.url.clone();
        let tab = request.0.into_tab(tab_id.0.clone());

        let persisted = self
            .state
            .config
            .mutate(Document::Tabs, |documents| {
                match documents
                    .tabs
                    .tabs
                    .iter_mut()
                    .find(|existing| existing.tab_id == tab_id.0)
                {
                    Some(existing) => *existing = tab.clone(),
                    None => documents.tabs.tabs.push(tab.clone()),
                }
            })
            .await
            .map_err(ApiError::internal)?;

        let _ = self.state.chrome.update_url(&tab_id.0, &url).await;

        Ok(Json(persisted.into()))
    }

    /// Remove a tab, and every playlist reference to it.
    #[oai(path = "/tabs/:tab_id", method = "delete")]
    async fn delete(
        &self,
        tab_id: Path<String>,
        authorization: Authorization,
    ) -> ApiResult<Json<MutationResult>> {
        authorize(&self.state, &authorization)?;

        self.state
            .config
            .tab(&tab_id.0)
            .await
            .ok_or_else(|| ApiError::not_found(&tab_id.0))?;

        let persisted = self
            .state
            .config
            .mutate(Document::Tabs, |documents| {
                documents.tabs.tabs.retain(|tab| tab.tab_id != tab_id.0);
            })
            .await
            .map_err(ApiError::internal)?;

        self.state
            .config
            .mutate(Document::Playlists, |documents| {
                for playlist in &mut documents.playlists.playlists {
                    playlist.tabs.retain(|id| id != &tab_id.0);
                    playlist.disabled_tabs.retain(|id| id != &tab_id.0);
                }
            })
            .await
            .map_err(ApiError::internal)?;

        let _ = tell(
            &self.state.chrome,
            ChromeMessage::CloseTab {
                tab_id: tab_id.0.clone(),
            },
        )
        .await;

        Ok(Json(persisted.into()))
    }

    /// Put a tab on screen without naming a playlist, and hold it there. The tab plays within the
    /// current playlist when that has it, otherwise within the first playlist that does.
    #[oai(path = "/tabs/:tab_id/activate", method = "post")]
    async fn activate(
        &self,
        tab_id: Path<String>,
        authorization: Authorization,
    ) -> ApiResult<Json<MutationResult>> {
        authorize_control(&self.state, &authorization)?;

        let tab_id = tab_id.0;

        self.state
            .config
            .tab(&tab_id)
            .await
            .ok_or_else(|| ApiError::not_found(&tab_id))?;

        let current = self
            .state
            .chrome
            .state
            .lock()
            .await
            .current_playlist_id
            .clone();
        let playlists = self.state.config.playlists().await;
        let has_tab = |playlist: &&Playlist| playlist.tabs.contains(&tab_id);

        let playlist_id = current
            .as_deref()
            .and_then(|current| {
                playlists
                    .iter()
                    .find(|playlist| playlist.playlist_id == current)
            })
            .filter(has_tab)
            .or_else(|| playlists.iter().find(has_tab))
            .map(|playlist| playlist.playlist_id.clone())
            .or(current)
            .ok_or_else(|| {
                ApiError::bad_request(format!(
                    "tab {tab_id} is in no playlist, and no playlist is playing"
                ))
            })?;

        tell(
            &self.state.chrome,
            ChromeMessage::ActivateTab {
                tab_id,
                playlist_id,
            },
        )
        .await
        .map_err(ApiError::internal)?;

        Ok(Json(MutationResult::applied()))
    }

    /// Reload a tab's page.
    #[oai(path = "/tabs/:tab_id/refresh", method = "post")]
    async fn refresh(
        &self,
        tab_id: Path<String>,
        authorization: Authorization,
    ) -> ApiResult<Json<MutationResult>> {
        authorize_control(&self.state, &authorization)?;
        tell(
            &self.state.chrome,
            ChromeMessage::RefreshTab { tab_id: tab_id.0 },
        )
        .await
        .map_err(ApiError::internal)?;

        Ok(Json(MutationResult::applied()))
    }

    /// Close and reopen a tab's page.
    #[oai(path = "/tabs/:tab_id/recreate", method = "post")]
    async fn recreate(
        &self,
        tab_id: Path<String>,
        authorization: Authorization,
    ) -> ApiResult<Json<MutationResult>> {
        authorize_control(&self.state, &authorization)?;
        tell(
            &self.state.chrome,
            ChromeMessage::RecreateTab { tab_id: tab_id.0 },
        )
        .await
        .map_err(ApiError::internal)?;

        Ok(Json(MutationResult::applied()))
    }
}
