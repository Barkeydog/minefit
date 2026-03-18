use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use time::{Month, OffsetDateTime};

const DEFAULT_FALLBACK_RATE_USD_KWH: f64 = 0.10;
const EIA_STATE_RATE_URL: &str =
    "https://www.eia.gov/electricity/monthly/epm_table_grapher.php?t=epmt_5_6_a";
const IPWHOIS_URL: &str = "https://ipwho.is/";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ElectricityMode {
    Estimated,
    Manual,
    Fallback,
}

#[derive(Debug, Clone, Serialize)]
pub struct ElectricityProfile {
    pub usd_per_kwh: f64,
    pub cents_per_kwh: f64,
    pub mode: ElectricityMode,
    pub region_code: Option<String>,
    pub region_name: Option<String>,
    pub city: Option<String>,
    pub postal_code: Option<String>,
    pub country_code: Option<String>,
    pub source: String,
    pub data_month: Option<String>,
    pub release_date: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PowerPlanId {
    StateAverage,
    Manual,
    Fallback,
    PgeE1Tier2,
    PgeETouC,
    PgeEv2A,
    SceTouD49,
    SceTouD58,
    SdgeStandardDrTier2,
    SdgeTouDr2Tier2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CaUtility {
    Pge,
    Sce,
    Sdge,
}

#[derive(Debug, Clone, Serialize)]
pub struct PowerContext {
    pub location_profile: ElectricityProfile,
    pub plan_id: PowerPlanId,
    pub plan_label: String,
    pub plan_description: String,
    pub source: String,
    pub effective_rate_hint_usd_kwh: f64,
    pub season_label: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PowerCostEstimate {
    pub daily_cost_usd: f64,
    pub monthly_cost_usd: f64,
    pub effective_rate_usd_kwh: f64,
    pub daily_energy_kwh: f64,
    pub monthly_energy_kwh: f64,
    pub breakdown: String,
}

impl ElectricityProfile {
    pub fn manual(rate: f64) -> Self {
        Self {
            usd_per_kwh: clamp_rate(rate),
            cents_per_kwh: clamp_rate(rate) * 100.0,
            mode: ElectricityMode::Manual,
            region_code: None,
            region_name: None,
            city: None,
            postal_code: None,
            country_code: None,
            source: "Manual override".to_string(),
            data_month: None,
            release_date: None,
            note: Some("Manual electricity rate supplied by the user.".to_string()),
        }
    }

    pub fn fallback(note: impl Into<String>) -> Self {
        let rate = DEFAULT_FALLBACK_RATE_USD_KWH;
        Self {
            usd_per_kwh: rate,
            cents_per_kwh: rate * 100.0,
            mode: ElectricityMode::Fallback,
            region_code: None,
            region_name: None,
            city: None,
            postal_code: None,
            country_code: None,
            source: "Fallback default".to_string(),
            data_month: None,
            release_date: None,
            note: Some(note.into()),
        }
    }

    pub fn short_badge(&self) -> String {
        match self.mode {
            ElectricityMode::Estimated => self
                .region_code
                .as_deref()
                .map(|code| format!("{code} est"))
                .unwrap_or_else(|| "est".to_string()),
            ElectricityMode::Manual => "manual".to_string(),
            ElectricityMode::Fallback => "fallback".to_string(),
        }
    }

    pub fn summary_line(&self) -> String {
        match self.mode {
            ElectricityMode::Estimated => {
                let region = self
                    .region_name
                    .as_deref()
                    .or(self.region_code.as_deref())
                    .unwrap_or("local");
                let mut parts = vec![format!("{region} residential estimate")];
                if let Some(month) = &self.data_month {
                    parts.push(month.clone());
                }
                parts.push(self.source.clone());
                if let Some(release_date) = &self.release_date {
                    parts.push(format!("released {release_date}"));
                }
                parts.join(" | ")
            }
            ElectricityMode::Manual => format!("Manual electricity rate at ${:.2}/kWh", self.usd_per_kwh),
            ElectricityMode::Fallback => format!(
                "Fallback electricity rate at ${:.2}/kWh{}",
                self.usd_per_kwh,
                self.note
                    .as_deref()
                    .map(|note| format!(" | {note}"))
                    .unwrap_or_default()
            ),
        }
    }
}

impl PowerContext {
    pub fn badge(&self) -> String {
        match self.plan_id {
            PowerPlanId::StateAverage => format!(
                "{} avg ${:.2}/kWh",
                self.location_profile.short_badge(),
                self.effective_rate_hint_usd_kwh
            ),
            PowerPlanId::Manual | PowerPlanId::Fallback => format!(
                "{} ${:.2}/kWh",
                self.location_profile.short_badge(),
                self.effective_rate_hint_usd_kwh
            ),
            PowerPlanId::PgeE1Tier2 => format!("PG&E E-1 ${:.2}/kWh", self.effective_rate_hint_usd_kwh),
            PowerPlanId::PgeETouC => format!("PG&E TOU-C ${:.2}/kWh", self.effective_rate_hint_usd_kwh),
            PowerPlanId::PgeEv2A => format!("PG&E EV2-A ${:.2}/kWh", self.effective_rate_hint_usd_kwh),
            PowerPlanId::SceTouD49 => format!("SCE 4-9 ${:.2}/kWh", self.effective_rate_hint_usd_kwh),
            PowerPlanId::SceTouD58 => format!("SCE 5-8 ${:.2}/kWh", self.effective_rate_hint_usd_kwh),
            PowerPlanId::SdgeStandardDrTier2 => {
                format!("SDG&E DR ${:.2}/kWh", self.effective_rate_hint_usd_kwh)
            }
            PowerPlanId::SdgeTouDr2Tier2 => {
                format!("SDG&E DR2 ${:.2}/kWh", self.effective_rate_hint_usd_kwh)
            }
        }
    }

    pub fn summary_line(&self) -> String {
        match self.plan_id {
            PowerPlanId::StateAverage | PowerPlanId::Manual | PowerPlanId::Fallback => {
                self.location_profile.summary_line()
            }
            _ => {
                let mut parts = vec![self.plan_label.clone(), self.plan_description.clone(), self.source.clone()];
                if let Some(season) = &self.season_label {
                    parts.push(format!("{season} season"));
                }
                parts.join(" | ")
            }
        }
    }

    pub fn estimate_cost(&self, power_watts: f64, days: f64) -> PowerCostEstimate {
        let power_kw = power_watts.max(0.0) / 1_000.0;
        let daily_energy_kwh = power_kw * 24.0;
        let monthly_energy_kwh = daily_energy_kwh * days;

        let (effective_rate_usd_kwh, breakdown) = match self.plan_id {
            PowerPlanId::StateAverage | PowerPlanId::Manual | PowerPlanId::Fallback => (
                self.location_profile.usd_per_kwh,
                self.location_profile.summary_line(),
            ),
            PowerPlanId::PgeE1Tier2 => (
                0.41,
                "PG&E Tiered Rate Plan (E-1) Tier 2 marginal rate, effective March 1, 2026."
                    .to_string(),
            ),
            PowerPlanId::PgeETouC => {
                let (off_peak, peak, season) = if is_summer_month() {
                    (0.40, 0.52, "Summer")
                } else {
                    (0.37, 0.40, "Winter")
                };
                let effective = ((19.0 * off_peak) + (5.0 * peak)) / 24.0;
                (
                    effective,
                    format!(
                        "PG&E E-TOU-C above-baseline marginal load: {season} off-peak ${off_peak:.2}/kWh for 19h, peak ${peak:.2}/kWh for 5h."
                    ),
                )
            }
            PowerPlanId::PgeEv2A => {
                let (off_peak, partial_peak, peak, season) = if is_summer_month() {
                    (0.23, 0.43, 0.54, "Summer")
                } else {
                    (0.23, 0.39, 0.41, "Winter")
                };
                let effective = ((15.0 * off_peak) + (4.0 * partial_peak) + (5.0 * peak)) / 24.0;
                (
                    effective,
                    format!(
                        "PG&E EV2-A marginal load: {season} off-peak ${off_peak:.2}, partial ${partial_peak:.2}, peak ${peak:.2}."
                    ),
                )
            }
            PowerPlanId::SceTouD49 => {
                if is_summer_month() {
                    let weekday = (19.0 * 0.22) + (5.0 * 0.41);
                    let weekend = (19.0 * 0.22) + (5.0 * 0.27);
                    let effective = ((5.0 * weekday) + (2.0 * weekend)) / (7.0 * 24.0);
                    (
                        effective,
                        "SCE TOU-D-4-9PM summer mix: weekdays 19h off-peak $0.22/kWh + 5h on-peak $0.41; weekends 19h off-peak $0.22 + 5h mid-peak $0.27."
                            .to_string(),
                    )
                } else {
                    let effective = ((8.0 * 0.17) + (5.0 * 0.29) + (11.0 * 0.28)) / 24.0;
                    (
                        effective,
                        "SCE TOU-D-4-9PM winter mix: 8h super off-peak $0.17/kWh, 5h mid-peak $0.29, 11h off-peak $0.28."
                            .to_string(),
                    )
                }
            }
            PowerPlanId::SceTouD58 => {
                if is_summer_month() {
                    let weekday = (21.0 * 0.23) + (3.0 * 0.49);
                    let weekend = (21.0 * 0.23) + (3.0 * 0.29);
                    let effective = ((5.0 * weekday) + (2.0 * weekend)) / (7.0 * 24.0);
                    (
                        effective,
                        "SCE TOU-D-5-8PM summer mix: weekdays 21h off-peak $0.23/kWh + 3h on-peak $0.49; weekends 21h off-peak $0.23 + 3h mid-peak $0.29."
                            .to_string(),
                    )
                } else {
                    let effective = ((9.0 * 0.17) + (3.0 * 0.30) + (12.0 * 0.29)) / 24.0;
                    (
                        effective,
                        "SCE TOU-D-5-8PM winter mix: 9h super off-peak $0.17/kWh, 3h mid-peak $0.30, 12h off-peak $0.29."
                            .to_string(),
                    )
                }
            }
            PowerPlanId::SdgeStandardDrTier2 => (
                0.53,
                "SDG&E Standard DR Tier 2 marginal rate at $0.53/kWh, excluding the separate monthly base-services charge."
                    .to_string(),
            ),
            PowerPlanId::SdgeTouDr2Tier2 => {
                let effective = ((19.0 * 0.485) + (5.0 * 0.622)) / 24.0;
                (
                    effective,
                    "SDG&E TOU-DR2 Tier 2 marginal load: 19h off-peak $0.485/kWh and 5h on-peak $0.622, excluding the monthly base-services charge."
                        .to_string(),
                )
            }
        };

        let daily_cost_usd = daily_energy_kwh * effective_rate_usd_kwh;
        let monthly_cost_usd = daily_cost_usd * days;
        PowerCostEstimate {
            daily_cost_usd,
            monthly_cost_usd,
            effective_rate_usd_kwh,
            daily_energy_kwh,
            monthly_energy_kwh,
            breakdown,
        }
    }
}

pub fn resolve_electricity_profile(
    manual_rate: Option<f64>,
    location_hint: Option<&str>,
) -> Result<ElectricityProfile, String> {
    if let Some(rate) = manual_rate {
        return Ok(ElectricityProfile::manual(rate));
    }

    let location = if let Some(hint) = location_hint {
        resolve_state_hint(hint)?
    } else {
        detect_location_from_ip()?
    };

    let table = fetch_state_rate_table()?;
    let cents_per_kwh = table
        .residential_cents
        .get(&location.region_name)
        .copied()
        .ok_or_else(|| {
            format!(
                "EIA state table did not contain a residential rate for {}",
                location.region_name
            )
        })?;

    let usd_per_kwh = cents_per_kwh / 100.0;
    Ok(ElectricityProfile {
        usd_per_kwh,
        cents_per_kwh,
        mode: ElectricityMode::Estimated,
        region_code: Some(location.region_code),
        region_name: Some(location.region_name),
        city: location.city,
        postal_code: location.postal_code,
        country_code: Some(location.country_code),
        source: "EIA state average retail residential electricity price".to_string(),
        data_month: Some(table.data_month),
        release_date: Some(table.release_date),
        note: Some(format!(
            "Estimated from a state-level average{}.",
            location
                .detection_source
                .as_deref()
                .map(|source| format!(" using {source}"))
                .unwrap_or_default()
        )),
    })
}

pub fn resolve_power_context(
    manual_rate: Option<f64>,
    location_hint: Option<&str>,
    power_plan_hint: Option<&str>,
) -> Result<PowerContext, String> {
    let profile = resolve_electricity_profile(manual_rate, location_hint)?;

    let plan_id = if manual_rate.is_some() {
        PowerPlanId::Manual
    } else if matches!(profile.mode, ElectricityMode::Fallback) {
        PowerPlanId::Fallback
    } else if let Some(hint) = power_plan_hint {
        parse_power_plan_id(hint)?
    } else if let Some(utility) = infer_ca_utility(&profile) {
        default_plan_for_utility(utility)
    } else {
        PowerPlanId::StateAverage
    };

    Ok(power_context_from_profile(profile, plan_id))
}

pub fn fallback_power_context(note: impl Into<String>) -> PowerContext {
    power_context_from_profile(
        ElectricityProfile::fallback(note.into()),
        PowerPlanId::Fallback,
    )
}

pub fn expand_power_context_options(context: &PowerContext) -> Vec<PowerContext> {
    match context.plan_id {
        PowerPlanId::Manual | PowerPlanId::Fallback => vec![context.clone()],
        _ => {
            let profile = context.location_profile.clone();
            let utility = utility_for_plan(context.plan_id).or_else(|| infer_ca_utility(&profile));
            match utility {
                Some(CaUtility::Pge) => vec![
                    power_context_from_profile(profile.clone(), PowerPlanId::StateAverage),
                    power_context_from_profile(profile.clone(), PowerPlanId::PgeE1Tier2),
                    power_context_from_profile(profile.clone(), PowerPlanId::PgeETouC),
                    power_context_from_profile(profile, PowerPlanId::PgeEv2A),
                ],
                Some(CaUtility::Sce) => vec![
                    power_context_from_profile(profile.clone(), PowerPlanId::StateAverage),
                    power_context_from_profile(profile.clone(), PowerPlanId::SceTouD49),
                    power_context_from_profile(profile, PowerPlanId::SceTouD58),
                ],
                Some(CaUtility::Sdge) => vec![
                    power_context_from_profile(profile.clone(), PowerPlanId::StateAverage),
                    power_context_from_profile(profile.clone(), PowerPlanId::SdgeStandardDrTier2),
                    power_context_from_profile(profile, PowerPlanId::SdgeTouDr2Tier2),
                ],
                None => vec![power_context_from_profile(profile, PowerPlanId::StateAverage)],
            }
        }
    }
}

fn power_context_from_profile(profile: ElectricityProfile, plan_id: PowerPlanId) -> PowerContext {
    match plan_id {
        PowerPlanId::StateAverage => PowerContext {
            effective_rate_hint_usd_kwh: profile.usd_per_kwh,
            plan_id,
            plan_label: format!(
                "{} residential state average",
                profile
                    .region_name
                    .clone()
                    .unwrap_or_else(|| "Local".to_string())
            ),
            plan_description: "Latest EIA state-average residential rate.".to_string(),
            source: profile.source.clone(),
            season_label: profile.data_month.clone(),
            location_profile: profile,
        },
        PowerPlanId::Manual => PowerContext {
            effective_rate_hint_usd_kwh: profile.usd_per_kwh,
            plan_id,
            plan_label: "Manual flat rate".to_string(),
            plan_description: "User-supplied flat electricity rate.".to_string(),
            source: profile.source.clone(),
            season_label: None,
            location_profile: profile,
        },
        PowerPlanId::Fallback => PowerContext {
            effective_rate_hint_usd_kwh: profile.usd_per_kwh,
            plan_id,
            plan_label: "Fallback flat rate".to_string(),
            plan_description: "Default fallback used when no better location estimate is available."
                .to_string(),
            source: profile.source.clone(),
            season_label: None,
            location_profile: profile,
        },
        PowerPlanId::PgeE1Tier2 => PowerContext {
            effective_rate_hint_usd_kwh: 0.41,
            plan_id,
            plan_label: "PG&E E-1 Tier 2".to_string(),
            plan_description: "Tiered marginal residential load pricing, using Tier 2 as the mining-load assumption."
                .to_string(),
            source: "PG&E residential rate plan pricing sheet, effective March 1, 2026.".to_string(),
            season_label: None,
            location_profile: profile,
        },
        PowerPlanId::PgeETouC => PowerContext {
            effective_rate_hint_usd_kwh: if is_summer_month() { 0.425 } else { 0.37625 },
            plan_id,
            plan_label: "PG&E E-TOU-C".to_string(),
            plan_description:
                "Time-of-use pricing using above-baseline marginal rates for a 24/7 mining load."
                    .to_string(),
            source: "PG&E residential rate plan pricing sheet, effective March 1, 2026.".to_string(),
            season_label: Some(current_season_label()),
            location_profile: profile,
        },
        PowerPlanId::PgeEv2A => PowerContext {
            effective_rate_hint_usd_kwh: if is_summer_month() {
                ((15.0 * 0.23) + (4.0 * 0.43) + (5.0 * 0.54)) / 24.0
            } else {
                ((15.0 * 0.23) + (4.0 * 0.39) + (5.0 * 0.41)) / 24.0
            },
            plan_id,
            plan_label: "PG&E EV2-A".to_string(),
            plan_description:
                "EV-style time-of-use pricing applied as a marginal 24/7 mining load.".to_string(),
            source: "PG&E residential rate plan pricing sheet, effective March 1, 2026.".to_string(),
            season_label: Some(current_season_label()),
            location_profile: profile,
        },
        PowerPlanId::SceTouD49 => PowerContext {
            effective_rate_hint_usd_kwh: if is_summer_month() {
                ((5.0 * ((19.0 * 0.22) + (5.0 * 0.41)))
                    + (2.0 * ((19.0 * 0.22) + (5.0 * 0.27))))
                    / (7.0 * 24.0)
            } else {
                ((8.0 * 0.17) + (5.0 * 0.29) + (11.0 * 0.28)) / 24.0
            },
            plan_id,
            plan_label: "SCE TOU-D-4-9PM".to_string(),
            plan_description:
                "Utility-specific TOU pricing using SCE's current residential plan-comparison rates for a 24/7 mining load."
                    .to_string(),
            source: "SCE residential rate plan comparison page.".to_string(),
            season_label: Some(current_season_label()),
            location_profile: profile,
        },
        PowerPlanId::SceTouD58 => PowerContext {
            effective_rate_hint_usd_kwh: if is_summer_month() {
                ((5.0 * ((21.0 * 0.23) + (3.0 * 0.49)))
                    + (2.0 * ((21.0 * 0.23) + (3.0 * 0.29))))
                    / (7.0 * 24.0)
            } else {
                ((9.0 * 0.17) + (3.0 * 0.30) + (12.0 * 0.29)) / 24.0
            },
            plan_id,
            plan_label: "SCE TOU-D-5-8PM".to_string(),
            plan_description:
                "Utility-specific TOU pricing using SCE's current residential plan-comparison rates for a 24/7 mining load."
                    .to_string(),
            source: "SCE residential rate plan comparison page.".to_string(),
            season_label: Some(current_season_label()),
            location_profile: profile,
        },
        PowerPlanId::SdgeStandardDrTier2 => PowerContext {
            effective_rate_hint_usd_kwh: 0.53,
            plan_id,
            plan_label: "SDG&E Standard DR Tier 2".to_string(),
            plan_description:
                "Utility-specific tiered marginal pricing, excluding SDG&E's separate monthly base-services charge."
                    .to_string(),
            source: "SDG&E residential pricing plans page, prices effective January 1, 2026."
                .to_string(),
            season_label: None,
            location_profile: profile,
        },
        PowerPlanId::SdgeTouDr2Tier2 => PowerContext {
            effective_rate_hint_usd_kwh: ((19.0 * 0.485) + (5.0 * 0.622)) / 24.0,
            plan_id,
            plan_label: "SDG&E TOU-DR2 Tier 2".to_string(),
            plan_description:
                "Utility-specific TOU pricing, using Tier 2 marginal rates for an always-on mining load and excluding the monthly base-services charge."
                    .to_string(),
            source: "SDG&E residential pricing plans page, prices effective January 1, 2026."
                .to_string(),
            season_label: None,
            location_profile: profile,
        },
    }
}

#[derive(Debug, Clone)]
struct LocationTarget {
    region_code: String,
    region_name: String,
    city: Option<String>,
    postal_code: Option<String>,
    country_code: String,
    detection_source: Option<String>,
}

#[derive(Debug)]
struct StateRateTable {
    data_month: String,
    release_date: String,
    residential_cents: HashMap<String, f64>,
}

fn fetch_state_rate_table() -> Result<StateRateTable, String> {
    let page = ureq::get(EIA_STATE_RATE_URL)
        .header("User-Agent", "minefit/0.7.4")
        .config()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .call()
        .map_err(|err| format!("EIA electricity request failed: {err}"))?
        .into_body()
        .read_to_string()
        .map_err(|err| format!("EIA electricity HTML read failed: {err}"))?;

    let data_month = extract_data_month(&page)
        .ok_or_else(|| "Unable to parse the EIA data month from the response".to_string())?;
    let release_date = extract_release_date(&page)
        .ok_or_else(|| "Unable to parse the EIA release date from the response".to_string())?;

    let mut residential_cents = HashMap::new();
    let mut pending_state: Option<String> = None;
    for line in page.lines().map(str::trim) {
        if pending_state.is_none()
            && line.starts_with("<td")
            && line.contains("data sel-23")
            && !line.contains("format:0.00")
        {
            if let Some(state) = extract_table_cell_text(line) {
                pending_state = Some(state);
            }
            continue;
        }

        if let Some(state) = pending_state.take() {
            if line.starts_with("<td") && line.contains("format:0.00") {
                let Some(raw_value) = extract_table_cell_text(line) else {
                    continue;
                };
                let parsed = raw_value.replace(',', "").parse::<f64>().ok();
                if let Some(value) = parsed {
                    residential_cents.insert(state, value);
                }
            }
        }
    }

    if residential_cents.is_empty() {
        return Err("Unable to parse residential state electricity rates from the EIA page".to_string());
    }

    Ok(StateRateTable {
        data_month,
        release_date,
        residential_cents,
    })
}

fn detect_location_from_ip() -> Result<LocationTarget, String> {
    let response = ureq::get(IPWHOIS_URL)
        .header("User-Agent", "minefit/0.7.4")
        .config()
        .timeout_global(Some(Duration::from_secs(8)))
        .build()
        .call()
        .map_err(|err| format!("IP geolocation request failed: {err}"))?;
    let payload: IpWhoIsResponse = response
        .into_body()
        .read_json()
        .map_err(|err| format!("IP geolocation JSON parse failed: {err}"))?;

    if !payload.success {
        return Err(payload
            .message
            .unwrap_or_else(|| "IP geolocation provider returned an unsuccessful response".to_string()));
    }

    if !payload.country_code.eq_ignore_ascii_case("US") {
        return Err(format!(
            "Automatic electricity estimation currently supports U.S. locations only; detected {}",
            payload.country_code
        ));
    }

    if payload.region_code.trim().is_empty() || payload.region.trim().is_empty() {
        return Err("IP geolocation did not return a usable U.S. state".to_string());
    }

    Ok(LocationTarget {
        region_code: payload.region_code.trim().to_uppercase(),
        region_name: payload.region.trim().to_string(),
        city: payload.city.filter(|value| !value.trim().is_empty()),
        postal_code: payload.postal.filter(|value| !value.trim().is_empty()),
        country_code: payload.country_code.trim().to_uppercase(),
        detection_source: Some("ipwho.is IP geolocation".to_string()),
    })
}

fn resolve_state_hint(hint: &str) -> Result<LocationTarget, String> {
    let normalized = normalize_hint(hint);
    if normalized.is_empty() {
        return Err("The --location hint was empty".to_string());
    }

    for (code, name) in US_STATES {
        if normalized == *code || normalized == normalize_hint(name) {
            return Ok(LocationTarget {
                region_code: (*code).to_string(),
                region_name: (*name).to_string(),
                city: None,
                postal_code: None,
                country_code: "US".to_string(),
                detection_source: Some("the --location flag".to_string()),
            });
        }
    }

    Err(format!(
        "Unsupported --location value '{hint}'. Use a U.S. state code like CA or a full state name."
    ))
}

fn extract_data_month(page: &str) -> Option<String> {
    for line in page.lines().map(str::trim) {
        let Some(index) = line.find("Data for ") else {
            continue;
        };
        let fragment = &line[index + "Data for ".len()..];
        let Some(end) = fragment.find('<') else {
            continue;
        };
        let value = fragment[..end].trim();
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

fn extract_release_date(page: &str) -> Option<String> {
    let mut capture_next_text = false;
    let marker = "<span class=\"date\">";

    for line in page.lines().map(str::trim) {
        if let Some(index) = line.find(marker) {
            let fragment = &line[index + marker.len()..];
            let text = fragment
                .split("</span>")
                .next()
                .unwrap_or_default()
                .trim()
                .to_string();
            if !text.is_empty() {
                return Some(text);
            }
            capture_next_text = true;
            continue;
        }

        if capture_next_text {
            let text = line.replace("</span>", "").trim().to_string();
            if !text.is_empty() {
                return Some(text);
            }
        }
    }

    None
}

fn extract_table_cell_text(line: &str) -> Option<String> {
    let start = line.find('>')?;
    let end = line.rfind("</td>")?;
    let value = line[start + 1..end].trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn clamp_rate(rate: f64) -> f64 {
    rate.max(0.0).min(1.0)
}

fn parse_power_plan_id(value: &str) -> Result<PowerPlanId, String> {
    match normalize_hint(value).as_str() {
        "STATE" | "STATEAVG" | "STATEAVERAGE" => Ok(PowerPlanId::StateAverage),
        "PGEE1" => Ok(PowerPlanId::PgeE1Tier2),
        "PGEETOUC" | "PGETOUPC" | "PGETOUC" => Ok(PowerPlanId::PgeETouC),
        "PGEEV2A" | "EV2A" => Ok(PowerPlanId::PgeEv2A),
        "SCETOUD49" | "SCETOUD49PM" | "SCED49" => Ok(PowerPlanId::SceTouD49),
        "SCETOUD58" | "SCETOUD58PM" | "SCED58" => Ok(PowerPlanId::SceTouD58),
        "SDGESTANDARDDR" | "SDGEDR" | "SDGESTANDARDDRTIER2" => {
            Ok(PowerPlanId::SdgeStandardDrTier2)
        }
        "SDGETOUDR2" | "SDGEDR2" | "SDGETOUDR2TIER2" => Ok(PowerPlanId::SdgeTouDr2Tier2),
        other => Err(format!(
            "Unsupported --power-plan '{value}'. Use state, pge-e1, pge-e-tou-c, pge-ev2-a, sce-tou-d-4-9pm, sce-tou-d-5-8pm, sdge-standard-dr, or sdge-tou-dr2. Parsed token: {other}"
        )),
    }
}

fn infer_ca_utility(profile: &ElectricityProfile) -> Option<CaUtility> {
    if !matches!(profile.mode, ElectricityMode::Estimated) {
        return None;
    }

    if !profile
        .region_code
        .as_deref()
        .map(|code| code.eq_ignore_ascii_case("CA"))
        .unwrap_or(false)
    {
        return None;
    }

    let city = profile.city.as_deref().unwrap_or_default().to_ascii_lowercase();
    let postal = profile.postal_code.as_deref().unwrap_or_default();
    let has_granular_location = !city.is_empty() || !postal.is_empty();

    if city == "hayward"
        || city == "oakland"
        || city == "berkeley"
        || city == "fremont"
        || city == "san jose"
        || city == "san francisco"
        || city == "palo alto"
        || postal.starts_with("945")
        || postal.starts_with("946")
        || postal.starts_with("947")
        || postal.starts_with("948")
    {
        return Some(CaUtility::Pge);
    }

    if city == "san diego"
        || city == "chula vista"
        || city == "carlsbad"
        || city == "oceanside"
        || city == "escondido"
        || city == "el cajon"
        || city == "encinitas"
        || city == "vista"
        || city == "poway"
        || city == "santee"
        || city == "la mesa"
        || city == "national city"
        || city == "imperial beach"
        || city == "san marcos"
        || city == "solana beach"
        || city == "del mar"
        || postal.starts_with("919")
        || postal.starts_with("920")
        || postal.starts_with("921")
    {
        return Some(CaUtility::Sdge);
    }

    if has_granular_location {
        return Some(CaUtility::Sce);
    }

    None
}

fn default_plan_for_utility(utility: CaUtility) -> PowerPlanId {
    match utility {
        CaUtility::Pge => PowerPlanId::PgeETouC,
        CaUtility::Sce => PowerPlanId::SceTouD58,
        CaUtility::Sdge => PowerPlanId::SdgeTouDr2Tier2,
    }
}

fn utility_for_plan(plan_id: PowerPlanId) -> Option<CaUtility> {
    match plan_id {
        PowerPlanId::PgeE1Tier2 | PowerPlanId::PgeETouC | PowerPlanId::PgeEv2A => {
            Some(CaUtility::Pge)
        }
        PowerPlanId::SceTouD49 | PowerPlanId::SceTouD58 => Some(CaUtility::Sce),
        PowerPlanId::SdgeStandardDrTier2 | PowerPlanId::SdgeTouDr2Tier2 => {
            Some(CaUtility::Sdge)
        }
        _ => None,
    }
}

fn current_season_label() -> String {
    if is_summer_month() {
        "Summer".to_string()
    } else {
        "Winter".to_string()
    }
}

fn is_summer_month() -> bool {
    matches!(
        OffsetDateTime::now_utc().month(),
        Month::June | Month::July | Month::August | Month::September
    )
}

fn normalize_hint(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_uppercase())
        .collect()
}

#[derive(Debug, Deserialize)]
struct IpWhoIsResponse {
    success: bool,
    #[serde(default)]
    country_code: String,
    #[serde(default)]
    region: String,
    #[serde(default)]
    region_code: String,
    city: Option<String>,
    #[serde(rename = "postal")]
    postal: Option<String>,
    message: Option<String>,
}

const US_STATES: &[(&str, &str)] = &[
    ("AL", "Alabama"),
    ("AK", "Alaska"),
    ("AZ", "Arizona"),
    ("AR", "Arkansas"),
    ("CA", "California"),
    ("CO", "Colorado"),
    ("CT", "Connecticut"),
    ("DE", "Delaware"),
    ("DC", "District of Columbia"),
    ("FL", "Florida"),
    ("GA", "Georgia"),
    ("HI", "Hawaii"),
    ("ID", "Idaho"),
    ("IL", "Illinois"),
    ("IN", "Indiana"),
    ("IA", "Iowa"),
    ("KS", "Kansas"),
    ("KY", "Kentucky"),
    ("LA", "Louisiana"),
    ("ME", "Maine"),
    ("MD", "Maryland"),
    ("MA", "Massachusetts"),
    ("MI", "Michigan"),
    ("MN", "Minnesota"),
    ("MS", "Mississippi"),
    ("MO", "Missouri"),
    ("MT", "Montana"),
    ("NE", "Nebraska"),
    ("NV", "Nevada"),
    ("NH", "New Hampshire"),
    ("NJ", "New Jersey"),
    ("NM", "New Mexico"),
    ("NY", "New York"),
    ("NC", "North Carolina"),
    ("ND", "North Dakota"),
    ("OH", "Ohio"),
    ("OK", "Oklahoma"),
    ("OR", "Oregon"),
    ("PA", "Pennsylvania"),
    ("RI", "Rhode Island"),
    ("SC", "South Carolina"),
    ("SD", "South Dakota"),
    ("TN", "Tennessee"),
    ("TX", "Texas"),
    ("UT", "Utah"),
    ("VT", "Vermont"),
    ("VA", "Virginia"),
    ("WA", "Washington"),
    ("WV", "West Virginia"),
    ("WI", "Wisconsin"),
    ("WY", "Wyoming"),
];

#[cfg(test)]
mod tests {
    use super::{
        CaUtility, ElectricityMode, ElectricityProfile, PowerPlanId, extract_data_month,
        extract_release_date, fetch_state_rate_table, infer_ca_utility, normalize_hint,
        parse_power_plan_id, resolve_state_hint,
    };

    #[test]
    fn normalizes_location_hints() {
        assert_eq!(normalize_hint("California"), "CALIFORNIA");
        assert_eq!(normalize_hint("District of Columbia"), "DISTRICTOFCOLUMBIA");
        assert_eq!(normalize_hint("ca"), "CA");
    }

    #[test]
    fn resolves_state_code_and_name() {
        let ca = resolve_state_hint("CA").expect("CA should resolve");
        assert_eq!(ca.region_name, "California");

        let ny = resolve_state_hint("New York").expect("name should resolve");
        assert_eq!(ny.region_code, "NY");
    }

    #[test]
    fn parses_eia_metadata_snippets() {
        let page = r#"
            <span class="responsive-container">Data for December 2025</span>
            <span class="label">Release Date:</span> <span class="date">
                February 24, 2026
            </span>
        "#;
        assert_eq!(extract_data_month(page).as_deref(), Some("December 2025"));
        assert_eq!(extract_release_date(page).as_deref(), Some("February 24, 2026"));
    }

    #[test]
    fn manual_profile_marks_mode() {
        let profile = ElectricityProfile::manual(0.17);
        assert_eq!(profile.mode, ElectricityMode::Manual);
        assert_eq!(profile.usd_per_kwh, 0.17);
    }

    #[test]
    fn fallback_profile_marks_mode() {
        let profile = ElectricityProfile::fallback("test");
        assert_eq!(profile.mode, ElectricityMode::Fallback);
    }

    #[test]
    fn parses_extended_power_plan_tokens() {
        assert_eq!(
            parse_power_plan_id("sce-tou-d-5-8pm").unwrap(),
            PowerPlanId::SceTouD58
        );
        assert_eq!(
            parse_power_plan_id("sdge-standard-dr").unwrap(),
            PowerPlanId::SdgeStandardDrTier2
        );
        assert_eq!(
            parse_power_plan_id("sdge-tou-dr2").unwrap(),
            PowerPlanId::SdgeTouDr2Tier2
        );
    }

    #[test]
    fn infers_ca_utility_from_city_and_zip() {
        let pge = ElectricityProfile {
            usd_per_kwh: 0.34,
            cents_per_kwh: 34.0,
            mode: ElectricityMode::Estimated,
            region_code: Some("CA".to_string()),
            region_name: Some("California".to_string()),
            city: Some("Hayward".to_string()),
            postal_code: Some("94541".to_string()),
            country_code: Some("US".to_string()),
            source: "test".to_string(),
            data_month: None,
            release_date: None,
            note: None,
        };
        assert_eq!(infer_ca_utility(&pge), Some(CaUtility::Pge));

        let sdge = ElectricityProfile {
            city: Some("San Diego".to_string()),
            postal_code: Some("92101".to_string()),
            ..pge.clone()
        };
        assert_eq!(infer_ca_utility(&sdge), Some(CaUtility::Sdge));

        let sce = ElectricityProfile {
            city: Some("Irvine".to_string()),
            postal_code: Some("92612".to_string()),
            ..pge
        };
        assert_eq!(infer_ca_utility(&sce), Some(CaUtility::Sce));
    }

    #[test]
    fn live_table_fetch_returns_some_rates() {
        let table = fetch_state_rate_table().expect("live EIA state rate table should parse");
        assert!(table.residential_cents.get("California").copied().unwrap_or_default() > 0.0);
        assert!(table.residential_cents.get("Washington").copied().unwrap_or_default() > 0.0);
    }
}
