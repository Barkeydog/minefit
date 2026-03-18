use crate::electricity::PowerContext;
use crate::rig_profiles::{AlgorithmRule, MiningRigProfile, RigKind, algorithm_rule};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const HASHRATE_CPU_SYMBOLS: &[&str] = &[
    "EPIC", "XTM-RX", "DERO", "XMR", "SAL", "QRL", "ZEPH", "VRSC", "ETI", "FBIT", "RTM",
    "SCASH", "XKR", "XEL",
];
const HASHRATE_TIER_ONE_MIN_VOLUME_USD: f64 = 1_000.0;
const MININGPOOLSTATS_HOME_URL: &str = "https://miningpoolstats.stream/";
const MININGPOOLSTATS_DATA_BASE_URL: &str = "https://data.miningpoolstats.stream/data";
const MININGPOOLSTATS_BROWSER_UA: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36";
const MININGPOOLSTATS_MIN_MARKET_CAP_USD: f64 = 10_000.0;
const MININGPOOLSTATS_MIN_VOLUME_USD: f64 = 100.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SortColumn {
    Score,
    NetUsd,
    GrossUsd,
    BlocksPerMonth,
    Trend,
    MarketCap,
}

impl SortColumn {
    pub fn label(&self) -> &'static str {
        match self {
            SortColumn::Score => "Score",
            SortColumn::NetUsd => "Net $/d",
            SortColumn::GrossUsd => "Gross $/d",
            SortColumn::BlocksPerMonth => "Blocks/mo",
            SortColumn::Trend => "Trend",
            SortColumn::MarketCap => "Liquidity",
        }
    }

    pub fn next(self) -> Self {
        match self {
            SortColumn::Score => SortColumn::NetUsd,
            SortColumn::NetUsd => SortColumn::GrossUsd,
            SortColumn::GrossUsd => SortColumn::BlocksPerMonth,
            SortColumn::BlocksPerMonth => SortColumn::Trend,
            SortColumn::Trend => SortColumn::MarketCap,
            SortColumn::MarketCap => SortColumn::Score,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum FitLevel {
    Prime,
    Strong,
    Watch,
    Speculative,
    Avoid,
}

impl FitLevel {
    pub fn label(&self) -> &'static str {
        match self {
            FitLevel::Prime => "Prime",
            FitLevel::Strong => "Strong",
            FitLevel::Watch => "Watch",
            FitLevel::Speculative => "Speculative",
            FitLevel::Avoid => "Avoid",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum MiningStrategy {
    Pool,
    Solo,
    Hosted,
}

impl MiningStrategy {
    pub fn label(&self) -> &'static str {
        match self {
            MiningStrategy::Pool => "Pool",
            MiningStrategy::Solo => "Solo",
            MiningStrategy::Hosted => "Hosted",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PayoutMode {
    Pplns,
    PpsPlus,
    SoloPool,
    SoloNode,
}

impl PayoutMode {
    pub fn label(&self) -> &'static str {
        match self {
            PayoutMode::Pplns => "PPLNS",
            PayoutMode::PpsPlus => "PPS+",
            PayoutMode::SoloPool => "Solo Pool",
            PayoutMode::SoloNode => "Solo Node",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MiningMethod {
    pub id: &'static str,
    pub name: &'static str,
    pub strategy: MiningStrategy,
    pub payout_mode: PayoutMode,
    pub description: &'static str,
    pub pool_fee_rate: f64,
    pub stale_rate_pct: f64,
    pub uptime_pct: f64,
    pub service_fee_usd_day: f64,
    pub score_bias: f64,
    pub source: &'static str,
    pub supported_algorithms: &'static [&'static str],
    pub supported_rig_kinds: &'static [RigKind],
}

impl MiningMethod {
    pub fn supports_algorithm(&self, algorithm: &str) -> bool {
        self.supported_algorithms.is_empty()
            || self
                .supported_algorithms
                .iter()
                .any(|supported| supported.eq_ignore_ascii_case(algorithm))
    }

    pub fn supports_rig_kind(&self, rig_kind: RigKind) -> bool {
        self.supported_rig_kinds.is_empty() || self.supported_rig_kinds.contains(&rig_kind)
    }
}

const SUPPORTED_2MINERS: &[&str] = &[
    "Ethash",
    "Etchash",
    "KawPow",
    "Autolykos",
    "BeamHashIII",
    "Cuckatoo32",
    "Cuckaroo29",
    "ProgPowZ",
    "FiroPow",
];
const SUPPORTED_WOOLYPOOLY: &[&str] = &[
    "KawPow",
    "Autolykos",
    "ProgPow",
    "ProgPowZ",
    "FiroPow",
    "FishHash",
    "DynexSolve",
    "Cortex",
    "NexaPow",
    "Xelishashv3",
    "KarlsenHashV2",
    "Qhash",
    "SHA3x",
    "AbelHash",
];
const SUPPORTED_KRYPTEX: &[&str] = &[
    "KawPow",
    "Autolykos",
    "Ethash",
    "Etchash",
    "FishHash",
    "NexaPow",
    "Xelishashv3",
];
const SUPPORTED_CPU: &[&str] = &[
    "RandomX",
    "AstroBWTv3",
    "VerusHash",
    "Ghostrider",
    "Randomscash",
    "CryptoNightTurtle",
    "Xelishashv3",
];
const SUPPORTED_ALT_GPU: &[&str] = &[
    "Ethash",
    "Etchash",
    "Autolykos",
    "KawPow",
    "ProgPow",
    "ProgPowZ",
    "FiroPow",
    "BeamHashIII",
    "Cuckatoo32",
    "Cuckaroo29",
    "CuckooCycle",
    "FishHash",
    "DynexSolve",
    "Xelishashv3",
    "NexaPow",
    "Qhash",
    "SHA3x",
    "AbelHash",
    "KarlsenHashV2",
    "Cortex",
    "Verthash",
];
const SUPPORTED_ASIC: &[&str] = &["SHA256", "Scrypt", "KHeavyHash"];
const SUPPORTED_SOLO: &[&str] = &[];
const RIGS_GPU_OR_ASIC: &[RigKind] = &[RigKind::Gpu, RigKind::Asic];
const RIGS_GPU_ONLY: &[RigKind] = &[RigKind::Gpu];
const RIGS_CPU_ONLY: &[RigKind] = &[RigKind::Cpu];
const RIGS_ASIC_ONLY: &[RigKind] = &[RigKind::Asic];
const RIGS_ALL: &[RigKind] = &[];

pub const METHODS: &[MiningMethod] = &[
    MiningMethod {
        id: "2miners-pplns",
        name: "2Miners PPLNS",
        strategy: MiningStrategy::Pool,
        payout_mode: PayoutMode::Pplns,
        description: "2Miners multi-region PPLNS pool with 1% fee and frequent payouts.",
        pool_fee_rate: 0.010,
        stale_rate_pct: 0.70,
        uptime_pct: 99.5,
        service_fee_usd_day: 0.0,
        score_bias: 0.0,
        source: "2Miners official pool pages",
        supported_algorithms: SUPPORTED_2MINERS,
        supported_rig_kinds: RIGS_GPU_OR_ASIC,
    },
    MiningMethod {
        id: "woolypooly-pplns",
        name: "WoolyPooly",
        strategy: MiningStrategy::Pool,
        payout_mode: PayoutMode::Pplns,
        description: "WoolyPooly PPLNS pool with 0.9% fee, Vardiff, and low-ping global servers.",
        pool_fee_rate: 0.009,
        stale_rate_pct: 0.60,
        uptime_pct: 99.4,
        service_fee_usd_day: 0.0,
        score_bias: 1.5,
        source: "WoolyPooly official pool pages",
        supported_algorithms: SUPPORTED_WOOLYPOOLY,
        supported_rig_kinds: RIGS_GPU_OR_ASIC,
    },
    MiningMethod {
        id: "kryptex-pps",
        name: "Kryptex PPS+",
        strategy: MiningStrategy::Pool,
        payout_mode: PayoutMode::PpsPlus,
        description: "Kryptex Pool PPS+ venue with 1% fee and low payout variance.",
        pool_fee_rate: 0.010,
        stale_rate_pct: 0.50,
        uptime_pct: 99.6,
        service_fee_usd_day: 0.0,
        score_bias: 3.0,
        source: "Kryptex Pool official articles",
        supported_algorithms: SUPPORTED_KRYPTEX,
        supported_rig_kinds: RIGS_GPU_OR_ASIC,
    },
    MiningMethod {
        id: "cpu-pplns",
        name: "CPU Pool PPLNS",
        strategy: MiningStrategy::Pool,
        payout_mode: PayoutMode::Pplns,
        description: "Reference CPU pool envelope built from live Hashrate.no CPU mining pages and common PPLNS fee bands.",
        pool_fee_rate: 0.009,
        stale_rate_pct: 0.85,
        uptime_pct: 99.2,
        service_fee_usd_day: 0.0,
        score_bias: 1.0,
        source: "Hashrate.no CPU mining pages",
        supported_algorithms: SUPPORTED_CPU,
        supported_rig_kinds: RIGS_CPU_ONLY,
    },
    MiningMethod {
        id: "cpu-pps",
        name: "CPU Pool PPS+",
        strategy: MiningStrategy::Pool,
        payout_mode: PayoutMode::PpsPlus,
        description: "Lower-variance CPU payout envelope using PPS-style CPU pool assumptions.",
        pool_fee_rate: 0.012,
        stale_rate_pct: 0.70,
        uptime_pct: 99.3,
        service_fee_usd_day: 0.0,
        score_bias: 2.5,
        source: "Hashrate.no CPU mining pages",
        supported_algorithms: SUPPORTED_CPU,
        supported_rig_kinds: RIGS_CPU_ONLY,
    },
    MiningMethod {
        id: "alt-pplns",
        name: "Alt Pool PPLNS",
        strategy: MiningStrategy::Pool,
        payout_mode: PayoutMode::Pplns,
        description: "Reference niche-GPU pool envelope synthesized from live MiningPoolStats pool rosters.",
        pool_fee_rate: 0.010,
        stale_rate_pct: 0.75,
        uptime_pct: 99.2,
        service_fee_usd_day: 0.0,
        score_bias: 0.8,
        source: "MiningPoolStats pool rosters",
        supported_algorithms: SUPPORTED_ALT_GPU,
        supported_rig_kinds: RIGS_GPU_ONLY,
    },
    MiningMethod {
        id: "asic-fpps",
        name: "ASIC Pool FPPS",
        strategy: MiningStrategy::Pool,
        payout_mode: PayoutMode::PpsPlus,
        description: "Reference ASIC pool envelope for SHA256, Scrypt, and KHeavyHash pools with FPPS-style payouts.",
        pool_fee_rate: 0.022,
        stale_rate_pct: 0.30,
        uptime_pct: 99.7,
        service_fee_usd_day: 0.0,
        score_bias: 3.5,
        source: "MiningPoolStats ASIC pool rosters",
        supported_algorithms: SUPPORTED_ASIC,
        supported_rig_kinds: RIGS_ASIC_ONLY,
    },
    MiningMethod {
        id: "asic-pplns",
        name: "ASIC Pool PPLNS",
        strategy: MiningStrategy::Pool,
        payout_mode: PayoutMode::Pplns,
        description: "Reference ASIC pool PPLNS envelope across large SHA256, Scrypt, and KHeavyHash venues.",
        pool_fee_rate: 0.014,
        stale_rate_pct: 0.35,
        uptime_pct: 99.6,
        service_fee_usd_day: 0.0,
        score_bias: 1.6,
        source: "MiningPoolStats ASIC pool rosters",
        supported_algorithms: SUPPORTED_ASIC,
        supported_rig_kinds: RIGS_ASIC_ONLY,
    },
    MiningMethod {
        id: "2miners-solo",
        name: "2Miners SOLO",
        strategy: MiningStrategy::Solo,
        payout_mode: PayoutMode::SoloPool,
        description: "2Miners solo mode with pool-side routing and 1.5% fee, but full block variance.",
        pool_fee_rate: 0.015,
        stale_rate_pct: 0.70,
        uptime_pct: 99.5,
        service_fee_usd_day: 0.0,
        score_bias: -5.0,
        source: "2Miners official pool pages",
        supported_algorithms: SUPPORTED_2MINERS,
        supported_rig_kinds: RIGS_GPU_OR_ASIC,
    },
    MiningMethod {
        id: "cpu-solo-pool",
        name: "CPU SOLO Pool",
        strategy: MiningStrategy::Solo,
        payout_mode: PayoutMode::SoloPool,
        description: "Reference solo-pool envelope for CPU-first coins with block variance intact.",
        pool_fee_rate: 0.010,
        stale_rate_pct: 0.90,
        uptime_pct: 99.1,
        service_fee_usd_day: 0.0,
        score_bias: -4.0,
        source: "Hashrate.no CPU mining pages",
        supported_algorithms: SUPPORTED_CPU,
        supported_rig_kinds: RIGS_CPU_ONLY,
    },
    MiningMethod {
        id: "asic-solo-pool",
        name: "ASIC SOLO Pool",
        strategy: MiningStrategy::Solo,
        payout_mode: PayoutMode::SoloPool,
        description: "Reference solo-pool envelope for ASIC-first chains where payout variance dominates.",
        pool_fee_rate: 0.010,
        stale_rate_pct: 0.35,
        uptime_pct: 99.6,
        service_fee_usd_day: 0.0,
        score_bias: -3.0,
        source: "MiningPoolStats ASIC pool rosters",
        supported_algorithms: SUPPORTED_ASIC,
        supported_rig_kinds: RIGS_ASIC_ONLY,
    },
    MiningMethod {
        id: "solo-node",
        name: "Solo Node",
        strategy: MiningStrategy::Solo,
        payout_mode: PayoutMode::SoloNode,
        description: "Direct solo mining against your own node, with no pool fee and the highest payout variance.",
        pool_fee_rate: 0.0,
        stale_rate_pct: 1.10,
        uptime_pct: 98.5,
        service_fee_usd_day: 0.03,
        score_bias: -7.0,
        source: "Inference from direct-node mining assumptions",
        supported_algorithms: SUPPORTED_SOLO,
        supported_rig_kinds: RIGS_ALL,
    },
];

#[derive(Debug, Clone, Serialize)]
pub struct MiningCoin {
    pub id: u64,
    pub name: String,
    pub symbol: String,
    pub algorithm: String,
    pub block_time_sec: f64,
    pub block_reward: f64,
    pub blocks_per_day: f64,
    pub daily_emission: f64,
    pub network_hashrate_hs: f64,
    pub exchange_rate_btc: f64,
    pub price_usd: f64,
    pub market_cap_usd: f64,
    pub volume_24h_usd: f64,
    pub profitability: f64,
    pub profitability24: f64,
    pub reference_coin_per_day: f64,
    pub reference_btc_revenue: f64,
    pub reference_hashrate_hs: f64,
    pub price_trend_pct: f64,
    pub difficulty_trend_pct: f64,
    pub volatility: f64,
    pub lagging: bool,
    pub freshness_minutes: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MiningSnapshot {
    pub as_of: String,
    pub source: String,
    pub btc_usd: f64,
    pub coins: Vec<MiningCoin>,
}

impl MiningSnapshot {
    pub fn fetch_live() -> Result<Self, String> {
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        let btc_usd = fetch_coinbase_btc_spot()?;
        let mut sources = Vec::new();
        let mut coins = Vec::new();

        if let Ok(whattomine) = fetch_whattomine_coins(btc_usd, now_secs) {
            if !whattomine.is_empty() {
                sources.push("WhatToMine");
                coins.extend(whattomine);
            }
        }

        let mut seen_symbols = coins
            .iter()
            .map(|coin| coin.symbol.clone())
            .collect::<HashSet<_>>();

        let before_hashrate = coins.len();
        let hashrate_supplemental = fetch_hashrate_cpu_coins(btc_usd).unwrap_or_default();
        for coin in hashrate_supplemental {
            if seen_symbols.insert(coin.symbol.clone()) {
                coins.push(coin);
            }
        }
        if coins.len() > before_hashrate {
            sources.push("Hashrate.no");
        }

        let before_mps = coins.len();
        let miningpoolstats = fetch_miningpoolstats_coins(btc_usd, &seen_symbols).unwrap_or_default();
        for coin in miningpoolstats {
            if seen_symbols.insert(coin.symbol.clone()) {
                coins.push(coin);
            }
        }
        if coins.len() > before_mps {
            sources.push("MiningPoolStats");
        }

        if coins.is_empty() {
            return Err("No live mining coin feeds returned usable tier-one entries".to_string());
        }

        if sources.is_empty() {
            sources.push("Tier-one mining feeds");
        }

        coins.sort_by(|left, right| {
            right
                .profitability
                .partial_cmp(&left.profitability)
                .unwrap_or(Ordering::Equal)
        });

        Ok(Self {
            as_of: iso_timestamp_now(),
            source: format!("{} + Coinbase spot", sources.join(" + ")),
            btc_usd,
            coins,
        })
    }

    pub fn algorithms(&self) -> Vec<String> {
        self.coins
            .iter()
            .map(|coin| coin.algorithm.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

fn fetch_coinbase_btc_spot() -> Result<f64, String> {
    let btc_url = "https://api.coinbase.com/v2/prices/BTC-USD/spot";
    let btc_resp = ureq::get(btc_url)
        .config()
        .timeout_global(Some(Duration::from_secs(6)))
        .build()
        .call()
        .map_err(|err| format!("Coinbase spot request failed: {err}"))?;
    let btc_json: CoinbaseResponse = btc_resp
        .into_body()
        .read_json()
        .map_err(|err| format!("Coinbase JSON parse failed: {err}"))?;

    btc_json
        .data
        .amount
        .parse::<f64>()
        .map_err(|err| format!("Unable to parse BTC spot price: {err}"))
}

fn fetch_whattomine_coins(btc_usd: f64, now_secs: f64) -> Result<Vec<MiningCoin>, String> {
    let coins_url = "https://whattomine.com/coins.json";
    let coins_resp = ureq::get(coins_url)
        .config()
        .timeout_global(Some(Duration::from_secs(6)))
        .build()
        .call()
        .map_err(|err| format!("WhatToMine request failed: {err}"))?;
    let coins_json: WhatToMineResponse = coins_resp
        .into_body()
        .read_json()
        .map_err(|err| format!("WhatToMine JSON parse failed: {err}"))?;

    let mut coins = Vec::new();

    for (name, raw_coin) in coins_json.coins {
        let symbol = string_value(raw_coin.get("tag")).unwrap_or_else(|| name.clone());
        let algorithm = canonicalize_algorithm(
            &string_value(raw_coin.get("algorithm")).unwrap_or_else(|| "Unknown".to_string()),
        );
        let exchange_curr =
            string_value(raw_coin.get("exchange_rate_curr")).unwrap_or_else(|| "BTC".to_string());

        let block_time_sec = number_value(raw_coin.get("block_time"));
        let block_reward = number_value(raw_coin.get("block_reward"));
        let difficulty = number_value(raw_coin.get("difficulty"));
        let difficulty24 = number_value(raw_coin.get("difficulty24"));
        let network_hashrate_hs = number_value(raw_coin.get("nethash"));
        let exchange_rate_btc = number_value(raw_coin.get("exchange_rate"));
        let exchange_rate24 = number_value(raw_coin.get("exchange_rate24"));
        let reference_coin_per_day = number_value(raw_coin.get("estimated_rewards"));
        let reference_btc_revenue = number_value(raw_coin.get("btc_revenue"));

        if block_time_sec <= 0.0
            || block_reward <= 0.0
            || network_hashrate_hs <= 0.0
            || reference_coin_per_day <= 0.0
        {
            continue;
        }

        let blocks_per_day = 86_400.0 / block_time_sec;
        let daily_emission = blocks_per_day * block_reward;
        if daily_emission <= 0.0 {
            continue;
        }

        let reference_hashrate_hs =
            (reference_coin_per_day / daily_emission) * network_hashrate_hs;
        if !reference_hashrate_hs.is_finite() || reference_hashrate_hs <= 0.0 {
            continue;
        }

        let price_usd = if exchange_curr.eq_ignore_ascii_case("USD") {
            exchange_rate_btc
        } else {
            exchange_rate_btc * btc_usd
        };

        let timestamp = number_value(raw_coin.get("timestamp"));
        let freshness_minutes = if timestamp > 0.0 {
            ((now_secs - timestamp).max(0.0)) / 60.0
        } else {
            0.0
        };

        coins.push(MiningCoin {
            id: number_value(raw_coin.get("id")) as u64,
            name,
            symbol: symbol.to_uppercase(),
            algorithm,
            block_time_sec,
            block_reward,
            blocks_per_day,
            daily_emission,
            network_hashrate_hs,
            exchange_rate_btc,
            price_usd,
            market_cap_usd: market_cap_value(raw_coin.get("market_cap")),
            volume_24h_usd: 0.0,
            profitability: number_value(raw_coin.get("profitability")),
            profitability24: number_value(raw_coin.get("profitability24")),
            reference_coin_per_day,
            reference_btc_revenue,
            reference_hashrate_hs,
            price_trend_pct: safe_pct_change(exchange_rate_btc, exchange_rate24),
            difficulty_trend_pct: safe_pct_change(difficulty, difficulty24),
            volatility: number_value(raw_coin.get("exchange_rate_vol")),
            lagging: bool_value(raw_coin.get("lagging")),
            freshness_minutes,
        });
    }

    Ok(coins)
}

fn fetch_miningpoolstats_coins(
    btc_usd: f64,
    existing_symbols: &HashSet<String>,
) -> Result<Vec<MiningCoin>, String> {
    let homepage = ureq::get(MININGPOOLSTATS_HOME_URL)
        .header("User-Agent", MININGPOOLSTATS_BROWSER_UA)
        .config()
        .timeout_global(Some(Duration::from_secs(8)))
        .build()
        .call()
        .map_err(|err| format!("MiningPoolStats homepage request failed: {err}"))?
        .into_body()
        .read_to_string()
        .map_err(|err| format!("MiningPoolStats homepage read failed: {err}"))?;
    let timestamp = extract_miningpoolstats_timestamp(&homepage)
        .ok_or_else(|| "Unable to parse MiningPoolStats dataset timestamp".to_string())?;
    let summary = fetch_miningpoolstats_json(&format!(
        "{MININGPOOLSTATS_DATA_BASE_URL}/coins_data.js?t={timestamp}"
    ))?;
    let data = summary
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| "MiningPoolStats summary did not contain a data array".to_string())?;
    let fallback_timestamp = number_value(summary.get("time"));
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();

    let mut coins = Vec::new();
    let mut seen_symbols = existing_symbols.clone();

    for raw in data {
        let name = string_value(raw.get("name")).unwrap_or_default();
        let symbol = string_value(raw.get("symbol"))
            .unwrap_or_default()
            .trim()
            .to_uppercase();
        let page = string_value(raw.get("page")).unwrap_or_default();
        let algorithm = canonicalize_algorithm(
            &string_value(raw.get("algo")).unwrap_or_else(|| "Unknown".to_string()),
        );
        let network_hashrate_hs = number_value(raw.get("hashrate")).max(number_value(raw.get("ph")));
        let block_time_sec =
            first_positive(&[number_value(raw.get("target")), number_value(raw.get("bt"))]);
        let price_usd = number_value(raw.get("pr"));
        let market_cap_usd = number_value(raw.get("mc"));
        let volume_24h_usd = number_value(raw.get("v24"));
        let pools = number_value(raw.get("pools"));

        if symbol.is_empty()
            || seen_symbols.contains(&symbol)
            || !tier_one_algorithm_supported(&algorithm)
            || pools < 1.0
            || network_hashrate_hs <= 0.0
            || block_time_sec <= 0.0
            || price_usd <= 0.0
            || !miningpoolstats_liquid(market_cap_usd, volume_24h_usd)
        {
            continue;
        }

        let timestamp_secs = first_positive(&[number_value(raw.get("time")), fallback_timestamp]);
        let freshness_minutes = if timestamp_secs > 0.0 {
            ((now_secs - timestamp_secs).max(0.0)) / 60.0
        } else {
            0.0
        };
        let price_trend_pct = {
            let reported = number_value(raw.get("c7d"));
            if reported.abs() > f64::EPSILON {
                reported
            } else {
                series_pct_change(raw.get("p7d"))
            }
        };
        let difficulty_trend_pct = series_pct_change(raw.get("diff7"));
        let volatility = (series_volatility(raw.get("p7d")) * 0.55
            + series_volatility(raw.get("diff7")) * 0.45)
            .max(0.05);

        let mut candidate = MiningPoolStatsCandidate {
            id: synthetic_coin_id(&format!("mps-{symbol}")),
            name,
            symbol: symbol.clone(),
            algorithm,
            page,
            block_time_sec,
            block_reward: 0.0,
            daily_emission: number_value(raw.get("e24")).max(0.0),
            network_hashrate_hs,
            price_usd,
            market_cap_usd,
            volume_24h_usd,
            price_trend_pct,
            difficulty_trend_pct,
            volatility,
            lagging: freshness_minutes > 180.0,
            freshness_minutes,
        };

        if candidate.daily_emission > 0.0 {
            candidate.block_reward =
                candidate.daily_emission / (86_400.0 / candidate.block_time_sec.max(f64::EPSILON));
        }

        if (candidate.daily_emission <= 0.0 || candidate.block_reward <= 0.0) && !candidate.page.is_empty()
        {
            if let Ok(page_json) = fetch_miningpoolstats_json(&format!(
                "{MININGPOOLSTATS_DATA_BASE_URL}/{}.js?t={timestamp}",
                candidate.page
            )) {
                candidate.price_usd = first_positive(&[
                    candidate.price_usd,
                    number_value(page_json.get("price")),
                ]);
                candidate.block_time_sec = first_positive(&[
                    candidate.block_time_sec,
                    number_value(page_json.get("block_time_target")),
                    number_value(page_json.get("block_time_average")),
                ]);
                candidate.daily_emission = first_positive(&[
                    candidate.daily_emission,
                    number_value(page_json.get("supply").and_then(|supply| supply.get("emission24"))),
                ]);
                candidate.block_reward = first_positive(&[
                    candidate.block_reward,
                    number_value(page_json.get("minerstat").and_then(Value::as_array).and_then(|stats| stats.get(2))),
                ]);
                if candidate.daily_emission <= 0.0
                    && candidate.block_reward > 0.0
                    && candidate.block_time_sec > 0.0
                {
                    candidate.daily_emission =
                        candidate.block_reward * (86_400.0 / candidate.block_time_sec);
                }
                if candidate.block_reward <= 0.0
                    && candidate.daily_emission > 0.0
                    && candidate.block_time_sec > 0.0
                {
                    candidate.block_reward =
                        candidate.daily_emission / (86_400.0 / candidate.block_time_sec);
                }
                let page_timestamp = number_value(page_json.get("time"));
                if page_timestamp > 0.0 {
                    candidate.freshness_minutes = ((now_secs - page_timestamp).max(0.0)) / 60.0;
                    candidate.lagging = candidate.freshness_minutes > 180.0;
                }
            }
        }

        if let Some(coin) = build_miningpoolstats_coin(&candidate, btc_usd) {
            seen_symbols.insert(symbol);
            coins.push(coin);
        }
    }

    Ok(coins)
}

fn build_miningpoolstats_coin(
    candidate: &MiningPoolStatsCandidate,
    btc_usd: f64,
) -> Option<MiningCoin> {
    if candidate.block_time_sec <= 0.0
        || candidate.block_reward <= 0.0
        || candidate.daily_emission <= 0.0
        || candidate.network_hashrate_hs <= 0.0
        || candidate.price_usd <= 0.0
    {
        return None;
    }

    let blocks_per_day = 86_400.0 / candidate.block_time_sec.max(f64::EPSILON);
    let reference_hashrate_hs = 1.0;
    let reference_coin_per_day = candidate.daily_emission / candidate.network_hashrate_hs.max(f64::EPSILON);
    let reference_btc_revenue =
        (reference_coin_per_day * candidate.price_usd) / btc_usd.max(f64::EPSILON);
    let profitability = reference_btc_revenue * 1_000_000_000_000.0;

    Some(MiningCoin {
        id: candidate.id,
        name: candidate.name.clone(),
        symbol: candidate.symbol.clone(),
        algorithm: candidate.algorithm.clone(),
        block_time_sec: candidate.block_time_sec,
        block_reward: candidate.block_reward,
        blocks_per_day,
        daily_emission: candidate.daily_emission,
        network_hashrate_hs: candidate.network_hashrate_hs,
        exchange_rate_btc: candidate.price_usd / btc_usd.max(f64::EPSILON),
        price_usd: candidate.price_usd,
        market_cap_usd: candidate.market_cap_usd,
        volume_24h_usd: candidate.volume_24h_usd,
        profitability,
        profitability24: profitability,
        reference_coin_per_day,
        reference_btc_revenue,
        reference_hashrate_hs,
        price_trend_pct: candidate.price_trend_pct,
        difficulty_trend_pct: candidate.difficulty_trend_pct,
        volatility: candidate.volatility,
        lagging: candidate.lagging,
        freshness_minutes: candidate.freshness_minutes,
    })
}

fn miningpoolstats_liquid(market_cap_usd: f64, volume_24h_usd: f64) -> bool {
    market_cap_usd >= MININGPOOLSTATS_MIN_MARKET_CAP_USD
        || volume_24h_usd >= MININGPOOLSTATS_MIN_VOLUME_USD
}

fn tier_one_algorithm_supported(algorithm: &str) -> bool {
    matches!(
        algorithm.to_ascii_lowercase().as_str(),
        "ethash"
            | "etchash"
            | "autolykos"
            | "kawpow"
            | "progpow"
            | "progpowz"
            | "firopow"
            | "beamhashiii"
            | "cuckatoo32"
            | "cuckaroo29"
            | "cuckoocycle"
            | "fishhash"
            | "dynexsolve"
            | "xelishashv3"
            | "nexapow"
            | "qhash"
            | "sha3x"
            | "abelhash"
            | "karlsenhashv2"
            | "cortex"
            | "verthash"
            | "randomx"
            | "astrobwtv3"
            | "verushash"
            | "ghostrider"
            | "randomscash"
            | "cryptonightturtle"
            | "sha256"
            | "scrypt"
            | "kheavyhash"
    )
}

fn fetch_miningpoolstats_json(url: &str) -> Result<Value, String> {
    let response = ureq::get(url)
        .header("User-Agent", MININGPOOLSTATS_BROWSER_UA)
        .header("Referer", MININGPOOLSTATS_HOME_URL)
        .header("Origin", "https://miningpoolstats.stream")
        .config()
        .timeout_global(Some(Duration::from_secs(8)))
        .build()
        .call()
        .map_err(|err| format!("{url} request failed: {err}"))?;

    response
        .into_body()
        .read_json()
        .map_err(|err| format!("{url} JSON parse failed: {err}"))
}

fn extract_miningpoolstats_timestamp(page: &str) -> Option<u64> {
    miningpoolstats_timestamp_regex()
        .captures(page)
        .and_then(|captures| captures.get(1))
        .and_then(|value| value.as_str().parse::<u64>().ok())
}

fn miningpoolstats_timestamp_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"coins_data\.js\?t=(\d+)").expect("MiningPoolStats timestamp regex should compile")
    })
}

fn first_positive(values: &[f64]) -> f64 {
    values
        .iter()
        .copied()
        .find(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(0.0)
}

fn series_pct_change(value: Option<&Value>) -> f64 {
    let history = valid_history_values(value);
    let Some(first) = history.first().copied() else {
        return 0.0;
    };
    let Some(last) = history.last().copied() else {
        return 0.0;
    };
    safe_pct_change(last, first)
}

fn series_volatility(value: Option<&Value>) -> f64 {
    let history = valid_history_values(value);
    if history.len() < 2 {
        return 0.0;
    }

    let avg_abs_step = history
        .windows(2)
        .map(|pair| {
            if pair[0].abs() < f64::EPSILON {
                0.0
            } else {
                ((pair[1] - pair[0]) / pair[0]).abs() * 100.0
            }
        })
        .sum::<f64>()
        / (history.len() - 1) as f64;

    avg_abs_step
}

fn valid_history_values(value: Option<&Value>) -> Vec<f64> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let parsed = number_value(Some(item));
                    if parsed.is_finite() && parsed > 0.0 {
                        Some(parsed)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Debug, Clone, Serialize)]
pub struct MiningRow {
    pub coin: MiningCoin,
    pub method: MiningMethod,
    pub score: f64,
    pub fit_level: FitLevel,
    pub rig_name: String,
    pub rig_kind: RigKind,
    pub rig_vendor: String,
    pub rig_memory_gb: Option<f64>,
    pub benchmark_source: String,
    pub benchmark_miner: String,
    pub benchmark_tuning: String,
    pub benchmark_hashrate_hs: f64,
    pub benchmark_power_watts: f64,
    pub reject_rate_pct: f64,
    pub hashrate_hs: f64,
    pub gross_coin_day: f64,
    pub gross_btc_day: f64,
    pub gross_usd_day: f64,
    pub power_plan_label: String,
    pub power_breakdown: String,
    pub effective_power_rate_usd_kwh: f64,
    pub power_cost_usd_day: f64,
    pub power_cost_usd_month: f64,
    pub fee_cost_usd_day: f64,
    pub stale_cost_usd_day: f64,
    pub service_cost_usd_day: f64,
    pub net_usd_day: f64,
    pub blocks_day: f64,
    pub blocks_month: f64,
    pub trend_delta_pct: f64,
    pub profit_score: f64,
    pub liquidity_score: f64,
    pub trend_score: f64,
    pub stability_score: f64,
    pub efficiency_score: f64,
    pub variance_score: f64,
    pub variance_zero_block_pct: f64,
    pub variance_p50_usd_month: f64,
    pub variance_p90_usd_month: f64,
    pub eligible: bool,
    pub eligibility_note: String,
}

impl MiningRow {
    pub fn fit_text(&self) -> &'static str {
        self.fit_level.label()
    }

    pub fn strategy_text(&self) -> &'static str {
        self.method.strategy.label()
    }

    pub fn reason_lines(&self) -> Vec<String> {
        let mut reasons = Vec::new();

        if !self.eligible {
            reasons.push(self.eligibility_note.clone());
            reasons.push(
                "Score was capped because the selected rig and algorithm are not a clean operational fit."
                    .to_string(),
            );
            return reasons;
        }

        if self.net_usd_day >= 0.0 {
            reasons.push(format!(
                "${:.2}/day net after power, fee, stale-share, and service drag.",
                self.net_usd_day
            ));
        } else {
            reasons.push(format!(
                "${:.2}/day below breakeven once real-world drag is applied.",
                self.net_usd_day.abs()
            ));
        }

        reasons.push(format!(
            "{} benchmark: {} via {} at {:.2}% expected rejects.",
            self.rig_name, self.benchmark_tuning, self.benchmark_miner, self.reject_rate_pct
        ));
        reasons.push(self.eligibility_note.clone());

        reasons.push(format!(
            "{} payout mode with {:.2}% fee, {:.2}% stale-share, and {:.1}% uptime.",
            self.method.payout_mode.label(),
            self.method.pool_fee_rate * 100.0,
            self.method.stale_rate_pct,
            self.method.uptime_pct
        ));

        if matches!(self.method.strategy, MiningStrategy::Solo) {
            reasons.push(format!(
                "Solo variance is high: {:.1}% chance of zero blocks in 30 days, p50 monthly net ${:.2}, p90 ${:.2}.",
                self.variance_zero_block_pct,
                self.variance_p50_usd_month,
                self.variance_p90_usd_month
            ));
        } else {
            reasons.push(format!(
                "Pooled payouts stay smoother here; p50 monthly net stays near ${:.2}.",
                self.variance_p50_usd_month
            ));
        }

        reasons.push(self.power_breakdown.clone());

        if self.coin.lagging {
            reasons.push(
                "Upstream marked the coin feed as lagging, so treat this row as directional.".to_string(),
            );
        }

        reasons
    }
}

pub fn build_rankings(
    snapshot: &MiningSnapshot,
    power: &PowerContext,
    rig: &MiningRigProfile,
    rig_scale: f64,
) -> Vec<MiningRow> {
    build_rankings_for_rigs(snapshot, power, std::slice::from_ref(rig), rig_scale)
}

pub fn build_rankings_for_rigs(
    snapshot: &MiningSnapshot,
    power: &PowerContext,
    rigs: &[MiningRigProfile],
    rig_scale: f64,
) -> Vec<MiningRow> {
    let mut draft_rows = Vec::new();

    for rig in rigs {
        for coin in &snapshot.coins {
            let rule = algorithm_rule(&coin.algorithm);
            let benchmark_opt = rig.benchmark_for(&coin.algorithm);

            for method in METHODS {
                if !method.supports_algorithm(&coin.algorithm)
                    || !method.supports_rig_kind(rig.kind)
                {
                    continue;
                }

                let (
                    eligible,
                    eligibility_note,
                    benchmark_hashrate_hs,
                    benchmark_power_watts,
                    reject_rate_pct,
                    benchmark_miner,
                    benchmark_tuning,
                ) = resolve_benchmark(coin, rig, &rule, benchmark_opt);

                if !eligible {
                    continue;
                }

                let scaled_hashrate_hs = benchmark_hashrate_hs * rig_scale;
                let scaling_ratio =
                    scaled_hashrate_hs / coin.reference_hashrate_hs.max(f64::EPSILON);
                let gross_coin_day_raw = coin.reference_coin_per_day * scaling_ratio;
                let gross_btc_day_raw = coin.reference_btc_revenue * scaling_ratio;
                let gross_usd_day_raw = gross_btc_day_raw * snapshot.btc_usd;

                let efficiency_multiplier = (method.uptime_pct / 100.0)
                    * (1.0 - method.stale_rate_pct / 100.0)
                    * (1.0 - reject_rate_pct / 100.0);
                let mined_coin_day = gross_coin_day_raw * efficiency_multiplier;
                let mined_btc_day = gross_btc_day_raw * efficiency_multiplier;
                let mined_usd_day = gross_usd_day_raw * efficiency_multiplier;
                let liquidity_factor = liquidity_realization_factor(
                    mined_usd_day,
                    coin.market_cap_usd,
                    coin.volume_24h_usd,
                );
                let gross_coin_day = mined_coin_day * liquidity_factor;
                let gross_btc_day = mined_btc_day * liquidity_factor;
                let gross_usd_day = mined_usd_day * liquidity_factor;

                let power_estimate = power.estimate_cost(benchmark_power_watts * rig_scale, 30.0);
                let fee_cost_usd_day = gross_usd_day * method.pool_fee_rate;
                let stale_cost_usd_day =
                    gross_usd_day_raw * (method.stale_rate_pct / 100.0) * liquidity_factor;
                let service_cost_usd_day = method.service_fee_usd_day * rig_scale;
                let net_usd_day = gross_usd_day
                    - power_estimate.daily_cost_usd
                    - fee_cost_usd_day
                    - service_cost_usd_day;
                let blocks_day = (scaled_hashrate_hs / coin.network_hashrate_hs.max(f64::EPSILON))
                    * coin.blocks_per_day
                    * efficiency_multiplier;
                let blocks_month = blocks_day * 30.0;
                let (
                    variance_zero_block_pct,
                    variance_p50_usd_month,
                    variance_p90_usd_month,
                ) = payout_variance(
                    method,
                    coin,
                    blocks_month,
                    net_usd_day,
                    gross_usd_day,
                    power_estimate.monthly_cost_usd,
                    fee_cost_usd_day * 30.0,
                );

                draft_rows.push(MiningRow {
                    coin: coin.clone(),
                    method: method.clone(),
                    score: 0.0,
                    fit_level: FitLevel::Avoid,
                    rig_name: rig.name.to_string(),
                    rig_kind: rig.kind,
                    rig_vendor: rig.vendor.to_string(),
                    rig_memory_gb: rig.memory_gb,
                    benchmark_source: rig.source.to_string(),
                    benchmark_miner: benchmark_miner.to_string(),
                    benchmark_tuning: benchmark_tuning.to_string(),
                    benchmark_hashrate_hs,
                    benchmark_power_watts,
                    reject_rate_pct,
                    hashrate_hs: scaled_hashrate_hs,
                    gross_coin_day,
                    gross_btc_day,
                    gross_usd_day,
                    power_plan_label: power.plan_label.clone(),
                    power_breakdown: power_estimate.breakdown,
                    effective_power_rate_usd_kwh: power_estimate.effective_rate_usd_kwh,
                    power_cost_usd_day: power_estimate.daily_cost_usd,
                    power_cost_usd_month: power_estimate.monthly_cost_usd,
                    fee_cost_usd_day,
                    stale_cost_usd_day,
                    service_cost_usd_day,
                    net_usd_day,
                    blocks_day,
                    blocks_month,
                    trend_delta_pct: coin.price_trend_pct - coin.difficulty_trend_pct,
                    profit_score: 0.0,
                    liquidity_score: 0.0,
                    trend_score: 0.0,
                    stability_score: 0.0,
                    efficiency_score: 0.0,
                    variance_score: 0.0,
                    variance_zero_block_pct,
                    variance_p50_usd_month,
                    variance_p90_usd_month,
                    eligible,
                    eligibility_note,
                });
            }
        }
    }

    score_rows(&mut draft_rows);
    draft_rows
}

pub fn sort_rankings(rows: &mut [MiningRow], column: SortColumn, ascending: bool) {
    rows.sort_by(|left, right| {
        let ordering = match column {
            SortColumn::Score => cmp_f64(left.score, right.score),
            SortColumn::NetUsd => cmp_f64(left.net_usd_day, right.net_usd_day),
            SortColumn::GrossUsd => cmp_f64(left.gross_usd_day, right.gross_usd_day),
            SortColumn::BlocksPerMonth => cmp_f64(left.blocks_month, right.blocks_month),
            SortColumn::Trend => cmp_f64(left.trend_delta_pct, right.trend_delta_pct),
            SortColumn::MarketCap => {
                cmp_f64(liquidity_usd(&left.coin), liquidity_usd(&right.coin))
            }
        };

        if ascending {
            ordering
        } else {
            ordering.reverse()
        }
    });
}

fn score_rows(rows: &mut [MiningRow]) {
    let net_min = rows.iter().map(|row| row.net_usd_day).fold(0.0, f64::min);
    let net_max = rows.iter().map(|row| row.net_usd_day).fold(0.0, f64::max);
    let efficiency_min = rows.iter().map(usd_per_watt_day).fold(0.0, f64::min);
    let efficiency_max = rows.iter().map(usd_per_watt_day).fold(0.0, f64::max);

    for row in rows {
        row.profit_score = normalize(row.net_usd_day, net_min, net_max);
        row.liquidity_score = liquidity_score(row.coin.market_cap_usd, row.coin.volume_24h_usd);
        row.trend_score = trend_score(
            row.coin.price_trend_pct,
            row.coin.difficulty_trend_pct,
            row.coin.volatility,
            row.coin.lagging,
        );
        row.stability_score = stability_score(row);
        row.efficiency_score = normalize(usd_per_watt_day(row), efficiency_min, efficiency_max);
        row.variance_score = variance_score(row);
        row.score = if row.eligible {
            clamp(
                row.profit_score * 0.34
                    + row.liquidity_score * 0.14
                    + row.trend_score * 0.12
                    + row.stability_score * 0.12
                    + row.efficiency_score * 0.16
                    + row.variance_score * 0.12
                    + row.method.score_bias,
                0.0,
                100.0,
            )
        } else {
            clamp(
                row.profit_score * 0.15 + row.liquidity_score * 0.10 + row.method.score_bias,
                0.0,
                20.0,
            )
        };
        row.fit_level = score_to_fit(row.score, row.net_usd_day, row.eligible);
    }
}

fn resolve_benchmark(
    coin: &MiningCoin,
    rig: &MiningRigProfile,
    rule: &AlgorithmRule,
    benchmark_opt: Option<&crate::rig_profiles::AlgorithmBenchmark>,
) -> (bool, String, f64, f64, f64, &'static str, &'static str) {
    let vendor = rig.vendor.to_ascii_lowercase();
    let vendor_ok = match rig.kind {
        RigKind::Asic => rule.supports_asic,
        RigKind::Cpu => rule.supports_cpu,
        RigKind::Gpu => {
            if vendor.contains("nvidia") {
                rule.supports_nvidia
            } else if vendor.contains("amd") {
                rule.supports_amd
            } else {
                rule.supports_nvidia || rule.supports_amd
            }
        }
    };

    let memory_ok = match rig.kind {
        RigKind::Gpu => rig.memory_gb.unwrap_or(rule.min_vram_gb) + 0.01 >= rule.min_vram_gb,
        RigKind::Asic | RigKind::Cpu => true,
    };

    if !vendor_ok {
        return (
            false,
            format!(
                "{} is not a clean backend fit for {}. {} {}",
                rig.name, coin.algorithm, rule.tuning_note, rule.backend_note
            ),
            0.0,
            rig.fallback_power_watts,
            0.0,
            "n/a",
            "unsupported backend",
        );
    }

    if !memory_ok {
        return (
            false,
            format!(
                "{} needs about {:.1} GB VRAM for {}, but the selected rig only exposes {:.1} GB. {}",
                coin.algorithm,
                rule.min_vram_gb,
                coin.symbol,
                rig.memory_gb.unwrap_or_default(),
                rule.backend_note
            ),
            0.0,
            rig.fallback_power_watts,
            0.0,
            "n/a",
            "insufficient VRAM",
        );
    }

    if let Some(benchmark) = benchmark_opt {
        return (
            true,
            format!(
                "Operational note: {} on {}. {}",
                benchmark.miner, rig.name, rule.backend_note
            ),
            benchmark.hashrate_hs,
            benchmark.power_watts,
            benchmark.reject_rate_pct,
            benchmark.miner,
            benchmark.tuning,
        );
    }

    if rig.id == "generic-gpu" && matches!(rig.kind, RigKind::Gpu) {
        return (
            true,
            format!(
                "Using WhatToMine's reference-rig fallback for {} because no curated benchmark matched {}. {}",
                coin.algorithm, rig.name, rule.backend_note
            ),
            coin.reference_hashrate_hs,
            rig.fallback_power_watts,
            1.0,
            "Reference fallback",
            "No exact benchmark match",
        );
    }

    (
        false,
        format!(
            "No curated {} benchmark is available for {} yet. {} {}",
            coin.algorithm, rig.name, rule.tuning_note, rule.backend_note
        ),
        0.0,
        rig.fallback_power_watts,
        0.0,
        "n/a",
        "no benchmark",
    )
}

fn payout_variance(
    method: &MiningMethod,
    coin: &MiningCoin,
    blocks_month: f64,
    net_usd_day: f64,
    _gross_usd_day: f64,
    power_cost_usd_month: f64,
    fee_cost_usd_month: f64,
) -> (f64, f64, f64) {
    let expected_monthly_net = net_usd_day * 30.0;
    match method.payout_mode {
        PayoutMode::SoloPool | PayoutMode::SoloNode => {
            let lambda = blocks_month.max(0.0);
            let zero = (-lambda).exp();
            let median_blocks = poisson_quantile(lambda, 0.50);
            let high_blocks = poisson_quantile(lambda, 0.90);
            let block_value_usd = coin.block_reward * coin.price_usd;
            let p50 = median_blocks * block_value_usd - power_cost_usd_month - fee_cost_usd_month;
            let p90 = high_blocks * block_value_usd - power_cost_usd_month - fee_cost_usd_month;
            (zero * 100.0, p50, p90)
        }
        PayoutMode::Pplns => (
            0.0,
            expected_monthly_net * 0.97,
            expected_monthly_net * 1.05,
        ),
        PayoutMode::PpsPlus => (
            0.0,
            expected_monthly_net * 0.995,
            expected_monthly_net * 1.02,
        ),
    }
}

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}

fn normalize(value: f64, min: f64, max: f64) -> f64 {
    if (max - min).abs() < f64::EPSILON {
        return 50.0;
    }

    clamp(((value - min) / (max - min)) * 100.0, 0.0, 100.0)
}

fn liquidity_usd(coin: &MiningCoin) -> f64 {
    if coin.market_cap_usd > 0.0 {
        coin.market_cap_usd
    } else {
        coin.volume_24h_usd
    }
}

fn liquidity_realization_factor(
    gross_usd_day: f64,
    market_cap_usd: f64,
    volume_24h_usd: f64,
) -> f64 {
    if gross_usd_day <= 0.0 {
        return 1.0;
    }

    let volume_cap = if volume_24h_usd > 0.0 {
        volume_24h_usd * 0.03
    } else {
        f64::INFINITY
    };
    let market_cap_cap = if market_cap_usd > 0.0 {
        market_cap_usd * 0.0002
    } else {
        f64::INFINITY
    };
    let realization_cap = volume_cap.min(market_cap_cap);
    if !realization_cap.is_finite() {
        return 1.0;
    }

    clamp(realization_cap / gross_usd_day, 0.0, 1.0)
}

fn liquidity_score(market_cap_usd: f64, volume_24h_usd: f64) -> f64 {
    let market_score = if market_cap_usd > 0.0 {
        clamp(((market_cap_usd.log10() - 5.0) / 5.0) * 100.0, 0.0, 100.0)
    } else {
        0.0
    };
    let volume_score = if volume_24h_usd > 0.0 {
        clamp(((volume_24h_usd.log10() - 3.0) / 5.0) * 100.0, 0.0, 100.0)
    } else {
        0.0
    };

    if market_cap_usd > 0.0 {
        market_score.max(volume_score)
    } else {
        volume_score * 0.9
    }
}

fn trend_score(
    price_trend_pct: f64,
    difficulty_trend_pct: f64,
    volatility: f64,
    lagging: bool,
) -> f64 {
    let volatility_penalty = (1.0 + volatility.max(0.0)).ln() * 10.0;
    let raw = 58.0
        + price_trend_pct * 3.3
        - difficulty_trend_pct * 2.4
        - volatility_penalty
        - if lagging { 8.0 } else { 0.0 };
    clamp(raw, 0.0, 100.0)
}

fn stability_score(row: &MiningRow) -> f64 {
    if !row.eligible {
        return 0.0;
    }

    let volatility_penalty = (1.0 + row.coin.volatility.max(0.0)).ln() * 8.0;
    let pool_quality = row.method.uptime_pct - row.method.stale_rate_pct * 3.0 - row.reject_rate_pct * 2.0;

    match row.method.payout_mode {
        PayoutMode::SoloPool | PayoutMode::SoloNode => {
            let base = 42.0 + row.blocks_month.min(5.0) * 9.0 - row.variance_zero_block_pct * 0.35;
            clamp(base - volatility_penalty, 0.0, 100.0)
        }
        PayoutMode::Pplns => clamp(pool_quality - volatility_penalty, 0.0, 100.0),
        PayoutMode::PpsPlus => clamp(pool_quality + 6.0 - volatility_penalty * 0.8, 0.0, 100.0),
    }
}

fn variance_score(row: &MiningRow) -> f64 {
    if !row.eligible {
        return 0.0;
    }

    match row.method.payout_mode {
        PayoutMode::PpsPlus => 92.0,
        PayoutMode::Pplns => 78.0,
        PayoutMode::SoloPool | PayoutMode::SoloNode => clamp(100.0 - row.variance_zero_block_pct, 0.0, 100.0),
    }
}

fn usd_per_watt_day(row: &MiningRow) -> f64 {
    if row.benchmark_power_watts <= 0.0 {
        return 0.0;
    }
    row.gross_usd_day / row.benchmark_power_watts.max(f64::EPSILON)
}

fn score_to_fit(score: f64, net_usd_day: f64, eligible: bool) -> FitLevel {
    if !eligible {
        return FitLevel::Avoid;
    }

    if net_usd_day <= 0.0 && score < 55.0 {
        return FitLevel::Avoid;
    }

    if score >= 84.0 {
        FitLevel::Prime
    } else if score >= 72.0 {
        FitLevel::Strong
    } else if score >= 60.0 {
        FitLevel::Watch
    } else if score >= 46.0 {
        FitLevel::Speculative
    } else {
        FitLevel::Avoid
    }
}

fn poisson_quantile(lambda: f64, quantile: f64) -> f64 {
    if lambda <= 0.0 {
        return 0.0;
    }

    let mut cumulative = 0.0;
    let mut term = (-lambda).exp();
    let mut k = 0u32;
    cumulative += term;

    while cumulative < quantile && k < 256 {
        k += 1;
        term *= lambda / k as f64;
        cumulative += term;
    }

    k as f64
}

fn cmp_f64(left: f64, right: f64) -> Ordering {
    left.partial_cmp(&right).unwrap_or(Ordering::Equal)
}

fn number_value(value: Option<&Value>) -> f64 {
    match value {
        Some(Value::Number(number)) => number.as_f64().unwrap_or(0.0),
        Some(Value::String(text)) => text.parse::<f64>().unwrap_or(0.0),
        Some(Value::Bool(boolean)) => {
            if *boolean {
                1.0
            } else {
                0.0
            }
        }
        _ => 0.0,
    }
}

fn string_value(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::String(text)) => Some(text.trim().to_string()),
        Some(Value::Number(number)) => Some(number.to_string()),
        Some(Value::Bool(boolean)) => Some(boolean.to_string()),
        _ => None,
    }
}

fn bool_value(value: Option<&Value>) -> bool {
    match value {
        Some(Value::Bool(boolean)) => *boolean,
        Some(Value::String(text)) => matches!(text.as_str(), "true" | "True" | "1"),
        Some(Value::Number(number)) => number.as_i64().unwrap_or_default() != 0,
        _ => false,
    }
}

fn market_cap_value(value: Option<&Value>) -> f64 {
    let Some(raw) = string_value(value) else {
        return 0.0;
    };

    raw.replace(['$', ','], "").parse::<f64>().unwrap_or(0.0)
}

fn safe_pct_change(current: f64, previous: f64) -> f64 {
    if previous.abs() < f64::EPSILON {
        0.0
    } else {
        ((current - previous) / previous) * 100.0
    }
}

fn iso_timestamp_now() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

fn fetch_hashrate_cpu_coins(btc_usd: f64) -> Result<Vec<MiningCoin>, String> {
    let mut coins = Vec::new();

    for symbol in HASHRATE_CPU_SYMBOLS {
        let Ok(html) = fetch_text(&format!("https://www.hashrate.no/coins/{symbol}/")) else {
            continue;
        };
        let Ok(parsed) = parse_hashrate_coin_page(&html) else {
            continue;
        };

        if !cpu_algorithm_supported(&parsed.algorithm)
            || parsed.block_reward <= 0.0
            || parsed.block_time_sec <= 0.0
            || parsed.network_hashrate_hs <= 0.0
            || parsed.price_usd <= 0.0
            || parsed.volume_24h_usd < HASHRATE_TIER_ONE_MIN_VOLUME_USD
        {
            continue;
        }

        let blocks_per_day = 86_400.0 / parsed.block_time_sec.max(f64::EPSILON);
        let daily_emission = blocks_per_day * parsed.block_reward;
        let reference_hashrate_hs = 1.0;
        let reference_coin_per_day = daily_emission / parsed.network_hashrate_hs.max(f64::EPSILON);
        let reference_btc_revenue =
            (reference_coin_per_day * parsed.price_usd) / btc_usd.max(f64::EPSILON);
        let profitability = reference_btc_revenue * 1_000_000_000_000.0;

        coins.push(MiningCoin {
            id: synthetic_coin_id(&parsed.symbol),
            name: parsed.name,
            symbol: parsed.symbol,
            algorithm: parsed.algorithm,
            block_time_sec: parsed.block_time_sec,
            block_reward: parsed.block_reward,
            blocks_per_day,
            daily_emission,
            network_hashrate_hs: parsed.network_hashrate_hs,
            exchange_rate_btc: parsed.price_usd / btc_usd.max(f64::EPSILON),
            price_usd: parsed.price_usd,
            market_cap_usd: 0.0,
            volume_24h_usd: parsed.volume_24h_usd,
            profitability,
            profitability24: profitability,
            reference_coin_per_day,
            reference_btc_revenue,
            reference_hashrate_hs,
            price_trend_pct: parsed.price_trend_pct,
            difficulty_trend_pct: parsed.difficulty_trend_pct,
            volatility: ((parsed.price_trend_pct.abs() + parsed.difficulty_trend_pct.abs()) / 10.0)
                .max(0.05),
            lagging: parsed.freshness_minutes > 180.0,
            freshness_minutes: parsed.freshness_minutes,
        });
    }

    Ok(coins)
}

fn parse_hashrate_coin_page(html: &str) -> Result<HashrateCoinPage, String> {
    let title_caps = title_regex()
        .captures(html)
        .ok_or_else(|| "Hashrate.no title parse failed".to_string())?;
    let info_caps = coin_info_regex()
        .captures(html)
        .ok_or_else(|| "Hashrate.no info table parse failed".to_string())?;
    let price_caps = price_regex()
        .captures(html)
        .ok_or_else(|| "Hashrate.no price parse failed".to_string())?;
    let hashrate_caps = hashrate_regex()
        .captures(html)
        .ok_or_else(|| "Hashrate.no hashrate parse failed".to_string())?;

    let history = parse_history_points(html);
    let (difficulty_trend_pct, freshness_minutes) = if history.len() >= 2 {
        let latest = history.last().cloned().unwrap_or_default();
        let previous = history
            .iter()
            .rev()
            .nth(1)
            .cloned()
            .unwrap_or_else(|| latest.clone());
        (
            safe_pct_change(
                latest.difficulty.parse::<f64>().unwrap_or(0.0),
                previous.difficulty.parse::<f64>().unwrap_or(0.0),
            ),
            history_freshness_minutes(latest.time),
        )
    } else {
        (0.0, 0.0)
    };

    Ok(HashrateCoinPage {
        name: title_caps["name"].trim().to_string(),
        symbol: title_caps["symbol"].trim().to_uppercase(),
        algorithm: canonicalize_algorithm(info_caps["algorithm"].trim()),
        block_reward: parse_prefixed_number(&info_caps["block_reward"]),
        block_time_sec: parse_prefixed_number(&info_caps["block_time"]),
        network_hashrate_hs: parse_hashrate_value(&hashrate_caps["hashrate"]),
        price_usd: parse_plain_number(&price_caps["price"]),
        price_trend_pct: parse_percent(&price_caps["delta"]),
        volume_24h_usd: parse_plain_number(&info_caps["volume"]),
        difficulty_trend_pct,
        freshness_minutes,
    })
}

fn fetch_text(url: &str) -> Result<String, String> {
    let response = ureq::get(url)
        .config()
        .timeout_global(Some(Duration::from_secs(4)))
        .build()
        .call()
        .map_err(|err| format!("{url} request failed: {err}"))?;

    response
        .into_body()
        .read_to_string()
        .map_err(|err| format!("{url} body read failed: {err}"))
}

fn title_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"<title>(?P<name>.+?) \((?P<symbol>[^)]+)\) - Hashrate</title>")
            .expect("title regex should compile")
    })
}

fn coin_info_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?s)<table style='width: 100%'><tr><td style='width: 50%'>Algorithm</td><td style='width: 50%'>(?P<algorithm>[^<]+)</td></tr><tr><td>Block Reward</td><td>(?P<block_reward>[^<]+)</td></tr>(?:<tr><td>Block Reward[^<]*</td><td>[^<]+</td></tr>)*<tr><td>Block Value</td><td>\$(?P<block_value>[^<]+)</td></tr><tr><td>Block Time</td><td>(?P<block_time>[^<]+)</td></tr><tr><td>Emission</td><td>(?P<emission>[^<]+)<br />\$(?P<emission_usd>[^<]+)</td></tr><tr><td>Volume 24h</td><td>\$(?P<volume>[^<]+)</td></tr>",
        )
        .expect("coin info regex should compile")
    })
}

fn price_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?s)<div class='optionHeader'>Price</div><div class='optionSelectContainer'><div class='w3-right inStock'>[^<]+</div><div class='w3-rest'>\$(?P<price>[^ <]+)\s*<span style='color: [^']+'>(?P<delta>[^<]+)</span>",
        )
        .expect("price regex should compile")
    })
}

fn hashrate_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?s)<div class='optionHeader'>Hashrate</div><div class='optionSelectContainer'><div class='w3-rest'>(?P<hashrate>[^<]+)</div>",
        )
        .expect("hashrate regex should compile")
    })
}

fn history_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"const data = (?P<data>\[[^;]+\]);").expect("history regex should compile")
    })
}

fn parse_history_points(html: &str) -> Vec<HashrateHistoryPoint> {
    let Some(captures) = history_regex().captures(html) else {
        return Vec::new();
    };
    let Some(raw) = captures.name("data") else {
        return Vec::new();
    };

    serde_json::from_str(raw.as_str()).unwrap_or_default()
}

fn history_freshness_minutes(timestamp_ms: u64) -> f64 {
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();
    ((now_secs - (timestamp_ms as f64 / 1000.0)).max(0.0)) / 60.0
}

fn synthetic_coin_id(symbol: &str) -> u64 {
    symbol
        .bytes()
        .fold(7_000_000_000u64, |acc, byte| acc.wrapping_mul(131).wrapping_add(byte as u64))
}

fn cpu_algorithm_supported(algorithm: &str) -> bool {
    matches!(
        algorithm.to_ascii_lowercase().as_str(),
        "randomx"
            | "astrobwtv3"
            | "verushash"
            | "ghostrider"
            | "randomscash"
            | "cryptonightturtle"
            | "xelishashv3"
    )
}

fn canonicalize_algorithm(value: &str) -> String {
    match value.to_ascii_lowercase().as_str() {
        "ethash" => "Ethash".to_string(),
        "etchash" => "Etchash".to_string(),
        "autolykos" | "autolykos 2" | "autolykos2" => "Autolykos".to_string(),
        "kawpow" => "KawPow".to_string(),
        "progpow" => "ProgPow".to_string(),
        "evrprogpow" => "ProgPow".to_string(),
        "progpowz" | "progpow-z" => "ProgPowZ".to_string(),
        "firopow" | "firo pow" => "FiroPow".to_string(),
        "beamhashiii" | "beamhash iii" => "BeamHashIII".to_string(),
        "cuckatoo32" | "cuckatoo 32" => "Cuckatoo32".to_string(),
        "cuckaroo" | "cuckaroo29" | "cuckaroo 29" => "Cuckaroo29".to_string(),
        "cuckoo cycle" | "cuckoocycle" => "CuckooCycle".to_string(),
        "fishhash" => "FishHash".to_string(),
        "dynexsolve" | "dynex" => "DynexSolve".to_string(),
        "randomx" => "RandomX".to_string(),
        "astrobwtv3" | "astrobwt/v3" => "AstroBWTv3".to_string(),
        "verushash" => "VerusHash".to_string(),
        "ghostrider" => "Ghostrider".to_string(),
        "randomscash" => "Randomscash".to_string(),
        "cryptonight turtle" | "cryptonightturtle" => "CryptoNightTurtle".to_string(),
        "xelishashv3" | "xelis hash v3" => "Xelishashv3".to_string(),
        "nexapow" => "NexaPow".to_string(),
        "qhash" => "Qhash".to_string(),
        "sha-3x" | "sha3x" => "SHA3x".to_string(),
        "abelhash" | "abel hash" => "AbelHash".to_string(),
        "karlsenhashv2" | "karlsen hash v2" => "KarlsenHashV2".to_string(),
        "cortex" => "Cortex".to_string(),
        "verthash" => "Verthash".to_string(),
        "sha-256" | "sha256" | "sha-256 m" | "merged sha-256" => "SHA256".to_string(),
        "scrypt" => "Scrypt".to_string(),
        "kheavyhash" | "k-heavyhash" => "KHeavyHash".to_string(),
        _ => value.trim().to_string(),
    }
}

fn parse_prefixed_number(value: &str) -> f64 {
    let text = value.trim().replace(',', "");
    let number = text
        .chars()
        .take_while(|ch| ch.is_ascii_digit() || matches!(ch, '.' | '-'))
        .collect::<String>();
    number.parse::<f64>().unwrap_or(0.0)
}

fn parse_plain_number(value: &str) -> f64 {
    value.trim().replace(',', "").parse::<f64>().unwrap_or(0.0)
}

fn parse_percent(value: &str) -> f64 {
    value.trim().trim_end_matches('%').parse::<f64>().unwrap_or(0.0)
}

fn parse_hashrate_value(value: &str) -> f64 {
    let mut parts = value.split_whitespace();
    let amount = parts.next().map(parse_plain_number).unwrap_or_default();
    let unit = parts.next().unwrap_or("H/s").to_ascii_lowercase();

    let multiplier = match unit.as_str() {
        "h/s" => 1.0,
        "kh/s" => 1_000.0,
        "mh/s" => 1_000_000.0,
        "gh/s" => 1_000_000_000.0,
        "th/s" => 1_000_000_000_000.0,
        "ph/s" => 1_000_000_000_000_000.0,
        "eh/s" => 1_000_000_000_000_000_000.0,
        _ => 1.0,
    };

    amount * multiplier
}

#[derive(Debug, Clone, Default)]
struct HashrateCoinPage {
    name: String,
    symbol: String,
    algorithm: String,
    block_reward: f64,
    block_time_sec: f64,
    network_hashrate_hs: f64,
    price_usd: f64,
    price_trend_pct: f64,
    volume_24h_usd: f64,
    difficulty_trend_pct: f64,
    freshness_minutes: f64,
}

#[derive(Debug, Clone, Default)]
struct MiningPoolStatsCandidate {
    id: u64,
    name: String,
    symbol: String,
    algorithm: String,
    page: String,
    block_time_sec: f64,
    block_reward: f64,
    daily_emission: f64,
    network_hashrate_hs: f64,
    price_usd: f64,
    market_cap_usd: f64,
    volume_24h_usd: f64,
    price_trend_pct: f64,
    difficulty_trend_pct: f64,
    volatility: f64,
    lagging: bool,
    freshness_minutes: f64,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct HashrateHistoryPoint {
    difficulty: String,
    time: u64,
}

#[derive(Deserialize)]
struct WhatToMineResponse {
    coins: HashMap<String, HashMap<String, Value>>,
}

#[derive(Deserialize)]
struct CoinbaseResponse {
    data: CoinbasePrice,
}

#[derive(Deserialize)]
struct CoinbasePrice {
    amount: String,
}
