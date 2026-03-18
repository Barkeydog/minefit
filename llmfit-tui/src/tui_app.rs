use crate::persistence::{PersistedAppState, PersistedSelection, save_persisted_state};
use crate::theme::Theme;
use llmfit_core::{
    MiningRigProfile, PowerContext, SnapshotCacheStatus, build_rankings_for_rigs,
    describe_rig_scope,
    expand_power_context_options,
};
use llmfit_core::hardware::SystemSpecs;
use llmfit_core::mining::{
    FitLevel, METHODS, MiningRow, MiningSnapshot, SortColumn, sort_rankings,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
    AlgorithmPopup,
    MethodPopup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FitFilter {
    All,
    Positive,
    Prime,
    Viable,
    Risky,
    Avoid,
}

impl FitFilter {
    pub fn label(&self) -> &'static str {
        match self {
            FitFilter::All => "All",
            FitFilter::Positive => "Positive",
            FitFilter::Prime => "Prime",
            FitFilter::Viable => "Viable",
            FitFilter::Risky => "Risky",
            FitFilter::Avoid => "Avoid",
        }
    }

    pub fn next(self) -> Self {
        match self {
            FitFilter::All => FitFilter::Positive,
            FitFilter::Positive => FitFilter::Prime,
            FitFilter::Prime => FitFilter::Viable,
            FitFilter::Viable => FitFilter::Risky,
            FitFilter::Risky => FitFilter::Avoid,
            FitFilter::Avoid => FitFilter::All,
        }
    }
}

pub struct App {
    pub should_quit: bool,
    pub input_mode: InputMode,
    pub search_query: String,
    pub cursor_position: usize,
    pub specs: SystemSpecs,
    pub snapshot: MiningSnapshot,
    pub all_rows: Vec<MiningRow>,
    pub filtered_rows: Vec<usize>,
    pub algorithms: Vec<String>,
    pub selected_algorithms: Vec<bool>,
    pub methods: Vec<String>,
    pub selected_methods: Vec<bool>,
    pub algorithm_cursor: usize,
    pub method_cursor: usize,
    pub fit_filter: FitFilter,
    pub sort_column: SortColumn,
    pub sort_ascending: bool,
    pub selected_row: usize,
    pub show_detail: bool,
    pub theme: Theme,
    pub power: PowerContext,
    pub power_options: Vec<PowerContext>,
    pub power_index: usize,
    pub rigs: Vec<MiningRigProfile>,
    pub rig_label: String,
    pub snapshot_status: SnapshotCacheStatus,
    pub status_message: String,
}

impl App {
    pub fn new(
        specs: SystemSpecs,
        snapshot: MiningSnapshot,
        snapshot_status: SnapshotCacheStatus,
        power: PowerContext,
        rigs: Vec<MiningRigProfile>,
        sort_column: SortColumn,
        persisted_state: Option<&PersistedAppState>,
    ) -> Self {
        let algorithms = snapshot.algorithms();
        let methods = METHODS.iter().map(|method| method.name.to_string()).collect();
        let power_options = build_power_options(&power);
        let power_index = power_options
            .iter()
            .position(|option| option.plan_id == power.plan_id)
            .unwrap_or(0);
        let rig_label = describe_rig_scope(&rigs);

        let mut app = Self {
            should_quit: false,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            cursor_position: 0,
            specs,
            snapshot,
            all_rows: Vec::new(),
            filtered_rows: Vec::new(),
            algorithms,
            selected_algorithms: Vec::new(),
            methods,
            selected_methods: vec![true; METHODS.len()],
            algorithm_cursor: 0,
            method_cursor: 0,
            fit_filter: FitFilter::All,
            sort_column,
            sort_ascending: false,
            selected_row: 0,
            show_detail: false,
            theme: Theme::load(),
            power,
            power_options,
            power_index,
            rigs,
            rig_label,
            snapshot_status,
            status_message: String::new(),
        };

        app.selected_algorithms = vec![true; app.algorithms.len()];
        let persisted_selection = persisted_state.and_then(|state| app.apply_persisted_state(state));
        app.status_message = app.startup_status_message(persisted_state);
        app.rebuild_rows(persisted_selection);
        app
    }

    pub fn power_badge(&self) -> String {
        self.power.badge()
    }

    pub fn snapshot_badge(&self) -> String {
        self.snapshot_status.badge()
    }

    pub fn rig_badge(&self) -> String {
        self.rig_label.clone()
    }

    pub fn selected_fit(&self) -> Option<&MiningRow> {
        let index = *self.filtered_rows.get(self.selected_row)?;
        self.all_rows.get(index)
    }

    pub fn visible_algorithm_count(&self) -> usize {
        self.selected_algorithms.iter().filter(|selected| **selected).count()
    }

    pub fn visible_method_count(&self) -> usize {
        self.selected_methods.iter().filter(|selected| **selected).count()
    }

    pub fn refresh_data(&mut self) {
        let selected_key = self.selected_key();
        let all_algorithms_selected = self.selected_algorithms.iter().all(|selected| *selected);
        let visible_algorithms = self
            .algorithms
            .iter()
            .zip(self.selected_algorithms.iter())
            .filter_map(|(algorithm, selected)| if *selected { Some(algorithm.clone()) } else { None })
            .collect::<Vec<_>>();
        match MiningSnapshot::refresh_with_cache() {
            Ok(load) => {
                self.snapshot = load.snapshot;
                self.snapshot_status = load.status;
                self.algorithms = self.snapshot.algorithms();
                self.selected_algorithms = if all_algorithms_selected {
                    vec![true; self.algorithms.len()]
                } else {
                    self.algorithms
                        .iter()
                        .map(|algorithm| visible_algorithms.iter().any(|saved| saved == algorithm))
                        .collect()
                };
                if !self.selected_algorithms.iter().any(|selected| *selected) {
                    self.selected_algorithms = vec![true; self.algorithms.len()];
                }
                self.status_message = format!(
                    "Feed refreshed: {} rankable | {} catalog | Snapshot: {} | Power: {}",
                    self.snapshot.rankable_coin_count(),
                    self.snapshot.catalog_asset_count(),
                    self.snapshot_badge(),
                    self.power_badge(),
                );
                self.rebuild_rows(selected_key);
            }
            Err(err) => {
                self.status_message = format!("Refresh failed: {err}");
            }
        }
    }

    pub fn move_up(&mut self) {
        if self.selected_row > 0 {
            self.selected_row -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected_row + 1 < self.filtered_rows.len() {
            self.selected_row += 1;
        }
    }

    pub fn page_up(&mut self) {
        self.selected_row = self.selected_row.saturating_sub(10);
    }

    pub fn page_down(&mut self) {
        self.selected_row = (self.selected_row + 10).min(self.filtered_rows.len().saturating_sub(1));
    }

    pub fn home(&mut self) {
        self.selected_row = 0;
    }

    pub fn end(&mut self) {
        if !self.filtered_rows.is_empty() {
            self.selected_row = self.filtered_rows.len() - 1;
        }
    }

    pub fn enter_search(&mut self) {
        self.input_mode = InputMode::Search;
        self.cursor_position = self.search_query.len();
    }

    pub fn exit_search(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.cursor_position = 0;
        self.apply_filters();
    }

    pub fn search_input(&mut self, value: char) {
        self.search_query.insert(self.cursor_position, value);
        self.cursor_position += value.len_utf8();
        self.apply_filters();
    }

    pub fn search_backspace(&mut self) {
        if self.cursor_position == 0 || self.search_query.is_empty() {
            return;
        }

        self.cursor_position -= 1;
        self.search_query.remove(self.cursor_position);
        self.apply_filters();
    }

    pub fn search_delete(&mut self) {
        if self.cursor_position >= self.search_query.len() {
            return;
        }

        self.search_query.remove(self.cursor_position);
        self.apply_filters();
    }

    pub fn cycle_fit_filter(&mut self) {
        self.fit_filter = self.fit_filter.next();
        self.apply_filters();
    }

    pub fn cycle_sort_column(&mut self) {
        self.sort_column = self.sort_column.next();
        self.rebuild_rows(self.selected_key());
    }

    pub fn cycle_theme(&mut self) {
        self.theme = self.theme.next();
        self.theme.save();
    }

    pub fn cycle_electricity(&mut self) {
        self.power_index = (self.power_index + 1) % self.power_options.len();
        self.power = self.power_options[self.power_index].clone();
        self.status_message = format!("Power: {}", self.power.summary_line());
        self.rebuild_rows(self.selected_key());
    }

    pub fn toggle_detail(&mut self) {
        self.show_detail = !self.show_detail;
    }

    pub fn open_algorithm_popup(&mut self) {
        self.input_mode = InputMode::AlgorithmPopup;
    }

    pub fn close_algorithm_popup(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    pub fn algorithm_popup_up(&mut self) {
        if self.algorithm_cursor > 0 {
            self.algorithm_cursor -= 1;
        }
    }

    pub fn algorithm_popup_down(&mut self) {
        if self.algorithm_cursor + 1 < self.algorithms.len() {
            self.algorithm_cursor += 1;
        }
    }

    pub fn algorithm_popup_toggle(&mut self) {
        if self.algorithm_cursor < self.selected_algorithms.len() {
            self.selected_algorithms[self.algorithm_cursor] =
                !self.selected_algorithms[self.algorithm_cursor];
            self.apply_filters();
        }
    }

    pub fn algorithm_popup_select_all(&mut self) {
        let next = !self.selected_algorithms.iter().all(|selected| *selected);
        for selected in &mut self.selected_algorithms {
            *selected = next;
        }
        self.apply_filters();
    }

    pub fn open_method_popup(&mut self) {
        self.input_mode = InputMode::MethodPopup;
    }

    pub fn close_method_popup(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    pub fn method_popup_up(&mut self) {
        if self.method_cursor > 0 {
            self.method_cursor -= 1;
        }
    }

    pub fn method_popup_down(&mut self) {
        if self.method_cursor + 1 < self.methods.len() {
            self.method_cursor += 1;
        }
    }

    pub fn method_popup_toggle(&mut self) {
        if self.method_cursor < self.selected_methods.len() {
            self.selected_methods[self.method_cursor] = !self.selected_methods[self.method_cursor];
            self.apply_filters();
        }
    }

    pub fn method_popup_select_all(&mut self) {
        let next = !self.selected_methods.iter().all(|selected| *selected);
        for selected in &mut self.selected_methods {
            *selected = next;
        }
        self.apply_filters();
    }

    fn active_rigs(&self) -> &[MiningRigProfile] {
        &self.rigs
    }

    fn selected_key(&self) -> Option<(String, String, String)> {
        let row = self.selected_fit()?;
        Some((
            row.coin.symbol.clone(),
            row.method.id.to_string(),
            row.rig_name.clone(),
        ))
    }

    fn rebuild_rows(&mut self, selected_key: Option<(String, String, String)>) {
        self.all_rows = build_rankings_for_rigs(&self.snapshot, &self.power, self.active_rigs(), 1.0);
        sort_rankings(&mut self.all_rows, self.sort_column, self.sort_ascending);
        self.apply_filters();

        if let Some((symbol, method_id, rig_name)) = selected_key
            && let Some(idx) = self.filtered_rows.iter().position(|row_idx| {
                self.all_rows
                    .get(*row_idx)
                    .map(|row| {
                        row.coin.symbol == symbol
                            && row.method.id == method_id
                            && row.rig_name == rig_name
                    })
                    .unwrap_or(false)
            })
        {
            self.selected_row = idx;
        }
    }

    fn apply_filters(&mut self) {
        let query = self.search_query.to_lowercase();
        let terms: Vec<&str> = query.split_whitespace().collect();

        self.filtered_rows = self
            .all_rows
            .iter()
            .enumerate()
            .filter(|(_, row)| {
                let algo_idx = self
                    .algorithms
                    .iter()
                    .position(|algorithm| algorithm == &row.coin.algorithm);
                let method_idx = METHODS.iter().position(|method| method.id == row.method.id);

                let algo_ok = algo_idx
                    .map(|idx| self.selected_algorithms.get(idx).copied().unwrap_or(true))
                    .unwrap_or(true);
                let method_ok = method_idx
                    .map(|idx| self.selected_methods.get(idx).copied().unwrap_or(true))
                    .unwrap_or(true);

                let fit_ok = match self.fit_filter {
                    FitFilter::All => true,
                    FitFilter::Positive => row.net_usd_day > 0.0,
                    FitFilter::Prime => row.fit_level == FitLevel::Prime,
                    FitFilter::Viable => matches!(
                        row.fit_level,
                        FitLevel::Prime | FitLevel::Strong | FitLevel::Watch
                    ),
                    FitFilter::Risky => row.fit_level == FitLevel::Speculative,
                    FitFilter::Avoid => row.fit_level == FitLevel::Avoid,
                };

                let search_ok = if terms.is_empty() {
                    true
                } else {
                    let haystack = format!(
                        "{} {} {} {} {} {} {}",
                        row.coin.name.to_lowercase(),
                        row.coin.symbol.to_lowercase(),
                        row.coin.algorithm.to_lowercase(),
                        row.rig_name.to_lowercase(),
                        row.method.name.to_lowercase(),
                        row.method.description.to_lowercase(),
                        row.strategy_text().to_lowercase(),
                    );
                    terms.iter().all(|term| haystack.contains(term))
                };

                algo_ok && method_ok && fit_ok && search_ok
            })
            .map(|(idx, _)| idx)
            .collect();

        if self.filtered_rows.is_empty() {
            self.selected_row = 0;
            self.show_detail = false;
        } else if self.selected_row >= self.filtered_rows.len() {
            self.selected_row = self.filtered_rows.len() - 1;
        }
    }

    fn startup_status_message(&self, persisted_state: Option<&PersistedAppState>) -> String {
        let mut parts = vec![
            format!("Snapshot: {}", self.snapshot_badge()),
            format!("{} rankable", self.snapshot.rankable_coin_count()),
            format!("{} catalog", self.snapshot.catalog_asset_count()),
        ];
        if let Some(state) = persisted_state {
            parts.push(if state.hardware_changed(&self.specs) {
                "Hardware changed since last launch".to_string()
            } else {
                "Layout restored".to_string()
            });
        }
        parts.join(" | ")
    }

    fn apply_persisted_state(
        &mut self,
        state: &PersistedAppState,
    ) -> Option<(String, String, String)> {
        self.search_query = state.search_query.clone();
        self.cursor_position = self.search_query.len();
        self.fit_filter = state.fit_filter;
        self.sort_column = state.sort_column;
        self.sort_ascending = state.sort_ascending;
        self.show_detail = state.show_detail;

        if let Some(idx) = self
            .power_options
            .iter()
            .position(|option| option.plan_id == state.power.plan_id)
        {
            self.power_index = idx;
            self.power = self.power_options[idx].clone();
        }

        if !state.selected_algorithms.is_empty() {
            self.selected_algorithms = self
                .algorithms
                .iter()
                .map(|algorithm| state.selected_algorithms.iter().any(|saved| saved == algorithm))
                .collect();
            if !self.selected_algorithms.iter().any(|selected| *selected) {
                self.selected_algorithms = vec![true; self.algorithms.len()];
            }
        }

        if !state.selected_method_ids.is_empty() {
            self.selected_methods = METHODS
                .iter()
                .map(|method| {
                    state
                        .selected_method_ids
                        .iter()
                        .any(|saved| saved == method.id)
                })
                .collect();
            if !self.selected_methods.iter().any(|selected| *selected) {
                self.selected_methods = vec![true; METHODS.len()];
            }
        }

        match (
            state.selection.symbol.clone(),
            state.selection.method_id.clone(),
            state.selection.rig_name.clone(),
        ) {
            (Some(symbol), Some(method_id), Some(rig_name)) => Some((symbol, method_id, rig_name)),
            _ => None,
        }
    }

    pub fn persist_state(&self) {
        let selected_key = self.selected_key();
        let state = PersistedAppState::new(
            self.specs.clone(),
            self.power.clone(),
            self.sort_column,
            self.sort_ascending,
            self.fit_filter,
            self.search_query.clone(),
            self.algorithms
                .iter()
                .zip(self.selected_algorithms.iter())
                .filter_map(|(algorithm, selected)| if *selected { Some(algorithm.clone()) } else { None })
                .collect(),
            METHODS
                .iter()
                .zip(self.selected_methods.iter())
                .filter_map(|(method, selected)| if *selected { Some(method.id.to_string()) } else { None })
                .collect(),
            self.show_detail,
            PersistedSelection {
                symbol: selected_key.as_ref().map(|key| key.0.clone()),
                method_id: selected_key.as_ref().map(|key| key.1.clone()),
                rig_name: selected_key.as_ref().map(|key| key.2.clone()),
            },
        );
        save_persisted_state(&state);
    }
}

fn build_power_options(power: &PowerContext) -> Vec<PowerContext> {
    expand_power_context_options(power)
}
