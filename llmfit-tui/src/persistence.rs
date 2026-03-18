use crate::tui_app::FitFilter;
use llmfit_core::{PowerContext, SortColumn, SystemSpecs};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const APP_STATE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedSelection {
    pub symbol: Option<String>,
    pub method_id: Option<String>,
    pub rig_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedAppState {
    pub schema_version: u32,
    pub saved_at_epoch: u64,
    pub detected_specs: SystemSpecs,
    pub power: PowerContext,
    pub sort_column: SortColumn,
    pub sort_ascending: bool,
    pub fit_filter: FitFilter,
    pub search_query: String,
    pub selected_algorithms: Vec<String>,
    pub selected_method_ids: Vec<String>,
    pub show_detail: bool,
    pub selection: PersistedSelection,
}

impl PersistedAppState {
    pub fn new(
        detected_specs: SystemSpecs,
        power: PowerContext,
        sort_column: SortColumn,
        sort_ascending: bool,
        fit_filter: FitFilter,
        search_query: String,
        selected_algorithms: Vec<String>,
        selected_method_ids: Vec<String>,
        show_detail: bool,
        selection: PersistedSelection,
    ) -> Self {
        Self {
            schema_version: APP_STATE_SCHEMA_VERSION,
            saved_at_epoch: current_epoch_seconds(),
            detected_specs,
            power,
            sort_column,
            sort_ascending,
            fit_filter,
            search_query,
            selected_algorithms,
            selected_method_ids,
            show_detail,
            selection,
        }
    }

    pub fn hardware_changed(&self, current_specs: &SystemSpecs) -> bool {
        self.detected_specs.cpu_name != current_specs.cpu_name
            || self.detected_specs.gpu_name != current_specs.gpu_name
            || self.detected_specs.gpu_count != current_specs.gpu_count
    }
}

pub fn load_persisted_state() -> Option<PersistedAppState> {
    let raw = fs::read_to_string(state_path()?).ok()?;
    let state = serde_json::from_str::<PersistedAppState>(&raw).ok()?;
    if state.schema_version != APP_STATE_SCHEMA_VERSION {
        return None;
    }
    Some(state)
}

pub fn save_persisted_state(state: &PersistedAppState) {
    let Some(path) = state_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(payload) = serde_json::to_vec_pretty(state) {
        let _ = fs::write(path, payload);
    }
}

fn state_path() -> Option<PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()?;
    Some(
        PathBuf::from(home)
            .join(".config")
            .join("minefit")
            .join("state.json"),
    )
}

fn current_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use llmfit_core::{GpuBackend, fallback_power_context};

    #[test]
    fn persisted_state_round_trips_json() {
        let state = PersistedAppState::new(
            SystemSpecs {
                total_ram_gb: 32.0,
                available_ram_gb: 20.0,
                total_cpu_cores: 16,
                cpu_name: "Test CPU".to_string(),
                has_gpu: true,
                gpu_vram_gb: Some(8.0),
                total_gpu_vram_gb: Some(8.0),
                gpu_name: Some("Test GPU".to_string()),
                gpu_count: 1,
                unified_memory: false,
                backend: GpuBackend::Cuda,
                gpus: Vec::new(),
            },
            fallback_power_context("test"),
            SortColumn::NetUsd,
            false,
            FitFilter::Viable,
            "btc".to_string(),
            vec!["SHA256".to_string()],
            vec!["sha256-pool-pps".to_string()],
            true,
            PersistedSelection {
                symbol: Some("BTC".to_string()),
                method_id: Some("sha256-pool-pps".to_string()),
                rig_name: Some("Test GPU".to_string()),
            },
        );

        let raw = serde_json::to_string(&state).expect("state should serialize");
        let restored: PersistedAppState =
            serde_json::from_str(&raw).expect("state should deserialize");

        assert_eq!(restored.detected_specs.cpu_name, "Test CPU");
        assert_eq!(restored.sort_column, SortColumn::NetUsd);
        assert_eq!(restored.fit_filter, FitFilter::Viable);
        assert_eq!(restored.search_query, "btc");
        assert_eq!(
            restored.selected_method_ids,
            vec!["sha256-pool-pps".to_string()]
        );
        assert_eq!(restored.selection.symbol.as_deref(), Some("BTC"));
    }
}
