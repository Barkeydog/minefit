mod persistence;
mod theme;
mod tui_app;
mod tui_events;
mod tui_ui;

use clap::Parser;
use persistence::{PersistedAppState, load_persisted_state};
use llmfit_core::{
    MiningRigProfile, PowerContext, SnapshotCacheStatus, build_rankings_for_rigs,
    describe_rig_scope,
    describe_rig_scope_summary, fallback_power_context, resolve_detected_rig_profiles,
    resolve_power_context, expand_power_context_options,
};
use llmfit_core::hardware::{SystemSpecs, parse_memory_size};
use llmfit_core::mining::{MiningSnapshot, SortColumn, sort_rankings};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use serde_json::json;
use std::io::Stdout;

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum SortArg {
    Score,
    Net,
    Gross,
    Blocks,
    Trend,
    Market,
}

impl From<SortArg> for SortColumn {
    fn from(value: SortArg) -> Self {
        match value {
            SortArg::Score => SortColumn::Score,
            SortArg::Net => SortColumn::NetUsd,
            SortArg::Gross => SortColumn::GrossUsd,
            SortArg::Blocks => SortColumn::BlocksPerMonth,
            SortArg::Trend => SortColumn::Trend,
            SortArg::Market => SortColumn::MarketCap,
        }
    }
}

#[derive(Parser)]
#[command(name = "minefit")]
#[command(about = "Live crypto mining comparisons in an llmfit-style terminal UI")]
#[command(version)]
struct Cli {
    /// Use classic CLI table output instead of the TUI
    #[arg(long)]
    cli: bool,

    /// Output rows as JSON for scripting
    #[arg(long)]
    json: bool,

    /// Limit rows in CLI/JSON mode
    #[arg(short = 'n', long)]
    limit: Option<usize>,

    /// Sort column for CLI output
    #[arg(long, value_enum)]
    sort: Option<SortArg>,

    /// Electricity rate in USD/kWh
    #[arg(long)]
    electricity: Option<f64>,

    /// U.S. state code or full name used for electricity estimation (e.g. CA, Texas)
    #[arg(long)]
    location: Option<String>,

    /// Power plan override (state, pge-e1, pge-e-tou-c, pge-ev2-a, sce-tou-d-4-9pm, sce-tou-d-5-8pm, sdge-standard-dr, sdge-tou-dr2)
    #[arg(long = "power-plan")]
    power_plan: Option<String>,

    /// Override detected GPU VRAM size (e.g. 24G, 32000M)
    #[arg(long, value_name = "SIZE")]
    memory: Option<String>,
}

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}

fn detect_specs(memory_override: &Option<String>) -> SystemSpecs {
    let specs = SystemSpecs::detect();
    if let Some(mem_str) = memory_override {
        match parse_memory_size(mem_str) {
            Some(gb) => specs.with_gpu_memory_override(gb),
            None => {
                eprintln!(
                    "Warning: could not parse --memory value '{}'. Expected 24G, 32000M, 1.5T.",
                    mem_str
                );
                specs
            }
        }
    } else {
        specs
    }
}

fn run_cli(
    specs: &SystemSpecs,
    snapshot: &MiningSnapshot,
    snapshot_status: &SnapshotCacheStatus,
    mut rows: Vec<llmfit_core::MiningRow>,
    sort: SortColumn,
    limit: Option<usize>,
    power: &PowerContext,
    rigs: &[MiningRigProfile],
) {
    sort_rankings(&mut rows, sort, false);

    println!(
        "minefit | snapshot {} | BTC ${:.2} | power {}",
        snapshot_status.badge(),
        snapshot.btc_usd,
        power.badge()
    );
    println!("Power source: {}", power.summary_line());
    println!(
        "Rig scope: {} | {}",
        describe_rig_scope(rigs),
        describe_rig_scope_summary(rigs)
    );
    specs.display();
    println!();
    println!(
        "{:<12} {:<12} {:<12} {:<12} {:>6} {:>11} {:>11} {:>10} {:>8} {:<12}",
        "Coin", "Algo", "Rig", "Method", "Score", "Net/day", "Gross/day", "Blocks/mo", "Trend", "Fit"
    );
    println!("{}", "-".repeat(116));

    for row in rows.into_iter().take(limit.unwrap_or(25)) {
        println!(
            "{:<12} {:<12} {:<12} {:<12} {:>6.0} {:>11.2} {:>11.2} {:>10.2} {:>7.1}% {:<12}",
            format!("{} ({})", row.coin.symbol, row.coin.name.chars().take(3).collect::<String>()),
            cli_truncate(&row.coin.algorithm, 12),
            cli_truncate(&row.rig_name, 12),
            cli_truncate(row.method.name, 12),
            row.score,
            row.net_usd_day,
            row.gross_usd_day,
            row.blocks_month,
            row.trend_delta_pct,
            row.fit_text()
        );
    }
}

#[allow(dead_code)]
fn truncate(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        value.to_string()
    } else {
        value.chars().take(width.saturating_sub(1)).collect::<String>() + "…"
    }
}

fn cli_truncate(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        value.to_string()
    } else {
        format!(
            "{}...",
            value.chars().take(width.saturating_sub(3)).collect::<String>()
        )
    }
}

fn draw_boot_screen(
    terminal: &mut ratatui::Terminal<CrosstermBackend<Stdout>>,
    message: &str,
) -> std::io::Result<()> {
    terminal.draw(|frame| {
        let area = frame.area();
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(45),
                Constraint::Length(3),
                Constraint::Percentage(52),
            ])
            .split(area);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" minefit ")
            .title_style(Style::default().add_modifier(Modifier::BOLD));
        let line = Line::from(vec![
            Span::raw(" "),
            Span::styled("Loading: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(message),
        ]);
        frame.render_widget(Paragraph::new(line).block(block), layout[1]);
    })?;
    Ok(())
}

fn run_tui(
    specs: SystemSpecs,
    snapshot: MiningSnapshot,
    snapshot_status: SnapshotCacheStatus,
    power: PowerContext,
    rigs: Vec<MiningRigProfile>,
    sort: SortColumn,
    persisted_state: Option<&PersistedAppState>,
) -> std::io::Result<()> {
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;
    draw_boot_screen(&mut terminal, "Building mining comparison matrix...")?;

    let mut app = tui_app::App::new(
        specs,
        snapshot,
        snapshot_status,
        power,
        rigs,
        sort,
        persisted_state,
    );

    loop {
        terminal.draw(|frame| {
            tui_ui::draw(frame, &mut app);
        })?;

        tui_events::handle_events(&mut app)?;

        if app.should_quit {
            break;
        }
    }

    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    let persisted_state = load_persisted_state();
    let manual_electricity = cli.electricity.map(|value| clamp(value, 0.0, 1.0));
    let sort = cli
        .sort
        .map(SortColumn::from)
        .or_else(|| persisted_state.as_ref().map(|state| state.sort_column))
        .unwrap_or(SortColumn::Score);

    let specs = detect_specs(&cli.memory);
    let active_rigs = resolve_detected_rig_profiles(&specs);
    let resolved_power = resolve_power_context(
        manual_electricity,
        cli.location.as_deref(),
        cli.power_plan.as_deref(),
    )
    .unwrap_or_else(fallback_power_context);
    let power = restore_saved_power(
        resolved_power,
        &cli,
        persisted_state.as_ref(),
    );
    let snapshot_load = match MiningSnapshot::load_startup_snapshot() {
        Ok(snapshot) => snapshot,
        Err(err) => {
            eprintln!("Error: {err}");
            std::process::exit(1);
        }
    };
    let snapshot_status = snapshot_load.status.clone();
    let snapshot = snapshot_load.snapshot;

    let mut rows = build_rankings_for_rigs(&snapshot, &power, &active_rigs, 1.0);
    sort_rankings(&mut rows, sort, false);

    if cli.json {
        let output_rows = cli
            .limit
            .map(|limit| rows.iter().take(limit).cloned().collect::<Vec<_>>())
            .unwrap_or_else(|| rows.clone());
        let payload = json!({
            "system": specs,
            "snapshot": snapshot,
            "snapshot_status": snapshot_status,
            "rankable_coin_count": snapshot.rankable_coin_count(),
            "catalog_asset_count": snapshot.catalog_asset_count(),
            "power": power,
            "rig_scope": describe_rig_scope(&active_rigs),
            "rig_profiles": active_rigs,
            "rows": output_rows,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).expect("json serialization should succeed")
        );
        return;
    }

    if cli.cli {
        run_cli(
            &specs,
            &snapshot,
            &snapshot_status,
            rows,
            sort,
            cli.limit,
            &power,
            &active_rigs,
        );
        return;
    }

    if let Err(err) = run_tui(
        specs,
        snapshot,
        snapshot_status,
        power,
        active_rigs,
        sort,
        persisted_state.as_ref(),
    ) {
        eprintln!("Error running TUI: {err}");
        std::process::exit(1);
    }
}

fn restore_saved_power(
    default_power: PowerContext,
    cli: &Cli,
    persisted_state: Option<&PersistedAppState>,
) -> PowerContext {
    if cli.electricity.is_some() || cli.location.is_some() || cli.power_plan.is_some() {
        return default_power;
    }

    let Some(state) = persisted_state else {
        return default_power;
    };
    expand_power_context_options(&default_power)
        .into_iter()
        .find(|option| option.plan_id == state.power.plan_id)
        .unwrap_or(default_power)
}
