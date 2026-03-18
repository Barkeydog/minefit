use crate::theme::ThemeColors;
use crate::tui_app::{App, FitFilter, InputMode};
use llmfit_core::mining::{FitLevel, MiningStrategy};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Clear, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table, TableState, Wrap,
    },
};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let tc = app.theme.colors();

    if tc.bg != Color::Reset {
        let bg = Block::default().style(Style::default().bg(tc.bg));
        frame.render_widget(bg, frame.area());
    }

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(1),
        ])
        .split(frame.area());

    draw_system_bar(frame, app, outer[0], &tc);
    draw_search_and_filters(frame, app, outer[1], &tc);

    if app.show_detail {
        draw_detail(frame, app, outer[2], &tc);
    } else {
        draw_table(frame, app, outer[2], &tc);
    }

    draw_status_bar(frame, app, outer[3], &tc);

    match app.input_mode {
        InputMode::AlgorithmPopup => draw_algorithm_popup(frame, app, &tc),
        InputMode::MethodPopup => draw_method_popup(frame, app, &tc),
        _ => {}
    }
}

fn draw_system_bar(frame: &mut Frame, app: &App, area: Rect, tc: &ThemeColors) {
    let gpu_info = if app.specs.gpus.is_empty() {
        format!("GPU: none ({})", app.specs.backend.label())
    } else {
        let primary = &app.specs.gpus[0];
        match primary.vram_gb {
            Some(vram) => format!(
                "GPU: {} ({:.1} GB{}, {})",
                primary.name,
                vram,
                if primary.count > 1 {
                    format!(" x{}", primary.count)
                } else {
                    String::new()
                },
                primary.backend.label()
            ),
            None => format!("GPU: {} ({})", primary.name, primary.backend.label()),
        }
    };

    let text = Line::from(vec![
        Span::styled(" CPU: ", Style::default().fg(tc.muted)),
        Span::styled(
            format!("{} ({} cores)", app.specs.cpu_name, app.specs.total_cpu_cores),
            Style::default().fg(tc.fg),
        ),
        Span::styled("  |  ", Style::default().fg(tc.muted)),
        Span::styled("RAM: ", Style::default().fg(tc.muted)),
        Span::styled(
            format!(
                "{:.1} GB avail / {:.1} GB total",
                app.specs.available_ram_gb, app.specs.total_ram_gb
            ),
            Style::default().fg(tc.accent),
        ),
        Span::styled("  |  ", Style::default().fg(tc.muted)),
        Span::styled(gpu_info, Style::default().fg(tc.accent_secondary)),
        Span::styled("  |  ", Style::default().fg(tc.muted)),
        Span::styled(
            format!("Rankable: {}", app.snapshot.rankable_coin_count()),
            Style::default().fg(tc.good),
        ),
        Span::styled("  |  ", Style::default().fg(tc.muted)),
        Span::styled(
            format!("Catalog: {}", app.snapshot.catalog_asset_count()),
            Style::default().fg(tc.accent),
        ),
        Span::styled("  |  ", Style::default().fg(tc.muted)),
        Span::styled(
            format!("Snapshot: {}", app.snapshot_badge()),
            Style::default().fg(tc.info),
        ),
        Span::styled("  |  ", Style::default().fg(tc.muted)),
        Span::styled(
            format!("Power: {}", app.power_badge()),
            Style::default().fg(tc.warning),
        ),
        Span::styled("  |  ", Style::default().fg(tc.muted)),
        Span::styled(
            format!("Rig: {}", app.rig_badge()),
            Style::default().fg(tc.good),
        ),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(tc.border))
        .title(" minefit ")
        .title_style(Style::default().fg(tc.title).add_modifier(Modifier::BOLD));

    frame.render_widget(Paragraph::new(text).block(block), area);
}

fn draw_search_and_filters(frame: &mut Frame, app: &App, area: Rect, tc: &ThemeColors) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(24),
            Constraint::Length(16),
            Constraint::Length(16),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(18),
            Constraint::Length(14),
        ])
        .split(area);

    let search_style = match app.input_mode {
        InputMode::Search => Style::default().fg(tc.accent_secondary),
        _ => Style::default().fg(tc.muted),
    };
    let search_text = if app.search_query.is_empty() && app.input_mode == InputMode::Normal {
        Line::from(Span::styled(
            "Press / to search coins, algos, or methods...",
            Style::default().fg(tc.muted),
        ))
    } else {
        Line::from(Span::styled(&app.search_query, Style::default().fg(tc.fg)))
    };
    frame.render_widget(
        Paragraph::new(search_text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(search_style)
                .title(" Search ")
                .title_style(search_style),
        ),
        chunks[0],
    );

    if app.input_mode == InputMode::Search {
        frame.set_cursor_position((chunks[0].x + app.cursor_position as u16 + 1, chunks[0].y + 1));
    }

    draw_summary_box(
        frame,
        chunks[1],
        " Algorithms (A) ",
        if app.visible_algorithm_count() == app.algorithms.len() {
            "All".to_string()
        } else {
            format!("{}/{}", app.visible_algorithm_count(), app.algorithms.len())
        },
        if app.visible_algorithm_count() == 0 {
            tc.error
        } else if app.visible_algorithm_count() == app.algorithms.len() {
            tc.good
        } else {
            tc.warning
        },
        tc,
    );

    draw_summary_box(
        frame,
        chunks[2],
        " Methods (M) ",
        if app.visible_method_count() == app.methods.len() {
            "All".to_string()
        } else {
            format!("{}/{}", app.visible_method_count(), app.methods.len())
        },
        if app.visible_method_count() == 0 {
            tc.error
        } else if app.visible_method_count() == app.methods.len() {
            tc.good
        } else {
            tc.warning
        },
        tc,
    );

    draw_summary_box(
        frame,
        chunks[3],
        " Sort [s] ",
        app.sort_column.label().to_string(),
        tc.accent,
        tc,
    );

    let fit_color = match app.fit_filter {
        FitFilter::All => tc.fg,
        FitFilter::Positive | FitFilter::Prime | FitFilter::Viable => tc.good,
        FitFilter::Risky => tc.warning,
        FitFilter::Avoid => tc.error,
    };
    draw_summary_box(
        frame,
        chunks[4],
        " Fit [f] ",
        app.fit_filter.label().to_string(),
        fit_color,
        tc,
    );

    draw_summary_box(
        frame,
        chunks[5],
        " Power [e] ",
        app.power_badge(),
        tc.warning,
        tc,
    );

    draw_summary_box(
        frame,
        chunks[6],
        " Theme [t] ",
        app.theme.label().to_string(),
        tc.info,
        tc,
    );
}

fn draw_summary_box(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    value: String,
    color: Color,
    tc: &ThemeColors,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(tc.border))
        .title(title)
        .title_style(Style::default().fg(tc.muted));
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!(" {}", value),
            Style::default().fg(color),
        )))
        .block(block),
        area,
    );
}

fn fit_color(level: FitLevel, tc: &ThemeColors) -> Color {
    match level {
        FitLevel::Prime => tc.fit_perfect,
        FitLevel::Strong => tc.fit_good,
        FitLevel::Watch => tc.warning,
        FitLevel::Speculative => tc.fit_marginal,
        FitLevel::Avoid => tc.fit_tight,
    }
}

fn strategy_color(strategy: MiningStrategy, tc: &ThemeColors) -> Color {
    match strategy {
        MiningStrategy::Pool => tc.mode_gpu,
        MiningStrategy::Hosted => tc.mode_moe,
        MiningStrategy::Solo => tc.mode_offload,
    }
}

fn draw_table(frame: &mut Frame, app: &mut App, area: Rect, tc: &ThemeColors) {
    if app.filtered_rows.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(tc.border))
            .title(" Comparison Matrix ")
            .title_style(Style::default().fg(tc.title).add_modifier(Modifier::BOLD));
        let text = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "No rows match the current filters.",
                Style::default().fg(tc.fg),
            )),
            Line::from(Span::styled(
                "Try clearing search, enabling more methods, or widening the fit filter.",
                Style::default().fg(tc.muted),
            )),
        ])
        .block(block)
        .wrap(Wrap { trim: false });
        frame.render_widget(text, area);
        return;
    }

    let header = Row::new(vec![
        Cell::from("Coin"),
        Cell::from("Algo"),
        Cell::from("Rig"),
        Cell::from("Method"),
        Cell::from("Score"),
        Cell::from("Net $/d"),
        Cell::from("Gross $/d"),
        Cell::from("Power"),
        Cell::from("Blk/mo"),
        Cell::from("Trend"),
        Cell::from("Fit"),
        Cell::from("Liq"),
    ])
    .style(Style::default().fg(tc.accent).add_modifier(Modifier::BOLD));

    let visible_rows = area.height.saturating_sub(4) as usize;
    let viewport_start = if app.selected_row >= visible_rows && visible_rows > 0 {
        app.selected_row + 1 - visible_rows
    } else {
        0
    };

    let rows: Vec<Row> = app
        .filtered_rows
        .iter()
        .enumerate()
        .skip(viewport_start)
        .take(visible_rows.max(1))
        .map(|(row_idx, row_id)| {
            let row = &app.all_rows[*row_id];
            let selected = row_idx == app.selected_row;
            let base_style = if selected {
                Style::default().bg(tc.highlight_bg).fg(tc.fg)
            } else {
                Style::default().fg(tc.fg)
            };
            let trend_style = if row.trend_delta_pct >= 0.0 {
                base_style.fg(tc.good)
            } else {
                base_style.fg(tc.error)
            };
            let net_style = if row.net_usd_day >= 0.0 {
                base_style.fg(tc.good)
            } else {
                base_style.fg(tc.error)
            };

            Row::new(vec![
                Cell::from(format!("{} {}", row.coin.symbol, truncate(&row.coin.name, 10))).style(base_style),
                Cell::from(truncate(&row.coin.algorithm, 12)).style(base_style),
                Cell::from(truncate(&row.rig_name, 12)).style(base_style.fg(tc.info)),
                Cell::from(row.method.name).style(base_style.fg(strategy_color(row.method.strategy, tc))),
                Cell::from(format!("{:.0}", row.score)).style(base_style.fg(score_color(row.score, tc))),
                Cell::from(format!("{:.2}", row.net_usd_day)).style(net_style),
                Cell::from(format!("{:.2}", row.gross_usd_day)).style(base_style),
                Cell::from(format!("{:.2}", row.power_cost_usd_day)).style(base_style),
                Cell::from(format!("{:.2}", row.blocks_month)).style(base_style),
                Cell::from(format!("{:+.1}%", row.trend_delta_pct)).style(trend_style),
                Cell::from(row.fit_text()).style(base_style.fg(fit_color(row.fit_level, tc))),
                Cell::from(format_liquidity(row.coin.market_cap_usd, row.coin.volume_24h_usd))
                    .style(base_style),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(14),
        Constraint::Length(13),
        Constraint::Length(14),
        Constraint::Length(13),
        Constraint::Length(7),
        Constraint::Length(11),
        Constraint::Length(11),
        Constraint::Length(9),
        Constraint::Length(9),
        Constraint::Length(9),
        Constraint::Length(12),
        Constraint::Length(10),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(tc.border))
                .title(" Comparison Matrix ")
                .title_style(Style::default().fg(tc.title).add_modifier(Modifier::BOLD)),
        )
        .row_highlight_style(Style::default().bg(tc.highlight_bg))
        .highlight_symbol(">> ");

    let mut state = TableState::default().with_selected(Some(app.selected_row.saturating_sub(viewport_start)));
    frame.render_stateful_widget(table, area, &mut state);

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
    let scrollbar_state = ScrollbarState::new(app.filtered_rows.len()).position(app.selected_row);
    frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state.clone());
}

fn draw_detail(frame: &mut Frame, app: &App, area: Rect, tc: &ThemeColors) {
    let Some(row) = app.selected_fit() else {
        return;
    };

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(6),
            Constraint::Length(9),
            Constraint::Min(8),
        ])
        .split(area);

    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                format!("{} ", row.coin.symbol),
                Style::default()
                    .fg(tc.title)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(&row.coin.name, Style::default().fg(tc.fg)),
            Span::styled("  |  ", Style::default().fg(tc.muted)),
            Span::styled(&row.coin.algorithm, Style::default().fg(tc.accent)),
            Span::styled("  |  ", Style::default().fg(tc.muted)),
            Span::styled(row.method.name, Style::default().fg(strategy_color(row.method.strategy, tc))),
            Span::styled("  |  ", Style::default().fg(tc.muted)),
            Span::styled(row.fit_text(), Style::default().fg(fit_color(row.fit_level, tc))),
        ]),
        Line::from(Span::styled(
            row.method.description,
            Style::default().fg(tc.muted),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(tc.border))
            .title(" Detail ")
            .title_style(Style::default().fg(tc.title).add_modifier(Modifier::BOLD)),
    );
    frame.render_widget(header, sections[0]);

    let metrics = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(sections[1]);

    draw_metric_box(frame, metrics[0], "Score", &format!("{:.0}/100", row.score), tc.score_high, tc);
    draw_metric_box(frame, metrics[1], "Net/day", &format!("${:.2}", row.net_usd_day), if row.net_usd_day >= 0.0 { tc.good } else { tc.error }, tc);
    draw_metric_box(frame, metrics[2], "Hashrate", &format_hashrate(row.hashrate_hs), tc.accent, tc);
    draw_metric_box(frame, metrics[3], "Blocks/mo", &format!("{:.2}", row.blocks_month), tc.info, tc);

    let middle = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(sections[2]);

    let economics = Paragraph::new(vec![
        detail_line("Gross/day", &format!("${:.2}", row.gross_usd_day), tc),
        detail_line("Power/day", &format!("${:.2}", row.power_cost_usd_day), tc),
        detail_line("Power/mo", &format!("${:.2}", row.power_cost_usd_month), tc),
        detail_line("Fees/day", &format!("${:.2}", row.fee_cost_usd_day), tc),
        detail_line("Stale/day", &format!("${:.2}", row.stale_cost_usd_day), tc),
        detail_line("Service/day", &format!("${:.2}", row.service_cost_usd_day), tc),
        detail_line("Zero-Blk %", &format!("{:.1}%", row.variance_zero_block_pct), tc),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(tc.border))
            .title(" Economics ")
            .title_style(Style::default().fg(tc.accent_secondary)),
    );
    frame.render_widget(economics, middle[0]);

    let market = Paragraph::new(vec![
        detail_line("Price", &format!("${:.6}", row.coin.price_usd), tc),
        detail_line(
            "Liquidity",
            &format_liquidity(row.coin.market_cap_usd, row.coin.volume_24h_usd),
            tc,
        ),
        detail_line("Block Time", &format!("{:.0}s", row.coin.block_time_sec), tc),
        detail_line("Block Reward", &format!("{:.4}", row.coin.block_reward), tc),
        detail_line("24h Price", &format!("{:+.1}%", row.coin.price_trend_pct), tc),
        detail_line("24h Difficulty", &format!("{:+.1}%", row.coin.difficulty_trend_pct), tc),
        detail_line("Rig", &row.rig_name, tc),
        detail_line("Payout", row.method.payout_mode.label(), tc),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(tc.border))
            .title(" Coin Snapshot ")
            .title_style(Style::default().fg(tc.accent_secondary)),
    );
    frame.render_widget(market, middle[1]);

    let mut reason_lines = vec![
        Line::from(vec![
            Span::styled("Profit ", Style::default().fg(tc.muted)),
            Span::styled(format!("{:.0}", row.profit_score), Style::default().fg(tc.good)),
            Span::styled(" | Liquidity ", Style::default().fg(tc.muted)),
            Span::styled(format!("{:.0}", row.liquidity_score), Style::default().fg(tc.accent)),
            Span::styled(" | Trend ", Style::default().fg(tc.muted)),
            Span::styled(format!("{:.0}", row.trend_score), Style::default().fg(tc.warning)),
            Span::styled(" | Stability ", Style::default().fg(tc.muted)),
            Span::styled(format!("{:.0}", row.stability_score), Style::default().fg(tc.info)),
        ]),
        Line::from(""),
    ];
    for reason in row.reason_lines() {
        reason_lines.push(Line::from(Span::styled(format!("- {}", reason), Style::default().fg(tc.fg))));
    }
    reason_lines.push(Line::from(""));
    reason_lines.push(Line::from(Span::styled(
        format!(
            "Rig: {} | Power: {} | Feed: {}",
            row.rig_name,
            app.power_badge(),
            app.snapshot.source
        ),
        Style::default().fg(tc.muted),
    )));
    reason_lines.push(Line::from(Span::styled(
        format!("Power source: {}", app.power.summary_line()),
        Style::default().fg(tc.muted),
    )));
    reason_lines.push(Line::from(Span::styled(
        format!(
            "Benchmark: {} | {} | p50 monthly ${:.2} | p90 ${:.2}",
            row.benchmark_miner, row.benchmark_tuning, row.variance_p50_usd_month, row.variance_p90_usd_month
        ),
        Style::default().fg(tc.muted),
    )));

    let reasons = Paragraph::new(reason_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(tc.border))
                .title(" Why It Ranks Here ")
                .title_style(Style::default().fg(tc.title).add_modifier(Modifier::BOLD)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(reasons, sections[3]);
}

fn draw_metric_box(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    value: &str,
    color: Color,
    tc: &ThemeColors,
) {
    let paragraph = Paragraph::new(vec![
        Line::from(Span::styled(title, Style::default().fg(tc.muted))),
        Line::from(Span::styled(
            value,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(tc.border)),
    );
    frame.render_widget(paragraph, area);
}

fn detail_line(label: &str, value: &str, tc: &ThemeColors) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label}: "), Style::default().fg(tc.muted)),
        Span::styled(value.to_string(), Style::default().fg(tc.fg)),
    ])
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect, tc: &ThemeColors) {
    let (keys, mode_text) = match app.input_mode {
        InputMode::Normal => (
            " j/k:nav  Enter:detail  /:search  A:algorithms  M:methods  f:fit  s:sort  e:power  R:refresh  t:theme  q:quit".to_string(),
            "NORMAL".to_string(),
        ),
        InputMode::Search => (
            " type:search  Enter/Esc:done  Ctrl-U:clear".to_string(),
            "SEARCH".to_string(),
        ),
        InputMode::AlgorithmPopup => (
            " j/k:move  Space:toggle  a:all/none  Esc:close".to_string(),
            "ALGORITHMS".to_string(),
        ),
        InputMode::MethodPopup => (
            " j/k:move  Space:toggle  a:all/none  Esc:close".to_string(),
            "METHODS".to_string(),
        ),
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length((app.status_message.len() as u16 + 3).min(area.width / 2))])
        .split(area);

    let status_line = Line::from(vec![
        Span::styled(
            format!(" {} ", mode_text),
            Style::default().fg(tc.status_fg).bg(tc.status_bg).bold(),
        ),
        Span::styled(keys, Style::default().fg(tc.muted)),
    ]);
    frame.render_widget(Paragraph::new(status_line), chunks[0]);

    if !app.status_message.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" {} ", app.status_message),
                Style::default().fg(tc.info),
            ))),
            chunks[1],
        );
    }
}

fn draw_algorithm_popup(frame: &mut Frame, app: &App, tc: &ThemeColors) {
    draw_checkbox_popup(
        frame,
        frame.area(),
        " Algorithms ",
        &app.algorithms,
        &app.selected_algorithms,
        app.algorithm_cursor,
        tc,
    );
}

fn draw_method_popup(frame: &mut Frame, app: &App, tc: &ThemeColors) {
    draw_checkbox_popup(
        frame,
        frame.area(),
        " Methods ",
        &app.methods,
        &app.selected_methods,
        app.method_cursor,
        tc,
    );
}

fn draw_checkbox_popup(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    items: &[String],
    selected: &[bool],
    cursor: usize,
    tc: &ThemeColors,
) {
    let max_name_len = items.iter().map(|item| item.len()).max().unwrap_or(12);
    let popup_width = (max_name_len as u16 + 10).min(area.width.saturating_sub(4));
    let popup_height = (items.len() as u16 + 2).min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup = Rect::new(x, y, popup_width, popup_height);
    let inner_height = popup_height.saturating_sub(2) as usize;
    let scroll_offset = if cursor >= inner_height && inner_height > 0 {
        cursor - inner_height + 1
    } else {
        0
    };

    frame.render_widget(Clear, popup);

    let lines: Vec<Line> = items
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(inner_height.max(1))
        .map(|(idx, item)| {
            let checkbox = if selected.get(idx).copied().unwrap_or(false) {
                "[x]"
            } else {
                "[ ]"
            };
            let style = if idx == cursor {
                Style::default()
                    .fg(if selected.get(idx).copied().unwrap_or(false) {
                        tc.good
                    } else {
                        tc.fg
                    })
                    .bg(tc.highlight_bg)
                    .add_modifier(Modifier::BOLD)
            } else if selected.get(idx).copied().unwrap_or(false) {
                Style::default().fg(tc.good)
            } else {
                Style::default().fg(tc.muted)
            };

            Line::from(Span::styled(format!(" {} {}", checkbox, item), style))
        })
        .collect();

    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(tc.accent_secondary))
                .title(title)
                .title_style(Style::default().fg(tc.accent_secondary).add_modifier(Modifier::BOLD)),
        ),
        popup,
    );
}

fn score_color(score: f64, tc: &ThemeColors) -> Color {
    if score >= 80.0 {
        tc.score_high
    } else if score >= 60.0 {
        tc.score_mid
    } else {
        tc.score_low
    }
}

fn format_liquidity(market_cap_usd: f64, volume_24h_usd: f64) -> String {
    if market_cap_usd > 0.0 {
        format_compact_usd(market_cap_usd)
    } else if volume_24h_usd > 0.0 {
        format!("Vol {}", format_compact_usd(volume_24h_usd))
    } else {
        "n/a".to_string()
    }
}

fn format_compact_usd(value: f64) -> String {
    if value >= 1_000_000_000.0 {
        format!("{:.1}B", value / 1_000_000_000.0)
    } else if value >= 1_000_000.0 {
        format!("{:.1}M", value / 1_000_000.0)
    } else if value >= 1_000.0 {
        format!("{:.0}K", value / 1_000.0)
    } else {
        format!("{value:.0}")
    }
}

fn format_hashrate(value: f64) -> String {
    let units = ["H/s", "kH/s", "MH/s", "GH/s", "TH/s", "PH/s", "EH/s"];
    let mut current = value;
    let mut idx = 0usize;
    while current >= 1000.0 && idx + 1 < units.len() {
        current /= 1000.0;
        idx += 1;
    }
    if current >= 100.0 {
        format!("{current:.0} {}", units[idx])
    } else if current >= 10.0 {
        format!("{current:.1} {}", units[idx])
    } else {
        format!("{current:.2} {}", units[idx])
    }
}

fn truncate(value: &str, width: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= width {
        value.to_string()
    } else {
        format!("{}...", value.chars().take(width.saturating_sub(3)).collect::<String>())
    }
}
