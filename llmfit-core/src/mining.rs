use crate::electricity::PowerContext;
use crate::rig_profiles::{AlgorithmRule, MiningRigProfile, RigKind, algorithm_rule};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::{LazyLock, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const HASHRATE_CPU_SYMBOLS: &[&str] = &[
    "EPIC", "XTM-RX", "DERO", "XMR", "SAL", "QRL", "ZEPH", "VRSC", "ETI", "FBIT", "RTM", "SCASH",
    "XKR", "XEL",
];
const HASHRATE_TIER_ONE_MIN_VOLUME_USD: f64 = 1_000.0;
const MININGPOOLSTATS_HOME_URL: &str = "https://miningpoolstats.stream/";
const MININGPOOLSTATS_DATA_BASE_URL: &str = "https://data.miningpoolstats.stream/data";
const MININGPOOLSTATS_BROWSER_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36";
const MININGPOOLSTATS_MIN_MARKET_CAP_USD: f64 = 10_000.0;
const MININGPOOLSTATS_MIN_VOLUME_USD: f64 = 100.0;
const SNAPSHOT_CACHE_SCHEMA_VERSION: u32 = 2;
const SNAPSHOT_CACHE_FRESH_SECS: u64 = 900;
const SNAPSHOT_ARCHIVE_KEEP_COUNT: usize = 24;
const COINPAPRIKA_COINS_URL: &str = "https://api.coinpaprika.com/v1/coins";
const COINPAPRIKA_TICKERS_URL: &str = "https://api.coinpaprika.com/v1/tickers";
const COINPAPRIKA_UA: &str = "minefit/0.7.4";
const COINGECKO_COINS_LIST_URL: &str = "https://api.coingecko.com/api/v3/coins/list";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    Fpps,
    Prop,
    Marketplace,
    SoloPool,
    SoloNode,
}

impl PayoutMode {
    pub fn label(&self) -> &'static str {
        match self {
            PayoutMode::Pplns => "PPLNS",
            PayoutMode::PpsPlus => "PPS+",
            PayoutMode::Fpps => "FPPS",
            PayoutMode::Prop => "PROP",
            PayoutMode::Marketplace => "Market",
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
    pub runtime_pct: f64,
    pub hashrate_multiplier: f64,
    pub power_multiplier: f64,
    pub reject_penalty_pct: f64,
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

const SUPPORTED_GPU_GENERAL: &[&str] = &[
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
    "X11",
    "NeoScrypt",
    "Equihash",
    "Equihash1445",
    "Equihash1927",
    "Lyra2z",
    "HMQ1725",
    "MyriadGroestl",
    "Skein",
    "Argon2d",
    "Lyra2REv2",
    "Blake2S",
    "Qubit",
    "Keccak",
    "Blake3",
    "HeavyHash",
];
const SUPPORTED_CPU: &[&str] = &[
    "RandomX",
    "AstroBWTv3",
    "VerusHash",
    "Ghostrider",
    "Randomscash",
    "CryptoNightTurtle",
    "Xelishashv3",
    "YesPower",
    "Yescrypt",
    "YescryptR16",
    "MinotaurX",
    "RandomARQ",
    "CryptoNightUPX",
    "Argon2idChukwa",
];
const SUPPORTED_SHA256: &[&str] = &["SHA256"];
const SUPPORTED_SCRYPT: &[&str] = &["Scrypt"];
const SUPPORTED_KHEAVY: &[&str] = &["KHeavyHash"];
const SUPPORTED_DISCOVERY: &[&str] = &["DiscoveryProxy"];
const SUPPORTED_SOLO: &[&str] = &[];
const RIGS_GPU_ONLY: &[RigKind] = &[RigKind::Gpu];
const RIGS_CPU_ONLY: &[RigKind] = &[RigKind::Cpu];
const RIGS_ASIC_ONLY: &[RigKind] = &[RigKind::Asic];
const RIGS_CPU_GPU_ASIC: &[RigKind] = &[RigKind::Cpu, RigKind::Gpu, RigKind::Asic];
const RIGS_ALL: &[RigKind] = &[];

fn technique(
    id: &'static str,
    name: &'static str,
    strategy: MiningStrategy,
    payout_mode: PayoutMode,
    description: &'static str,
    pool_fee_rate: f64,
    stale_rate_pct: f64,
    uptime_pct: f64,
    runtime_pct: f64,
    hashrate_multiplier: f64,
    power_multiplier: f64,
    reject_penalty_pct: f64,
    service_fee_usd_day: f64,
    score_bias: f64,
    source: &'static str,
    supported_algorithms: &'static [&'static str],
    supported_rig_kinds: &'static [RigKind],
) -> MiningMethod {
    MiningMethod {
        id,
        name,
        strategy,
        payout_mode,
        description,
        pool_fee_rate,
        stale_rate_pct,
        uptime_pct,
        runtime_pct,
        hashrate_multiplier,
        power_multiplier,
        reject_penalty_pct,
        service_fee_usd_day,
        score_bias,
        source,
        supported_algorithms,
        supported_rig_kinds,
    }
}

pub static METHODS: LazyLock<Vec<MiningMethod>> = LazyLock::new(|| {
    vec![
        technique(
            "gpu-core-pplns",
            "GPU Core PPLNS",
            MiningStrategy::Pool,
            PayoutMode::Pplns,
            "Baseline always-on GPU pool strategy with balanced fee and stale-share assumptions.",
            0.009,
            0.65,
            99.5,
            100.0,
            1.00,
            1.00,
            0.00,
            0.0,
            0.0,
            "Modeled from large global GPU pools",
            SUPPORTED_GPU_GENERAL,
            RIGS_GPU_ONLY,
        ),
        technique(
            "gpu-global-pps",
            "GPU Global PPS+",
            MiningStrategy::Pool,
            PayoutMode::PpsPlus,
            "Lower-variance GPU pool envelope that sacrifices a bit of fee for smoother payouts.",
            0.012,
            0.50,
            99.7,
            100.0,
            1.00,
            1.02,
            0.00,
            0.0,
            2.5,
            "Modeled from low-variance GPU pool fee bands",
            SUPPORTED_GPU_GENERAL,
            RIGS_GPU_ONLY,
        ),
        technique(
            "gpu-low-latency",
            "GPU Low-Latency",
            MiningStrategy::Pool,
            PayoutMode::Pplns,
            "Nearby-server GPU routing with tighter stale-share control and slightly better uptime.",
            0.010,
            0.35,
            99.8,
            100.0,
            1.00,
            1.00,
            -0.10,
            0.0,
            2.0,
            "Modeled from region-optimized stratum setups",
            SUPPORTED_GPU_GENERAL,
            RIGS_GPU_ONLY,
        ),
        technique(
            "gpu-auto-exchange",
            "GPU Auto-Exchange",
            MiningStrategy::Pool,
            PayoutMode::Marketplace,
            "Mine a supported GPU coin and auto-convert it on payout for simpler cashflow.",
            0.020,
            0.60,
            99.5,
            100.0,
            1.00,
            1.00,
            0.10,
            0.0,
            1.0,
            "Modeled from auto-exchange pool payout bands",
            SUPPORTED_GPU_GENERAL,
            RIGS_GPU_ONLY,
        ),
        technique(
            "gpu-marketplace",
            "GPU Marketplace",
            MiningStrategy::Pool,
            PayoutMode::Marketplace,
            "Sell GPU hashrate into a marketplace-style venue with instant buyer-side settlement.",
            0.030,
            0.45,
            99.7,
            100.0,
            0.99,
            1.00,
            0.00,
            0.0,
            1.5,
            "Modeled from hashrate marketplace spreads",
            SUPPORTED_GPU_GENERAL,
            RIGS_GPU_ONLY,
        ),
        technique(
            "gpu-eco-undervolt",
            "GPU Eco Undervolt",
            MiningStrategy::Pool,
            PayoutMode::Pplns,
            "Undervolted GPU tuning that trims watts faster than hashrate for better net economics.",
            0.010,
            0.55,
            99.4,
            100.0,
            0.94,
            0.78,
            0.00,
            0.0,
            2.2,
            "Modeled from modern low-watt GPU tuning envelopes",
            SUPPORTED_GPU_GENERAL,
            RIGS_GPU_ONLY,
        ),
        technique(
            "gpu-balanced-lock",
            "GPU Balanced Lock",
            MiningStrategy::Pool,
            PayoutMode::Pplns,
            "Conservative core-lock profile meant to preserve most yield while improving efficiency.",
            0.009,
            0.45,
            99.6,
            100.0,
            1.00,
            0.90,
            -0.05,
            0.0,
            1.8,
            "Modeled from balanced GPU lock profiles",
            SUPPORTED_GPU_GENERAL,
            RIGS_GPU_ONLY,
        ),
        technique(
            "gpu-turbo-oc",
            "GPU Turbo OC",
            MiningStrategy::Pool,
            PayoutMode::Pplns,
            "Aggressive GPU overclock profile that chases top-line output at the cost of power and rejects.",
            0.010,
            0.95,
            99.0,
            100.0,
            1.08,
            1.18,
            0.40,
            0.0,
            0.6,
            "Modeled from high-output GPU overclock profiles",
            SUPPORTED_GPU_GENERAL,
            RIGS_GPU_ONLY,
        ),
        technique(
            "gpu-proxy-stratum",
            "GPU Proxy Stratum",
            MiningStrategy::Pool,
            PayoutMode::Pplns,
            "Proxy-routed GPU stratum path with slightly lower latency and cleaner share timing.",
            0.008,
            0.30,
            99.8,
            100.0,
            1.00,
            1.00,
            -0.10,
            0.0,
            1.6,
            "Modeled from local proxy and stratum relay setups",
            SUPPORTED_GPU_GENERAL,
            RIGS_GPU_ONLY,
        ),
        technique(
            "gpu-small-pool-prop",
            "GPU Small-Pool PROP",
            MiningStrategy::Pool,
            PayoutMode::Prop,
            "Smaller pool exposure with thinner fees but more payout wobble and occasional stale drift.",
            0.005,
            0.90,
            98.7,
            100.0,
            1.00,
            1.00,
            0.10,
            0.0,
            -0.4,
            "Modeled from smaller PROP-style GPU pools",
            SUPPORTED_GPU_GENERAL,
            RIGS_GPU_ONLY,
        ),
        technique(
            "gpu-alt-niche",
            "GPU Alt-Niche Pool",
            MiningStrategy::Pool,
            PayoutMode::Pplns,
            "Niche-algo GPU pool route where algo coverage is wide but infra quality is less uniform.",
            0.011,
            0.80,
            99.2,
            100.0,
            1.01,
            1.00,
            0.10,
            0.0,
            0.8,
            "Modeled from niche GPU algo pool rosters",
            SUPPORTED_GPU_GENERAL,
            RIGS_GPU_ONLY,
        ),
        technique(
            "gpu-weekend-burst",
            "GPU Weekend Burst",
            MiningStrategy::Pool,
            PayoutMode::Pplns,
            "Partial-duty GPU schedule that favors higher-output windows instead of seven-day uptime.",
            0.010,
            0.55,
            99.5,
            55.0,
            1.05,
            1.08,
            0.15,
            0.0,
            -0.2,
            "Modeled from time-window GPU mining schedules",
            SUPPORTED_GPU_GENERAL,
            RIGS_GPU_ONLY,
        ),
        technique(
            "gpu-night-window",
            "GPU Night Window",
            MiningStrategy::Pool,
            PayoutMode::Pplns,
            "Night-only GPU operation tuned for cheaper hours and quieter thermals.",
            0.009,
            0.45,
            99.6,
            50.0,
            0.97,
            0.82,
            0.00,
            0.0,
            1.0,
            "Modeled from off-peak GPU mining schedules",
            SUPPORTED_GPU_GENERAL,
            RIGS_GPU_ONLY,
        ),
        technique(
            "gpu-hosted-rack",
            "GPU Hosted Rack",
            MiningStrategy::Hosted,
            PayoutMode::PpsPlus,
            "Managed GPU rack where uptime is strong but hosting overhead trims net returns.",
            0.000,
            0.40,
            99.0,
            100.0,
            0.98,
            1.05,
            0.00,
            0.95,
            -0.8,
            "Modeled from GPU hosting and colo service bands",
            SUPPORTED_GPU_GENERAL,
            RIGS_GPU_ONLY,
        ),
        technique(
            "gpu-solo-pool",
            "GPU Solo Pool",
            MiningStrategy::Solo,
            PayoutMode::SoloPool,
            "Solo-pool GPU route with pooled routing convenience but full block variance.",
            0.010,
            0.70,
            99.3,
            100.0,
            1.00,
            1.00,
            0.10,
            0.0,
            -4.0,
            "Modeled from solo-pool GPU routing",
            SUPPORTED_GPU_GENERAL,
            RIGS_GPU_ONLY,
        ),
        technique(
            "gpu-solo-node",
            "GPU Solo Node",
            MiningStrategy::Solo,
            PayoutMode::SoloNode,
            "Direct-node GPU mining with no pool fee and the highest payout variance.",
            0.000,
            1.10,
            98.5,
            100.0,
            1.00,
            1.00,
            0.20,
            0.03,
            -6.0,
            "Inference from direct-node GPU mining assumptions",
            SUPPORTED_GPU_GENERAL,
            RIGS_GPU_ONLY,
        ),
        technique(
            "cpu-core-pplns",
            "CPU Core PPLNS",
            MiningStrategy::Pool,
            PayoutMode::Pplns,
            "Baseline always-on CPU pool strategy across cache-sensitive algorithms.",
            0.009,
            0.80,
            99.2,
            100.0,
            1.00,
            1.00,
            0.00,
            0.0,
            1.0,
            "Modeled from established CPU pool fee bands",
            SUPPORTED_CPU,
            RIGS_CPU_ONLY,
        ),
        technique(
            "cpu-smooth-pps",
            "CPU Smooth PPS+",
            MiningStrategy::Pool,
            PayoutMode::PpsPlus,
            "Lower-variance CPU payout route that trades a bit of fee for steadier cashflow.",
            0.012,
            0.60,
            99.4,
            100.0,
            1.00,
            1.02,
            0.00,
            0.0,
            2.0,
            "Modeled from low-variance CPU pool envelopes",
            SUPPORTED_CPU,
            RIGS_CPU_ONLY,
        ),
        technique(
            "cpu-auto-switch",
            "CPU Auto-Switch",
            MiningStrategy::Pool,
            PayoutMode::Marketplace,
            "Rotate between supported CPU-first algorithms and auto-payout the strongest route.",
            0.018,
            0.75,
            99.2,
            100.0,
            1.02,
            1.02,
            0.10,
            0.0,
            1.4,
            "Modeled from CPU auto-switching pool behavior",
            SUPPORTED_CPU,
            RIGS_CPU_ONLY,
        ),
        technique(
            "cpu-cache-tuned",
            "CPU Cache-Tuned",
            MiningStrategy::Pool,
            PayoutMode::Pplns,
            "Cache-aware thread pinning and memory tuning that nudges CPU efficiency upward.",
            0.009,
            0.65,
            99.5,
            100.0,
            1.04,
            0.97,
            -0.05,
            0.0,
            1.8,
            "Modeled from tuned CPU miner deployments",
            SUPPORTED_CPU,
            RIGS_CPU_ONLY,
        ),
        technique(
            "cpu-hugepages-turbo",
            "CPU HugePages Turbo",
            MiningStrategy::Pool,
            PayoutMode::Pplns,
            "Aggressive huge-pages and thread tuning for higher CPU output and more heat.",
            0.009,
            0.85,
            99.0,
            100.0,
            1.10,
            1.12,
            0.25,
            0.0,
            0.5,
            "Modeled from maximum-throughput CPU configurations",
            SUPPORTED_CPU,
            RIGS_CPU_ONLY,
        ),
        technique(
            "cpu-eco-background",
            "CPU Eco Background",
            MiningStrategy::Pool,
            PayoutMode::Pplns,
            "Background CPU mining that leaves headroom for desktop responsiveness and thermals.",
            0.009,
            0.70,
            99.6,
            100.0,
            0.88,
            0.65,
            0.00,
            0.0,
            1.6,
            "Modeled from low-impact desktop CPU mining",
            SUPPORTED_CPU,
            RIGS_CPU_ONLY,
        ),
        technique(
            "cpu-night-window",
            "CPU Night Window",
            MiningStrategy::Pool,
            PayoutMode::Pplns,
            "Overnight-only CPU schedule that avoids daytime contention and higher room loads.",
            0.010,
            0.60,
            99.5,
            45.0,
            0.96,
            0.78,
            0.00,
            0.0,
            0.8,
            "Modeled from overnight CPU mining schedules",
            SUPPORTED_CPU,
            RIGS_CPU_ONLY,
        ),
        technique(
            "cpu-proxy-pool",
            "CPU Proxy Pool",
            MiningStrategy::Pool,
            PayoutMode::Pplns,
            "Local proxy route for CPU miners that trims share latency and keeps config centralized.",
            0.008,
            0.35,
            99.8,
            100.0,
            1.00,
            1.00,
            -0.10,
            0.0,
            1.6,
            "Modeled from proxied CPU miner farms",
            SUPPORTED_CPU,
            RIGS_CPU_ONLY,
        ),
        technique(
            "cpu-p2pool-mini",
            "CPU P2Pool Mini",
            MiningStrategy::Pool,
            PayoutMode::Prop,
            "Decentralized CPU mini-pool route with lower operator overhead and choppier payouts.",
            0.005,
            1.00,
            98.5,
            100.0,
            1.00,
            1.00,
            0.10,
            0.0,
            -0.6,
            "Modeled from decentralized CPU pool networks",
            SUPPORTED_CPU,
            RIGS_CPU_ONLY,
        ),
        technique(
            "cpu-mixed-algo",
            "CPU Mixed-Algorithm",
            MiningStrategy::Pool,
            PayoutMode::Marketplace,
            "Broader CPU algorithm rotation that prioritizes whichever supported chain clears best.",
            0.020,
            0.75,
            99.0,
            100.0,
            1.03,
            1.01,
            0.10,
            0.0,
            1.0,
            "Modeled from mixed-algorithm CPU routing",
            SUPPORTED_CPU,
            RIGS_CPU_ONLY,
        ),
        technique(
            "cpu-hosted-farm",
            "CPU Hosted Thread Farm",
            MiningStrategy::Hosted,
            PayoutMode::PpsPlus,
            "Remote CPU nodes with managed uptime and a hosting surcharge layered on top.",
            0.000,
            0.55,
            98.8,
            100.0,
            0.99,
            1.04,
            0.00,
            0.55,
            -0.7,
            "Modeled from hosted CPU fleet pricing",
            SUPPORTED_CPU,
            RIGS_CPU_ONLY,
        ),
        technique(
            "cpu-solo-pool",
            "CPU Solo Pool",
            MiningStrategy::Solo,
            PayoutMode::SoloPool,
            "Reference solo-pool envelope for CPU-first chains with full block variance.",
            0.010,
            0.90,
            99.1,
            100.0,
            1.00,
            1.00,
            0.10,
            0.0,
            -4.0,
            "Modeled from solo-pool CPU mining assumptions",
            SUPPORTED_CPU,
            RIGS_CPU_ONLY,
        ),
        technique(
            "cpu-solo-node",
            "CPU Solo Node",
            MiningStrategy::Solo,
            PayoutMode::SoloNode,
            "Direct-node CPU mining with no pool fee and maximum payout variance.",
            0.000,
            1.20,
            98.2,
            100.0,
            1.00,
            1.00,
            0.20,
            0.02,
            -6.5,
            "Inference from direct-node CPU mining assumptions",
            SUPPORTED_CPU,
            RIGS_CPU_ONLY,
        ),
        technique(
            "sha256-pool-pps",
            "SHA256 Pool PPS+",
            MiningStrategy::Pool,
            PayoutMode::PpsPlus,
            "Software or ASIC SHA256 mining into a steady PPS+ payout envelope.",
            0.025,
            0.40,
            99.7,
            100.0,
            1.00,
            1.00,
            0.00,
            0.0,
            1.6,
            "Modeled from large SHA256 pool fee bands",
            SUPPORTED_SHA256,
            RIGS_CPU_GPU_ASIC,
        ),
        technique(
            "sha256-pool-fpps",
            "SHA256 Pool FPPS",
            MiningStrategy::Pool,
            PayoutMode::Fpps,
            "Fee-smoothed SHA256 envelope that includes transaction-fee sharing in expectation.",
            0.028,
            0.35,
            99.8,
            100.0,
            1.00,
            1.00,
            0.00,
            0.0,
            2.0,
            "Modeled from FPPS-style SHA256 payouts",
            SUPPORTED_SHA256,
            RIGS_CPU_GPU_ASIC,
        ),
        technique(
            "sha256-marketplace",
            "SHA256 Marketplace",
            MiningStrategy::Pool,
            PayoutMode::Marketplace,
            "Sell SHA256 capacity into a marketplace-style venue instead of mining pure coin flow.",
            0.035,
            0.45,
            99.7,
            100.0,
            0.99,
            1.00,
            0.05,
            0.0,
            1.0,
            "Modeled from SHA256 marketplace spreads",
            SUPPORTED_SHA256,
            RIGS_CPU_GPU_ASIC,
        ),
        technique(
            "sha256-proxy-score",
            "SHA256 Proxy Score",
            MiningStrategy::Pool,
            PayoutMode::Pplns,
            "Proxy-routed SHA256 path with score-style behavior and lower network overhead.",
            0.018,
            0.25,
            99.8,
            100.0,
            1.00,
            1.00,
            -0.05,
            0.0,
            1.3,
            "Modeled from score-based SHA256 pool setups",
            SUPPORTED_SHA256,
            RIGS_CPU_GPU_ASIC,
        ),
        technique(
            "sha256-eco-window",
            "SHA256 Eco Window",
            MiningStrategy::Pool,
            PayoutMode::Pplns,
            "Partial-duty SHA256 operation limited to the best hours of the day.",
            0.022,
            0.35,
            99.6,
            50.0,
            0.95,
            0.80,
            0.00,
            0.0,
            0.8,
            "Modeled from windowed SHA256 mining schedules",
            SUPPORTED_SHA256,
            RIGS_CPU_GPU_ASIC,
        ),
        technique(
            "sha256-hosted-contract",
            "SHA256 Hosted Contract",
            MiningStrategy::Hosted,
            PayoutMode::Fpps,
            "Hosted SHA256 capacity with facility overhead layered onto otherwise stable pool economics.",
            0.000,
            0.30,
            99.5,
            100.0,
            0.98,
            1.04,
            0.00,
            2.40,
            -0.8,
            "Modeled from hosted SHA256 fleet contracts",
            SUPPORTED_SHA256,
            RIGS_CPU_GPU_ASIC,
        ),
        technique(
            "sha256-solo-pool",
            "SHA256 Solo Pool",
            MiningStrategy::Solo,
            PayoutMode::SoloPool,
            "Solo-pool SHA256 mode with full variance and pool-side routing convenience.",
            0.010,
            0.35,
            99.6,
            100.0,
            1.00,
            1.00,
            0.05,
            0.0,
            -3.0,
            "Modeled from solo-pool SHA256 operation",
            SUPPORTED_SHA256,
            RIGS_CPU_GPU_ASIC,
        ),
        technique(
            "sha256-solo-node",
            "SHA256 Solo Node",
            MiningStrategy::Solo,
            PayoutMode::SoloNode,
            "Direct-node SHA256 mining with no pool fee and extreme variance.",
            0.000,
            0.50,
            98.9,
            100.0,
            1.00,
            1.00,
            0.10,
            0.05,
            -6.0,
            "Inference from direct-node SHA256 assumptions",
            SUPPORTED_SHA256,
            RIGS_CPU_GPU_ASIC,
        ),
        technique(
            "scrypt-pool-pps",
            "Scrypt Pool PPS+",
            MiningStrategy::Pool,
            PayoutMode::PpsPlus,
            "Always-on Scrypt pool route with stable payout assumptions.",
            0.022,
            0.35,
            99.7,
            100.0,
            1.00,
            1.00,
            0.00,
            0.0,
            1.8,
            "Modeled from large Scrypt pool fee bands",
            SUPPORTED_SCRYPT,
            RIGS_ASIC_ONLY,
        ),
        technique(
            "scrypt-auto-profit",
            "Scrypt Auto-Profit",
            MiningStrategy::Pool,
            PayoutMode::Marketplace,
            "Auto-profit Scrypt route that optimizes for sellable output over pure coin loyalty.",
            0.030,
            0.45,
            99.4,
            100.0,
            0.99,
            1.02,
            0.05,
            0.0,
            0.9,
            "Modeled from auto-profit Scrypt venues",
            SUPPORTED_SCRYPT,
            RIGS_ASIC_ONLY,
        ),
        technique(
            "scrypt-hosted-contract",
            "Scrypt Hosted Contract",
            MiningStrategy::Hosted,
            PayoutMode::Fpps,
            "Hosted Scrypt operation with strong uptime and a facility surcharge.",
            0.000,
            0.30,
            99.5,
            100.0,
            0.98,
            1.04,
            0.00,
            2.10,
            -0.8,
            "Modeled from hosted Scrypt contracts",
            SUPPORTED_SCRYPT,
            RIGS_ASIC_ONLY,
        ),
        technique(
            "scrypt-solo-pool",
            "Scrypt Solo Pool",
            MiningStrategy::Solo,
            PayoutMode::SoloPool,
            "Solo-pool Scrypt mining where variance quickly dominates operator experience.",
            0.010,
            0.40,
            99.5,
            100.0,
            1.00,
            1.00,
            0.05,
            0.0,
            -3.2,
            "Modeled from solo-pool Scrypt operation",
            SUPPORTED_SCRYPT,
            RIGS_ASIC_ONLY,
        ),
        technique(
            "kheavy-pool-pplns",
            "KHeavy Pool PPLNS",
            MiningStrategy::Pool,
            PayoutMode::Pplns,
            "Baseline KHeavyHash pool envelope for always-on operation.",
            0.010,
            0.45,
            99.6,
            100.0,
            1.00,
            1.00,
            0.00,
            0.0,
            1.2,
            "Modeled from KHeavyHash pool fee bands",
            SUPPORTED_KHEAVY,
            RIGS_ASIC_ONLY,
        ),
        technique(
            "kheavy-pool-fpps",
            "KHeavy Pool FPPS",
            MiningStrategy::Pool,
            PayoutMode::Fpps,
            "Lower-variance KHeavyHash payout route that favors steadier realized cashflow.",
            0.015,
            0.35,
            99.7,
            100.0,
            1.00,
            1.00,
            0.00,
            0.0,
            2.0,
            "Modeled from KHeavyHash FPPS-style payouts",
            SUPPORTED_KHEAVY,
            RIGS_ASIC_ONLY,
        ),
        technique(
            "kheavy-hosted-contract",
            "KHeavy Hosted Contract",
            MiningStrategy::Hosted,
            PayoutMode::Fpps,
            "Hosted KHeavyHash capacity with managed uptime and an external facility fee.",
            0.000,
            0.30,
            99.3,
            100.0,
            0.98,
            1.04,
            0.00,
            1.60,
            -0.6,
            "Modeled from hosted KHeavyHash pricing",
            SUPPORTED_KHEAVY,
            RIGS_ASIC_ONLY,
        ),
        technique(
            "kheavy-solo-pool",
            "KHeavy Solo Pool",
            MiningStrategy::Solo,
            PayoutMode::SoloPool,
            "Solo-pool KHeavyHash route where payout variance dominates expected value.",
            0.010,
            0.40,
            99.5,
            100.0,
            1.00,
            1.00,
            0.05,
            0.0,
            -3.0,
            "Modeled from solo-pool KHeavyHash operation",
            SUPPORTED_KHEAVY,
            RIGS_ASIC_ONLY,
        ),
        technique(
            "discovery-proxy",
            "Discovery Proxy",
            MiningStrategy::Pool,
            PayoutMode::Marketplace,
            "Inferred long-tail ranking row built from market rank, liquidity, and trend when validated mining telemetry is missing.",
            0.020,
            0.90,
            98.5,
            100.0,
            1.00,
            1.00,
            0.10,
            0.0,
            -18.0,
            "CoinPaprika paged tickers plus minefit inference",
            SUPPORTED_DISCOVERY,
            RIGS_CPU_GPU_ASIC,
        ),
        technique(
            "solo-node",
            "Solo Node",
            MiningStrategy::Solo,
            PayoutMode::SoloNode,
            "Generic direct-node fallback for any validated benchmark and algorithm pair.",
            0.000,
            1.10,
            98.5,
            100.0,
            1.00,
            1.00,
            0.15,
            0.03,
            -7.0,
            "Inference from direct-node mining assumptions",
            SUPPORTED_SOLO,
            RIGS_ALL,
        ),
    ]
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningCoin {
    pub id: u64,
    pub name: String,
    pub symbol: String,
    pub algorithm: String,
    #[serde(default)]
    pub inferred_catalog: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogAsset {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub rank: u32,
    pub asset_type: String,
    pub is_active: bool,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningSnapshot {
    pub as_of: String,
    pub source: String,
    #[serde(default)]
    pub catalog_source: Option<String>,
    pub btc_usd: f64,
    pub coins: Vec<MiningCoin>,
    #[serde(default)]
    pub catalog_assets: Vec<CatalogAsset>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapshotLoadMode {
    Live,
    CacheWarm,
    CacheFallback,
}

impl SnapshotLoadMode {
    pub fn label(&self) -> &'static str {
        match self {
            SnapshotLoadMode::Live => "live",
            SnapshotLoadMode::CacheWarm => "cache",
            SnapshotLoadMode::CacheFallback => "stale-cache",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotCacheStatus {
    pub mode: SnapshotLoadMode,
    pub cache_age_minutes: Option<f64>,
    pub latest_snapshot_path: Option<String>,
}

impl SnapshotCacheStatus {
    pub fn badge(&self) -> String {
        match self.mode {
            SnapshotLoadMode::Live => "live now".to_string(),
            SnapshotLoadMode::CacheWarm => self
                .cache_age_minutes
                .map(|age| format!("cache {:.0}m", age))
                .unwrap_or_else(|| "cache".to_string()),
            SnapshotLoadMode::CacheFallback => self
                .cache_age_minutes
                .map(|age| format!("stale {:.0}m", age))
                .unwrap_or_else(|| "stale-cache".to_string()),
        }
    }

    pub fn summary_line(&self) -> String {
        match self.mode {
            SnapshotLoadMode::Live => "Live feeds loaded and cached.".to_string(),
            SnapshotLoadMode::CacheWarm => self
                .cache_age_minutes
                .map(|age| format!("Startup cache hit, snapshot age {:.1} minutes.", age))
                .unwrap_or_else(|| "Startup cache hit.".to_string()),
            SnapshotLoadMode::CacheFallback => self
                .cache_age_minutes
                .map(|age| {
                    format!(
                        "Live refresh failed, using cached snapshot from {:.1} minutes ago.",
                        age
                    )
                })
                .unwrap_or_else(|| "Live refresh failed, using cached snapshot.".to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SnapshotLoad {
    pub snapshot: MiningSnapshot,
    pub status: SnapshotCacheStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SnapshotCacheEnvelope {
    schema_version: u32,
    fetched_at_epoch: u64,
    snapshot: MiningSnapshot,
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
        let miningpoolstats =
            fetch_miningpoolstats_coins(btc_usd, &seen_symbols).unwrap_or_default();
        for coin in miningpoolstats {
            if seen_symbols.insert(coin.symbol.clone()) {
                coins.push(coin);
            }
        }
        if coins.len() > before_mps {
            sources.push("MiningPoolStats");
        }

        let (catalog_source, catalog_assets, inferred_catalog_coins) =
            fetch_discovery_catalog(btc_usd, &seen_symbols)
                .unwrap_or_else(|_| (String::new(), Vec::new(), Vec::new()));
        if !inferred_catalog_coins.is_empty() {
            sources.push("Discovery catalog inferred");
            for coin in inferred_catalog_coins {
                if seen_symbols.insert(coin.symbol.clone()) {
                    coins.push(coin);
                }
            }
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
            catalog_source: if catalog_assets.is_empty() {
                None
            } else {
                Some(catalog_source)
            },
            btc_usd,
            coins,
            catalog_assets,
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

    pub fn rankable_coin_count(&self) -> usize {
        self.coins.len()
    }

    pub fn catalog_asset_count(&self) -> usize {
        self.catalog_assets.len()
    }

    pub fn load_startup_snapshot() -> Result<SnapshotLoad, String> {
        load_snapshot_with_cache(false)
    }

    pub fn refresh_with_cache() -> Result<SnapshotLoad, String> {
        load_snapshot_with_cache(true)
    }
}

fn load_snapshot_with_cache(force_refresh: bool) -> Result<SnapshotLoad, String> {
    let cached = read_snapshot_cache();
    if !force_refresh && let Some((envelope, latest_path)) = &cached {
        let age_secs = current_epoch_seconds().saturating_sub(envelope.fetched_at_epoch);
        if age_secs <= SNAPSHOT_CACHE_FRESH_SECS {
            return Ok(SnapshotLoad {
                snapshot: envelope.snapshot.clone(),
                status: SnapshotCacheStatus {
                    mode: SnapshotLoadMode::CacheWarm,
                    cache_age_minutes: Some(age_secs as f64 / 60.0),
                    latest_snapshot_path: latest_path.to_str().map(|path| path.to_string()),
                },
            });
        }
    }

    match MiningSnapshot::fetch_live() {
        Ok(snapshot) => {
            let latest_path = write_snapshot_cache(&snapshot).ok();
            Ok(SnapshotLoad {
                snapshot,
                status: SnapshotCacheStatus {
                    mode: SnapshotLoadMode::Live,
                    cache_age_minutes: Some(0.0),
                    latest_snapshot_path: latest_path
                        .and_then(|path| path.to_str().map(|value| value.to_string())),
                },
            })
        }
        Err(err) => {
            if let Some((envelope, latest_path)) = cached {
                let age_secs = current_epoch_seconds().saturating_sub(envelope.fetched_at_epoch);
                Ok(SnapshotLoad {
                    snapshot: envelope.snapshot,
                    status: SnapshotCacheStatus {
                        mode: SnapshotLoadMode::CacheFallback,
                        cache_age_minutes: Some(age_secs as f64 / 60.0),
                        latest_snapshot_path: latest_path.to_str().map(|path| path.to_string()),
                    },
                })
            } else {
                Err(err)
            }
        }
    }
}

fn write_snapshot_cache(snapshot: &MiningSnapshot) -> Result<PathBuf, String> {
    let cache_dir = snapshot_cache_dir()
        .ok_or_else(|| "Unable to resolve minefit cache directory".to_string())?;
    let archive_dir = cache_dir.join("snapshots");
    fs::create_dir_all(&archive_dir)
        .map_err(|err| format!("Unable to create snapshot cache directory: {err}"))?;

    let fetched_at_epoch = current_epoch_seconds();
    let envelope = SnapshotCacheEnvelope {
        schema_version: SNAPSHOT_CACHE_SCHEMA_VERSION,
        fetched_at_epoch,
        snapshot: snapshot.clone(),
    };
    let payload = serde_json::to_vec_pretty(&envelope)
        .map_err(|err| format!("Unable to serialize snapshot cache: {err}"))?;

    let latest_path = cache_dir.join("latest.json");
    fs::write(&latest_path, &payload)
        .map_err(|err| format!("Unable to write snapshot cache: {err}"))?;

    let archived_path = archive_dir.join(format!("snapshot-{}.json", fetched_at_epoch));
    let _ = fs::write(&archived_path, payload);
    prune_snapshot_archives(&archive_dir);

    Ok(latest_path)
}

fn read_snapshot_cache() -> Option<(SnapshotCacheEnvelope, PathBuf)> {
    let latest_path = snapshot_cache_dir()?.join("latest.json");
    let raw = fs::read_to_string(&latest_path).ok()?;
    let envelope = serde_json::from_str::<SnapshotCacheEnvelope>(&raw).ok()?;
    if envelope.schema_version != SNAPSHOT_CACHE_SCHEMA_VERSION {
        return None;
    }
    Some((envelope, latest_path))
}

fn prune_snapshot_archives(archive_dir: &PathBuf) {
    let Ok(entries) = fs::read_dir(archive_dir) else {
        return;
    };

    let mut snapshots = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let metadata = entry.metadata().ok()?;
            let modified = metadata.modified().ok()?;
            Some((modified, entry.path()))
        })
        .collect::<Vec<_>>();
    snapshots.sort_by(|left, right| right.0.cmp(&left.0));

    for (_, path) in snapshots.into_iter().skip(SNAPSHOT_ARCHIVE_KEEP_COUNT) {
        let _ = fs::remove_file(path);
    }
}

fn snapshot_cache_dir() -> Option<PathBuf> {
    minefit_config_dir().map(|path| path.join("cache"))
}

fn minefit_config_dir() -> Option<PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()?;
    Some(PathBuf::from(home).join(".config").join("minefit"))
}

fn current_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
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

fn fetch_discovery_catalog(
    btc_usd: f64,
    existing_symbols: &HashSet<String>,
) -> Result<(String, Vec<CatalogAsset>, Vec<MiningCoin>), String> {
    fetch_coinpaprika_market_catalog(btc_usd, existing_symbols)
        .or_else(|_| fetch_coingecko_catalog(btc_usd, existing_symbols))
}

fn fetch_coinpaprika_market_catalog(
    btc_usd: f64,
    existing_symbols: &HashSet<String>,
) -> Result<(String, Vec<CatalogAsset>, Vec<MiningCoin>), String> {
    let response = ureq::get(COINPAPRIKA_COINS_URL)
        .header("User-Agent", COINPAPRIKA_UA)
        .config()
        .timeout_global(Some(Duration::from_secs(8)))
        .build()
        .call()
        .map_err(|err| format!("CoinPaprika coins request failed: {err}"))?;
    let mut rows: Vec<CoinPaprikaCoin> = response
        .into_body()
        .read_json()
        .map_err(|err| format!("CoinPaprika coins JSON parse failed: {err}"))?;
    rows.retain(|row| row.is_active && row.rank > 0);
    rows.sort_by_key(|row| row.rank);

    let market_overlays = fetch_coinpaprika_market_tickers().unwrap_or_default();
    let mut catalog = Vec::new();
    let mut inferred = Vec::new();
    let mut seen_display_symbols = existing_symbols.clone();

    for row in rows {
        catalog.push(CatalogAsset {
            id: row.id.clone(),
            name: row.name.clone(),
            symbol: row.symbol.clone(),
            rank: row.rank as u32,
            asset_type: row.asset_type.clone(),
            is_active: row.is_active,
            source: "CoinPaprika /coins".to_string(),
        });

        let inferred_coin = market_overlays
            .get(&row.id)
            .and_then(|ticker| {
                infer_coinpaprika_mining_coin(ticker, btc_usd, &mut seen_display_symbols)
            })
            .or_else(|| infer_coinpaprika_catalog_coin(&row, btc_usd, &mut seen_display_symbols));

        if let Some(coin) = inferred_coin {
            inferred.push(coin);
        }
    }

    catalog.sort_by(|left, right| left.rank.cmp(&right.rank));
    let source = if market_overlays.is_empty() {
        "CoinPaprika /coins discovery catalog".to_string()
    } else {
        "CoinPaprika /coins + /tickers discovery catalog".to_string()
    };
    Ok((source, catalog, inferred))
}

fn fetch_coingecko_catalog(
    btc_usd: f64,
    existing_symbols: &HashSet<String>,
) -> Result<(String, Vec<CatalogAsset>, Vec<MiningCoin>), String> {
    let response = ureq::get(COINGECKO_COINS_LIST_URL)
        .header("User-Agent", COINPAPRIKA_UA)
        .config()
        .timeout_global(Some(Duration::from_secs(8)))
        .build()
        .call()
        .map_err(|err| format!("CoinGecko coins list request failed: {err}"))?;
    let rows: Vec<CoinGeckoCoin> = response
        .into_body()
        .read_json()
        .map_err(|err| format!("CoinGecko coins list JSON parse failed: {err}"))?;

    let mut catalog = Vec::with_capacity(rows.len());
    let mut inferred = Vec::with_capacity(rows.len());
    let mut seen_display_symbols = existing_symbols.clone();

    for (index, row) in rows.into_iter().enumerate() {
        let rank = (index + 1) as u32;
        let coin = CoinPaprikaCoin {
            id: row.id.clone(),
            name: row.name.clone(),
            symbol: row.symbol,
            rank: rank as i64,
            asset_type: "asset".to_string(),
            is_active: true,
        };

        catalog.push(CatalogAsset {
            id: coin.id.clone(),
            name: coin.name.clone(),
            symbol: coin.symbol.clone(),
            rank,
            asset_type: coin.asset_type.clone(),
            is_active: true,
            source: "CoinGecko /coins/list".to_string(),
        });

        if let Some(inferred_coin) =
            infer_coinpaprika_catalog_coin(&coin, btc_usd, &mut seen_display_symbols)
        {
            inferred.push(inferred_coin);
        }
    }

    Ok((
        "CoinGecko /coins/list discovery catalog".to_string(),
        catalog,
        inferred,
    ))
}

fn fetch_coinpaprika_market_tickers() -> Result<HashMap<String, CoinPaprikaTicker>, String> {
    let response = ureq::get(&format!("{COINPAPRIKA_TICKERS_URL}?quotes=USD"))
        .header("User-Agent", COINPAPRIKA_UA)
        .config()
        .timeout_global(Some(Duration::from_secs(8)))
        .build()
        .call()
        .map_err(|err| format!("CoinPaprika tickers request failed: {err}"))?;
    let rows: Vec<CoinPaprikaTicker> = response
        .into_body()
        .read_json()
        .map_err(|err| format!("CoinPaprika tickers JSON parse failed: {err}"))?;

    Ok(rows
        .into_iter()
        .filter(|row| row.rank > 0)
        .map(|row| (row.id.clone(), row))
        .collect())
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

        let reference_hashrate_hs = (reference_coin_per_day / daily_emission) * network_hashrate_hs;
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
            inferred_catalog: false,
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
        let network_hashrate_hs =
            number_value(raw.get("hashrate")).max(number_value(raw.get("ph")));
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

        if (candidate.daily_emission <= 0.0 || candidate.block_reward <= 0.0)
            && !candidate.page.is_empty()
        {
            if let Ok(page_json) = fetch_miningpoolstats_json(&format!(
                "{MININGPOOLSTATS_DATA_BASE_URL}/{}.js?t={timestamp}",
                candidate.page
            )) {
                candidate.price_usd =
                    first_positive(&[candidate.price_usd, number_value(page_json.get("price"))]);
                candidate.block_time_sec = first_positive(&[
                    candidate.block_time_sec,
                    number_value(page_json.get("block_time_target")),
                    number_value(page_json.get("block_time_average")),
                ]);
                candidate.daily_emission = first_positive(&[
                    candidate.daily_emission,
                    number_value(
                        page_json
                            .get("supply")
                            .and_then(|supply| supply.get("emission24")),
                    ),
                ]);
                candidate.block_reward = first_positive(&[
                    candidate.block_reward,
                    number_value(
                        page_json
                            .get("minerstat")
                            .and_then(Value::as_array)
                            .and_then(|stats| stats.get(2)),
                    ),
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
    let reference_coin_per_day =
        candidate.daily_emission / candidate.network_hashrate_hs.max(f64::EPSILON);
    let reference_btc_revenue =
        (reference_coin_per_day * candidate.price_usd) / btc_usd.max(f64::EPSILON);
    let profitability = reference_btc_revenue * 1_000_000_000_000.0;

    Some(MiningCoin {
        id: candidate.id,
        name: candidate.name.clone(),
        symbol: candidate.symbol.clone(),
        algorithm: candidate.algorithm.clone(),
        inferred_catalog: false,
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
            | "x11"
            | "neoscrypt"
            | "equihash"
            | "equihash1445"
            | "equihash1927"
            | "yespower"
            | "yescrypt"
            | "yescryptr16"
            | "minotaurx"
            | "lyra2z"
            | "hmq1725"
            | "myriadgroestl"
            | "cryptonightupx"
            | "skein"
            | "argon2d"
            | "argon2idchukwa"
            | "lyra2rev2"
            | "blake2s"
            | "qubit"
            | "randomarq"
            | "keccak"
            | "blake3"
            | "heavyhash"
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

fn infer_coinpaprika_mining_coin(
    ticker: &CoinPaprikaTicker,
    btc_usd: f64,
    seen_display_symbols: &mut HashSet<String>,
) -> Option<MiningCoin> {
    let usd = ticker.quotes.usd.as_ref()?;
    if usd.price <= 0.0 || ticker.rank <= 0 {
        return None;
    }

    let rank_factor = clamp(1.0 - ((ticker.rank as f64).log10() / 5.0), 0.0, 1.0);
    let market_factor = scaled_log_score(usd.market_cap, 4.0, 12.0);
    let volume_factor = scaled_log_score(usd.volume_24h, 3.0, 11.0);
    let trend_factor = clamp((usd.percent_change_24h + 20.0) / 60.0, 0.0, 1.0);
    let proxy_usd_day = clamp(
        0.015
            + (rank_factor * 0.24)
            + (market_factor * 0.32)
            + (volume_factor * 0.28)
            + (trend_factor * 0.10),
        0.01,
        0.95,
    );

    let network_hashrate_hs = 1_000.0;
    let reference_coin_per_day = proxy_usd_day / usd.price.max(f64::EPSILON);
    let daily_emission = reference_coin_per_day * network_hashrate_hs;
    let block_time_sec = 60.0;
    let blocks_per_day = 86_400.0 / block_time_sec;
    let block_reward = daily_emission / blocks_per_day;
    let display_symbol =
        unique_catalog_symbol(&ticker.symbol, ticker.rank as u32, seen_display_symbols);

    Some(MiningCoin {
        id: synthetic_coin_id(&ticker.id),
        name: ticker.name.clone(),
        symbol: display_symbol,
        algorithm: "DiscoveryProxy".to_string(),
        inferred_catalog: true,
        block_time_sec,
        block_reward,
        blocks_per_day,
        daily_emission,
        network_hashrate_hs,
        exchange_rate_btc: usd.price / btc_usd.max(f64::EPSILON),
        price_usd: usd.price,
        market_cap_usd: usd.market_cap,
        volume_24h_usd: usd.volume_24h,
        profitability: (proxy_usd_day / btc_usd.max(f64::EPSILON)) * 1_000_000_000_000.0,
        profitability24: (proxy_usd_day / btc_usd.max(f64::EPSILON)) * 1_000_000_000_000.0,
        reference_coin_per_day,
        reference_btc_revenue: proxy_usd_day / btc_usd.max(f64::EPSILON),
        reference_hashrate_hs: 1.0,
        price_trend_pct: usd.percent_change_24h,
        difficulty_trend_pct: 0.0,
        volatility: ((usd.percent_change_24h.abs() * 0.55)
            + (usd.percent_change_7d.abs() * 0.30)
            + (usd.percent_change_30d.abs() * 0.15))
            / 10.0,
        lagging: false,
        freshness_minutes: 0.0,
    })
}

fn infer_coinpaprika_catalog_coin(
    coin: &CoinPaprikaCoin,
    btc_usd: f64,
    seen_display_symbols: &mut HashSet<String>,
) -> Option<MiningCoin> {
    if coin.rank <= 0 {
        return None;
    }

    let rank_factor = clamp(1.0 - ((coin.rank as f64).log10() / 5.0), 0.0, 1.0);
    let type_factor = if coin.asset_type.eq_ignore_ascii_case("coin") {
        1.0
    } else {
        0.85
    };
    let synthetic_price_usd = 10f64.powf(-4.0 + (rank_factor * 5.0));
    let market_cap_usd = 10f64.powf(3.5 + (rank_factor * 8.5));
    let volume_24h_usd = 10f64.powf(2.0 + (rank_factor * 7.0));
    let proxy_usd_day = clamp((0.004 + (rank_factor * 0.22)) * type_factor, 0.002, 0.26);

    let network_hashrate_hs = 1_000.0;
    let block_time_sec = 60.0;
    let blocks_per_day = 86_400.0 / block_time_sec;
    let reference_coin_per_day = proxy_usd_day / synthetic_price_usd.max(f64::EPSILON);
    let daily_emission = reference_coin_per_day * network_hashrate_hs;
    let block_reward = daily_emission / blocks_per_day;
    let display_symbol =
        unique_catalog_symbol(&coin.symbol, coin.rank as u32, seen_display_symbols);

    Some(MiningCoin {
        id: synthetic_coin_id(&coin.id),
        name: coin.name.clone(),
        symbol: display_symbol,
        algorithm: "DiscoveryProxy".to_string(),
        inferred_catalog: true,
        block_time_sec,
        block_reward,
        blocks_per_day,
        daily_emission,
        network_hashrate_hs,
        exchange_rate_btc: synthetic_price_usd / btc_usd.max(f64::EPSILON),
        price_usd: synthetic_price_usd,
        market_cap_usd,
        volume_24h_usd,
        profitability: (proxy_usd_day / btc_usd.max(f64::EPSILON)) * 1_000_000_000_000.0,
        profitability24: (proxy_usd_day / btc_usd.max(f64::EPSILON)) * 1_000_000_000_000.0,
        reference_coin_per_day,
        reference_btc_revenue: proxy_usd_day / btc_usd.max(f64::EPSILON),
        reference_hashrate_hs: 1.0,
        price_trend_pct: 0.0,
        difficulty_trend_pct: 0.0,
        volatility: 1.25 - (rank_factor * 0.75),
        lagging: true,
        freshness_minutes: 1_440.0,
    })
}

fn unique_catalog_symbol(
    symbol: &str,
    rank: u32,
    seen_display_symbols: &mut HashSet<String>,
) -> String {
    let base = symbol.trim().to_uppercase();
    if seen_display_symbols.insert(base.clone()) {
        return base;
    }

    let mut candidate = format!("{}@{}", base, rank);
    if seen_display_symbols.insert(candidate.clone()) {
        return candidate;
    }

    let mut suffix = 2u32;
    loop {
        candidate = format!("{}@{}-{}", base, rank, suffix);
        if seen_display_symbols.insert(candidate.clone()) {
            return candidate;
        }
        suffix += 1;
    }
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
        Regex::new(r"coins_data\.js\?t=(\d+)")
            .expect("MiningPoolStats timestamp regex should compile")
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

        if self.coin.inferred_catalog {
            reasons.push(
                "This row is inferred from catalog market data, not validated mining telemetry."
                    .to_string(),
            );
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
            "{} payout mode with {:.2}% fee, {:.2}% stale-share, {:.1}% uptime, and {:.0}% duty cycle.",
            self.method.payout_mode.label(),
            self.method.pool_fee_rate * 100.0,
            self.method.stale_rate_pct,
            self.method.uptime_pct,
            self.method.runtime_pct
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
                "Upstream marked the coin feed as lagging, so treat this row as directional."
                    .to_string(),
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

            for method in METHODS.iter() {
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

                let runtime_multiplier = clamp(method.runtime_pct / 100.0, 0.0, 1.0);
                let adjusted_benchmark_hashrate_hs =
                    benchmark_hashrate_hs * method.hashrate_multiplier.max(0.0);
                let adjusted_benchmark_power_watts =
                    benchmark_power_watts * method.power_multiplier.max(0.0);
                let effective_reject_rate_pct =
                    (reject_rate_pct + method.reject_penalty_pct).max(0.0);
                let scaled_hashrate_hs = adjusted_benchmark_hashrate_hs * rig_scale;
                let share_of_network = scaled_hashrate_hs
                    / (coin.network_hashrate_hs + scaled_hashrate_hs).max(f64::EPSILON);
                let gross_coin_day_raw = coin.daily_emission * share_of_network;
                let gross_usd_day_raw = gross_coin_day_raw * coin.price_usd;
                let gross_btc_day_raw = gross_usd_day_raw / snapshot.btc_usd.max(f64::EPSILON);

                let efficiency_multiplier = runtime_multiplier
                    * (method.uptime_pct / 100.0)
                    * (1.0 - method.stale_rate_pct / 100.0)
                    * (1.0 - effective_reject_rate_pct / 100.0);
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

                let power_estimate = power.estimate_cost(
                    adjusted_benchmark_power_watts * runtime_multiplier * rig_scale,
                    30.0,
                );
                let fee_cost_usd_day = gross_usd_day * method.pool_fee_rate;
                let stale_cost_usd_day =
                    gross_usd_day_raw * (method.stale_rate_pct / 100.0) * liquidity_factor;
                let service_cost_usd_day = method.service_fee_usd_day * rig_scale;
                let net_usd_day = gross_usd_day
                    - power_estimate.daily_cost_usd
                    - fee_cost_usd_day
                    - service_cost_usd_day;
                let blocks_day = share_of_network * coin.blocks_per_day * efficiency_multiplier;
                let blocks_month = blocks_day * 30.0;
                let (variance_zero_block_pct, variance_p50_usd_month, variance_p90_usd_month) =
                    payout_variance(
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
                    benchmark_hashrate_hs: adjusted_benchmark_hashrate_hs,
                    benchmark_power_watts: adjusted_benchmark_power_watts,
                    reject_rate_pct: effective_reject_rate_pct,
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
            SortColumn::MarketCap => cmp_f64(liquidity_usd(&left.coin), liquidity_usd(&right.coin)),
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
                    + if row.coin.inferred_catalog {
                        -14.0
                    } else {
                        0.0
                    }
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
        if coin.inferred_catalog {
            return (
                true,
                format!(
                    "Inferred discovery proxy for {} on {}. This row is rankable, but not backed by validated mining telemetry.",
                    coin.name, rig.name
                ),
                benchmark.hashrate_hs,
                benchmark.power_watts,
                benchmark.reject_rate_pct,
                benchmark.miner,
                benchmark.tuning,
            );
        }
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
        PayoutMode::Prop => (
            0.0,
            expected_monthly_net * 0.95,
            expected_monthly_net * 1.09,
        ),
        PayoutMode::Pplns => (
            0.0,
            expected_monthly_net * 0.97,
            expected_monthly_net * 1.05,
        ),
        PayoutMode::Fpps => (
            0.0,
            expected_monthly_net * 0.997,
            expected_monthly_net * 1.015,
        ),
        PayoutMode::PpsPlus => (
            0.0,
            expected_monthly_net * 0.995,
            expected_monthly_net * 1.02,
        ),
        PayoutMode::Marketplace => (
            0.0,
            expected_monthly_net * 0.99,
            expected_monthly_net * 1.03,
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
    let raw = 58.0 + price_trend_pct * 3.3
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
    let pool_quality =
        row.method.uptime_pct - row.method.stale_rate_pct * 3.0 - row.reject_rate_pct * 2.0;

    match row.method.payout_mode {
        PayoutMode::SoloPool | PayoutMode::SoloNode => {
            let base = 42.0 + row.blocks_month.min(5.0) * 9.0 - row.variance_zero_block_pct * 0.35;
            clamp(base - volatility_penalty, 0.0, 100.0)
        }
        PayoutMode::Prop => clamp(pool_quality - 4.0 - volatility_penalty * 1.05, 0.0, 100.0),
        PayoutMode::Pplns => clamp(pool_quality - volatility_penalty, 0.0, 100.0),
        PayoutMode::Fpps => clamp(pool_quality + 7.0 - volatility_penalty * 0.7, 0.0, 100.0),
        PayoutMode::PpsPlus => clamp(pool_quality + 6.0 - volatility_penalty * 0.8, 0.0, 100.0),
        PayoutMode::Marketplace => clamp(pool_quality + 4.0 - volatility_penalty * 0.9, 0.0, 100.0),
    }
}

fn variance_score(row: &MiningRow) -> f64 {
    if !row.eligible {
        return 0.0;
    }

    match row.method.payout_mode {
        PayoutMode::Fpps => 94.0,
        PayoutMode::PpsPlus => 92.0,
        PayoutMode::Marketplace => 88.0,
        PayoutMode::Prop => 70.0,
        PayoutMode::Pplns => 78.0,
        PayoutMode::SoloPool | PayoutMode::SoloNode => {
            clamp(100.0 - row.variance_zero_block_pct, 0.0, 100.0)
        }
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

fn scaled_log_score(value: f64, min_log10: f64, max_log10: f64) -> f64 {
    if value <= 0.0 {
        return 0.0;
    }

    clamp(
        (value.log10() - min_log10) / (max_log10 - min_log10).max(f64::EPSILON),
        0.0,
        1.0,
    )
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
            inferred_catalog: false,
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
    symbol.bytes().fold(7_000_000_000u64, |acc, byte| {
        acc.wrapping_mul(131).wrapping_add(byte as u64)
    })
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
        "x11" => "X11".to_string(),
        "neoscrypt" => "NeoScrypt".to_string(),
        "equihash" => "Equihash".to_string(),
        "equihash 144,5" => "Equihash1445".to_string(),
        "equihash 192,7" => "Equihash1927".to_string(),
        "yespower" | "yespower r16" | "yespowerr16" | "yespowerltncg" | "yespoweriots"
        | "yespoweric" | "yespowerlitb" | "yespowerurx" => "YesPower".to_string(),
        "yescrypt" => "Yescrypt".to_string(),
        "yescryptr16" => "YescryptR16".to_string(),
        "minotaurx" => "MinotaurX".to_string(),
        "lyra2z" => "Lyra2z".to_string(),
        "hmq1725" => "HMQ1725".to_string(),
        "myriad-groestl" | "myriad groestl" => "MyriadGroestl".to_string(),
        "cryptonight upx" | "cryptonightupx" => "CryptoNightUPX".to_string(),
        "skein" => "Skein".to_string(),
        "argon2d" => "Argon2d".to_string(),
        "argon2id chukwa" | "argon2idchukwa" => "Argon2idChukwa".to_string(),
        "lyra2rev2" | "lyra2rev 2" => "Lyra2REv2".to_string(),
        "blake2s" => "Blake2S".to_string(),
        "qubit" => "Qubit".to_string(),
        "randomarq" => "RandomARQ".to_string(),
        "keccak" => "Keccak".to_string(),
        "blake3" => "Blake3".to_string(),
        "heavyhash" => "HeavyHash".to_string(),
        "sha3solidity" | "sha3 solidity" => "SHA3Solidity".to_string(),
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
    value
        .trim()
        .trim_end_matches('%')
        .parse::<f64>()
        .unwrap_or(0.0)
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

#[derive(Debug, Deserialize)]
struct CoinPaprikaCoin {
    id: String,
    name: String,
    symbol: String,
    rank: i64,
    #[serde(rename = "type")]
    asset_type: String,
    is_active: bool,
}

#[derive(Debug, Deserialize)]
struct CoinGeckoCoin {
    id: String,
    symbol: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct CoinPaprikaTicker {
    id: String,
    name: String,
    symbol: String,
    rank: i64,
    quotes: CoinPaprikaQuotes,
}

#[derive(Debug, Deserialize)]
struct CoinPaprikaQuotes {
    #[serde(rename = "USD")]
    usd: Option<CoinPaprikaQuote>,
}

#[derive(Debug, Deserialize)]
struct CoinPaprikaQuote {
    price: f64,
    volume_24h: f64,
    market_cap: f64,
    percent_change_24h: f64,
    percent_change_7d: f64,
    percent_change_30d: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn method_catalog_stays_broad_and_unique() {
        assert!(
            METHODS.len() >= 40,
            "expected at least 40 modeled techniques, found {}",
            METHODS.len()
        );

        let ids = METHODS.iter().map(|method| method.id).collect::<Vec<_>>();
        let unique = ids.iter().copied().collect::<HashSet<_>>();
        assert_eq!(ids.len(), unique.len(), "method ids should be unique");
    }

    #[test]
    fn catalog_only_inference_produces_rankable_proxy_coin() {
        let coin = CoinPaprikaCoin {
            id: "synthetic-coin".to_string(),
            name: "Synthetic Coin".to_string(),
            symbol: "SYNC".to_string(),
            rank: 123,
            asset_type: "coin".to_string(),
            is_active: true,
        };
        let mut seen = HashSet::new();

        let inferred =
            infer_coinpaprika_catalog_coin(&coin, 100_000.0, &mut seen).expect("coin should infer");

        assert_eq!(inferred.algorithm, "DiscoveryProxy");
        assert!(inferred.inferred_catalog);
        assert!(inferred.reference_btc_revenue > 0.0);
        assert!(inferred.price_usd > 0.0);
    }
}
