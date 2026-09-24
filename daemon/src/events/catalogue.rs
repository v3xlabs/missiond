use serde::Serialize;

use crate::config::Documents;

use super::{CataloguePlaylist, CatalogueTab};

/// The `catalogue` frame of `/api/events`: every playlist and the tabs in it, in play order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Catalogue {
    pub playlists: Vec<CataloguePlaylist>,
}

impl From<&Documents> for Catalogue {
    fn from(documents: &Documents) -> Self {
        let playlists = documents
            .playlists
            .playlists
            .iter()
            .map(|playlist| CataloguePlaylist {
                playlist_id: playlist.playlist_id.clone(),
                name: playlist.display_name().to_string(),
                tabs: playlist
                    .tabs
                    .iter()
                    .filter_map(|tab_id| {
                        let tab = documents
                            .tabs
                            .tabs
                            .iter()
                            .find(|tab| &tab.tab_id == tab_id)?;

                        Some(CatalogueTab {
                            tab_id: tab.tab_id.clone(),
                            name: tab.display_name().to_string(),
                            enabled: !playlist.disabled_tabs.contains(tab_id),
                        })
                    })
                    .collect(),
            })
            .collect();

        Self { playlists }
    }
}
