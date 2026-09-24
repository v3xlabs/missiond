use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CatalogueTab {
    pub tab_id: String,
    pub name: String,
    pub enabled: bool,
}
