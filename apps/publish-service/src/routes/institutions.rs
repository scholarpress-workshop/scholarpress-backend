use crate::institutions::Registry;
use axum::{extract::State, Json};
use serde::Serialize;

#[derive(Serialize)]
pub struct InstitutionSummary {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ui_config: Option<serde_yaml::Value>,
}

pub async fn handler(State(registry): State<Registry>) -> Json<Vec<InstitutionSummary>> {
    let list: Vec<InstitutionSummary> = registry
        .list()
        .iter()
        .map(|inst| InstitutionSummary {
            id: inst.id.clone(),
            name: inst.name.clone(),
            ui_config: inst.ui_config.clone(),
        })
        .collect();
    Json(list)
}
