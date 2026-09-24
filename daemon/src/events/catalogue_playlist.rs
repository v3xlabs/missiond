use serde::Serialize;

use super::CatalogueTab;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CataloguePlaylist {
    pub playlist_id: String,
    pub name: String,
    pub tabs: Vec<CatalogueTab>,
}
