use crate::hardware::SystemSpecs;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RigKind {
    Gpu,
    Asic,
    Cpu,
}

#[derive(Debug, Clone, Serialize)]
pub struct AlgorithmBenchmark {
    pub algorithm: &'static str,
    pub hashrate_hs: f64,
    pub power_watts: f64,
    pub reject_rate_pct: f64,
    pub miner: &'static str,
    pub tuning: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct MiningRigProfile {
    pub id: &'static str,
    pub name: &'static str,
    pub vendor: &'static str,
    pub kind: RigKind,
    pub memory_gb: Option<f64>,
    pub source: &'static str,
    pub tuning_summary: &'static str,
    pub fallback_power_watts: f64,
    pub benchmarks: Vec<AlgorithmBenchmark>,
}

impl MiningRigProfile {
    pub fn benchmark_for(&self, algorithm: &str) -> Option<&AlgorithmBenchmark> {
        self.benchmarks
            .iter()
            .find(|benchmark| benchmark.algorithm.eq_ignore_ascii_case(algorithm))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AlgorithmRule {
    pub algorithm: &'static str,
    pub min_vram_gb: f64,
    pub supports_nvidia: bool,
    pub supports_amd: bool,
    pub supports_asic: bool,
    pub supports_cpu: bool,
    pub tuning_note: &'static str,
    pub backend_note: &'static str,
}

pub fn resolve_rig_profile(specs: &SystemSpecs, profile_hint: Option<&str>) -> MiningRigProfile {
    let profiles = available_rig_profiles();
    if let Some(hint) = profile_hint {
        let normalized = normalize_id(hint);
        if let Some(profile) = profiles
            .iter()
            .find(|profile| normalize_id(profile.id) == normalized)
        {
            return profile.clone();
        }
    }

    resolve_detected_gpu_profile(specs).unwrap_or_else(|| resolve_detected_cpu_profile(specs))
}

pub fn resolve_detected_rig_profiles(specs: &SystemSpecs) -> Vec<MiningRigProfile> {
    let mut rigs = Vec::new();

    if let Some(gpu) = resolve_detected_gpu_profile(specs) {
        rigs.push(gpu);
    }

    rigs.push(resolve_detected_cpu_profile(specs));
    rigs
}

pub fn resolve_default_comparison_rigs(specs: &SystemSpecs) -> Vec<MiningRigProfile> {
    let mut rigs = resolve_detected_rig_profiles(specs);
    rigs.extend(reference_asic_profiles());
    rigs
}

pub fn describe_rig_scope(rigs: &[MiningRigProfile]) -> String {
    if rigs.is_empty() {
        return "No rig".to_string();
    }

    if rigs.len() == 1 {
        return rigs[0].name.to_string();
    }

    let has_cpu = rigs.iter().any(|rig| matches!(rig.kind, RigKind::Cpu));
    let has_gpu = rigs.iter().any(|rig| matches!(rig.kind, RigKind::Gpu));
    let has_asic = rigs.iter().any(|rig| matches!(rig.kind, RigKind::Asic));

    if has_cpu && has_gpu && has_asic {
        "Local CPU + GPU + ASIC Baselines".to_string()
    } else if has_cpu && has_gpu && !has_asic {
        "Local CPU + GPU".to_string()
    } else if has_gpu && has_asic && !has_cpu {
        "GPU + ASIC".to_string()
    } else {
        rigs.iter()
            .map(|rig| rig.name)
            .collect::<Vec<_>>()
            .join(" + ")
    }
}

pub fn describe_rig_scope_summary(rigs: &[MiningRigProfile]) -> String {
    if rigs.is_empty() {
        return "No detected rig profiles are active.".to_string();
    }

    let has_cpu = rigs.iter().any(|rig| matches!(rig.kind, RigKind::Cpu));
    let has_gpu = rigs.iter().any(|rig| matches!(rig.kind, RigKind::Gpu));
    let has_asic = rigs.iter().any(|rig| matches!(rig.kind, RigKind::Asic));

    if has_cpu && has_gpu && has_asic {
        return "Detected local CPU/GPU plus reference SHA256, Scrypt, and KHeavyHash ASIC baselines."
            .to_string();
    }

    if rigs.len() == 1 {
        return rigs[0].tuning_summary.to_string();
    }

    rigs.iter()
        .map(|rig| format!("{}: {}", rig.name, rig.tuning_summary))
        .collect::<Vec<_>>()
        .join(" | ")
}

pub fn reference_asic_profiles() -> Vec<MiningRigProfile> {
    vec![
        antminer_s19_pro_profile(),
        antminer_l9_profile(),
        iceriver_ks5m_profile(),
    ]
}

pub fn available_rig_profiles() -> Vec<MiningRigProfile> {
    let mut profiles = vec![
        rtx_4060_profile(),
        rtx_4090_profile(),
        rx_6700_xt_profile(),
        intel_i7_14700f_proxy_profile(),
        ryzen_5_7600_profile(),
        ryzen_9_7950x3d_profile(),
        amd_epyc_9754_profile(),
    ];
    profiles.extend(reference_asic_profiles());
    profiles
}

pub fn algorithm_rule(algorithm: &str) -> AlgorithmRule {
    match algorithm.to_ascii_lowercase().as_str() {
        "ethash" => AlgorithmRule {
            algorithm: "Ethash",
            min_vram_gb: 6.0,
            supports_nvidia: true,
            supports_amd: true,
            supports_asic: true,
            supports_cpu: false,
            tuning_note: "Large DAG and memory clock sensitivity; 6 GB is the practical floor in 2026.",
            backend_note: "Backend path: CUDA/OpenCL GPU miners or dedicated Ethash ASIC firmware.",
        },
        "etchash" => AlgorithmRule {
            algorithm: "Etchash",
            min_vram_gb: 4.0,
            supports_nvidia: true,
            supports_amd: true,
            supports_asic: true,
            supports_cpu: false,
            tuning_note: "Still memory-bound, but friendlier to older 4 GB cards than Ethash.",
            backend_note: "Backend path: CUDA/OpenCL GPU miners or Etchash-capable ASIC firmware.",
        },
        "kawpow" | "firopow" | "progpow" | "progpowz" => AlgorithmRule {
            algorithm: "ProgPow family",
            min_vram_gb: 3.0,
            supports_nvidia: true,
            supports_amd: true,
            supports_asic: false,
            supports_cpu: false,
            tuning_note: "Core-heavy algorithm family; higher overclocks tend to raise rejects and stale shares.",
            backend_note: "Backend path: GPU-only, typically modern CUDA or AMD/OpenCL miners.",
        },
        "autolykos" => AlgorithmRule {
            algorithm: "Autolykos",
            min_vram_gb: 3.0,
            supports_nvidia: true,
            supports_amd: true,
            supports_asic: false,
            supports_cpu: false,
            tuning_note: "Efficient on modern GPUs and strongly benefits from low-watt memory tuning.",
            backend_note: "Backend path: GPU-only, with mature NVIDIA and AMD miner support.",
        },
        "randomx" => AlgorithmRule {
            algorithm: "RandomX",
            min_vram_gb: 0.0,
            supports_nvidia: false,
            supports_amd: false,
            supports_asic: false,
            supports_cpu: true,
            tuning_note: "CPU-first and cache-sensitive; memory timings, huge pages, and thread pinning matter more than clocks.",
            backend_note: "Backend path: XMRig/SRBMiner-class CPU miners with huge pages and NUMA-aware tuning.",
        },
        "astrobwtv3" => AlgorithmRule {
            algorithm: "AstroBWTv3",
            min_vram_gb: 0.0,
            supports_nvidia: false,
            supports_amd: false,
            supports_asic: false,
            supports_cpu: true,
            tuning_note: "CPU-heavy DERO path where memory latency and stable all-core clocks dominate.",
            backend_note: "Backend path: CPU miners such as SRBMiner or dedicated AstroBWTv3 builds.",
        },
        "verushash" => AlgorithmRule {
            algorithm: "VerusHash",
            min_vram_gb: 0.0,
            supports_nvidia: false,
            supports_amd: false,
            supports_asic: false,
            supports_cpu: true,
            tuning_note: "CPU-friendly mixed workload that scales well with cache and thread affinity.",
            backend_note: "Backend path: CPU-oriented Verus miners with thread and affinity tuning.",
        },
        "ghostrider" => AlgorithmRule {
            algorithm: "Ghostrider",
            min_vram_gb: 0.0,
            supports_nvidia: false,
            supports_amd: false,
            supports_asic: false,
            supports_cpu: true,
            tuning_note: "CPU algorithm with wide variance across architectures; cache and branch behavior matter materially.",
            backend_note: "Backend path: CPU miners such as cpuminer-opt-class Ghostrider builds.",
        },
        "randomscash" => AlgorithmRule {
            algorithm: "Randomscash",
            min_vram_gb: 0.0,
            supports_nvidia: false,
            supports_amd: false,
            supports_asic: false,
            supports_cpu: true,
            tuning_note: "CPU-only path with strong sensitivity to memory timings and operating temperature.",
            backend_note: "Backend path: CPU miners with Randomscash support and stable per-thread tuning.",
        },
        "cryptonightturtle" => AlgorithmRule {
            algorithm: "CryptoNightTurtle",
            min_vram_gb: 0.0,
            supports_nvidia: false,
            supports_amd: false,
            supports_asic: false,
            supports_cpu: true,
            tuning_note: "Legacy CPU-oriented CryptoNight family workload where cache hit rate and pages matter.",
            backend_note: "Backend path: CPU miners with CryptoNightTurtle support and large-page tuning.",
        },
        "yespower" | "yescrypt" | "yescryptr16" | "minotaurx" | "randomarq" | "cryptonightupx"
        | "argon2idchukwa" => AlgorithmRule {
            algorithm: "CPU-first alt",
            min_vram_gb: 0.0,
            supports_nvidia: false,
            supports_amd: false,
            supports_asic: false,
            supports_cpu: true,
            tuning_note: "CPU-first alt algorithm where cache behavior, memory timings, and thread affinity matter more than raw clocks.",
            backend_note: "Backend path: cpuminer-opt, SRBMiner, or algorithm-specific CPU miner builds.",
        },
        "verthash" => AlgorithmRule {
            algorithm: "Verthash",
            min_vram_gb: 2.0,
            supports_nvidia: true,
            supports_amd: true,
            supports_asic: false,
            supports_cpu: false,
            tuning_note: "Requires the Verthash data file and enough local VRAM headroom.",
            backend_note: "Backend path: GPU miners only, with a local Verthash data file dependency.",
        },
        "cuckatoo32" => AlgorithmRule {
            algorithm: "Cuckatoo32",
            min_vram_gb: 8.0,
            supports_nvidia: true,
            supports_amd: false,
            supports_asic: true,
            supports_cpu: false,
            tuning_note: "Memory footprint is large; 8 GB cards are the realistic floor.",
            backend_note: "Backend path: NVIDIA CUDA miners or dedicated GRIN-style ASICs.",
        },
        "cuckaroo29" | "cuckoocycle" => AlgorithmRule {
            algorithm: "Cuckoo family",
            min_vram_gb: 6.0,
            supports_nvidia: true,
            supports_amd: true,
            supports_asic: false,
            supports_cpu: false,
            tuning_note: "Graph-based mining; memory stability matters more than raw wattage.",
            backend_note: "Backend path: niche CUDA/OpenCL miners with thinner benchmark coverage than mainstream algos.",
        },
        "beamhashiii" => AlgorithmRule {
            algorithm: "BeamHashIII",
            min_vram_gb: 6.0,
            supports_nvidia: true,
            supports_amd: true,
            supports_asic: false,
            supports_cpu: false,
            tuning_note: "NVIDIA and AMD both work, but low-memory cards get squeezed.",
            backend_note: "Backend path: modern CUDA/OpenCL miners only.",
        },
        "cortex" => AlgorithmRule {
            algorithm: "Cortex",
            min_vram_gb: 8.0,
            supports_nvidia: true,
            supports_amd: true,
            supports_asic: false,
            supports_cpu: false,
            tuning_note: "High memory pressure and heavier core load than Ethash-class algorithms.",
            backend_note: "Backend path: GMiner/lolMiner-style GPU stack with large-DAG headroom.",
        },
        "fishhash" => AlgorithmRule {
            algorithm: "FishHash",
            min_vram_gb: 8.0,
            supports_nvidia: true,
            supports_amd: true,
            supports_asic: true,
            supports_cpu: false,
            tuning_note: "Iron Fish still rewards stable memory clocks more than brute-force power.",
            backend_note: "Backend path: modern GPU miners or FishHash-capable ASIC firmware.",
        },
        "xelishashv3" => AlgorithmRule {
            algorithm: "XelisHashv3",
            min_vram_gb: 4.0,
            supports_nvidia: true,
            supports_amd: true,
            supports_asic: false,
            supports_cpu: true,
            tuning_note: "Modern niche algo with both GPU and emerging high-core CPU benchmark coverage.",
            backend_note: "Backend path: CUDA/OpenCL miners on GPU or validated Xelis-capable CPU miners on high-core CPUs.",
        },
        "nexapow" | "dynexsolve" | "qhash" | "sha3x" | "abelhash" | "karlsenhashv2" => AlgorithmRule {
            algorithm: "Modern alt algorithm",
            min_vram_gb: 4.0,
            supports_nvidia: true,
            supports_amd: true,
            supports_asic: false,
            supports_cpu: false,
            tuning_note: "Modern niche algo; miner support and clocks vary more than on mainstream chains.",
            backend_note: "Backend path: exact CUDA/OpenCL/Vulkan support depends on miner release cadence.",
        },
        "sha256" => AlgorithmRule {
            algorithm: "SHA256",
            min_vram_gb: 0.0,
            supports_nvidia: true,
            supports_amd: true,
            supports_asic: true,
            supports_cpu: true,
            tuning_note: "SHA256 is technically mineable on CPU and GPU, but those software paths are massively uncompetitive against ASICs in 2026.",
            backend_note: "Backend path: CPU/GPU SHA256 miners exist, but the realistic production path is still ASIC firmware or a specialized pool stack.",
        },
        "discoveryproxy" => AlgorithmRule {
            algorithm: "DiscoveryProxy",
            min_vram_gb: 0.0,
            supports_nvidia: true,
            supports_amd: true,
            supports_asic: true,
            supports_cpu: true,
            tuning_note: "Synthetic discovery benchmark used to rank assets that have market data but no validated mining telemetry.",
            backend_note: "Backend path: minefit inference only; this is a market-derived proxy, not a validated miner stack.",
        },
        "scrypt" | "kheavyhash" => AlgorithmRule {
            algorithm: "ASIC-first",
            min_vram_gb: 0.0,
            supports_nvidia: false,
            supports_amd: false,
            supports_asic: true,
            supports_cpu: false,
            tuning_note: "These are realistically ASIC-first algorithms in 2026.",
            backend_note: "Backend path: stock or aftermarket ASIC firmware, not general-purpose GPU miners.",
        },
        _ => AlgorithmRule {
            algorithm: "Generic",
            min_vram_gb: 4.0,
            supports_nvidia: true,
            supports_amd: true,
            supports_asic: false,
            supports_cpu: false,
            tuning_note: "No dedicated benchmark yet; treat compatibility as directional.",
            backend_note: "Backend path: no validated miner stack yet; treat unsupported matches cautiously.",
        },
    }
}

fn resolve_detected_gpu_profile(specs: &SystemSpecs) -> Option<MiningRigProfile> {
    if specs.gpus.is_empty() {
        return None;
    }

    let gpu_name = specs
        .gpu_name
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if gpu_name.contains("rtx 4060") {
        return Some(rtx_4060_profile());
    }
    if gpu_name.contains("rtx 4090") {
        return Some(rtx_4090_profile());
    }
    if gpu_name.contains("6700 xt") {
        return Some(rx_6700_xt_profile());
    }

    Some(generic_gpu_profile(specs))
}

fn resolve_detected_cpu_profile(specs: &SystemSpecs) -> MiningRigProfile {
    let cpu_name = specs.cpu_name.to_ascii_lowercase();
    if cpu_name.contains("ryzen 5 7600") {
        return ryzen_5_7600_profile();
    }
    if cpu_name.contains("7950x3d") {
        return ryzen_9_7950x3d_profile();
    }
    if cpu_name.contains("epyc 9754") || cpu_name.contains("9754") {
        return amd_epyc_9754_profile();
    }
    if cpu_name.contains("14700") {
        return intel_i7_14700f_proxy_profile();
    }

    generic_cpu_profile(specs)
}

fn rtx_4060_profile() -> MiningRigProfile {
    MiningRigProfile {
        id: "rtx-4060",
        name: "NVIDIA RTX 4060 8GB",
        vendor: "NVIDIA",
        kind: RigKind::Gpu,
        memory_gb: Some(8.0),
        source: "Hashrate.no benchmark pages",
        tuning_summary: "Medium tuned profile using Rigel/lolMiner-style memory OC and low core locks.",
        fallback_power_watts: 90.0,
        benchmarks: vec![
            benchmark("DiscoveryProxy", 1.0, 78.0, 0.40, "minefit market proxy", "synthetic GPU discovery baseline"),
            benchmark("SHA256", gh(1.85), 88.0, 0.20, "cgminer CUDA proxy estimate", "software SHA256 theoretical 4060 path"),
            benchmark("Ethash", mh(33.22), 72.0, 0.45, "lolMiner", "1005 core lock, +2500 mem"),
            benchmark("Etchash", mh(33.42), 72.0, 0.45, "lolMiner", "1005 core lock, +2500 mem"),
            benchmark("Autolykos", mh(68.67), 42.0, 0.30, "Rigel", "memory-lean low watt profile"),
            benchmark("KawPow", mh(17.92), 91.0, 0.85, "Rigel", "balanced core-heavy tuning"),
            benchmark("ProgPow", mh(16.11), 88.0, 0.90, "Rigel", "balanced ProgPow tuning"),
            benchmark("ProgPowZ", mh(16.11), 88.0, 0.90, "Rigel", "balanced ProgPow tuning"),
            benchmark("FiroPow", mh(16.11), 90.0, 0.95, "Rigel", "balanced ProgPow-family tuning"),
            benchmark("BeamHashIII", hs(23.55), 89.0, 0.70, "lolMiner", "BeamHash medium OC"),
            benchmark("Cuckatoo32", 0.46, 89.0, 0.60, "lolMiner", "GRIN medium profile"),
            benchmark("Cuckaroo29", 4.07, 65.0, 0.55, "lolMiner", "2190 core, +2000 mem"),
            benchmark("CuckooCycle", 8.10, 118.0, 0.65, "lolMiner", "inferred from adjacent NVIDIA benchmarks"),
            benchmark("FishHash", mh(22.36), 68.0, 0.55, "lolMiner", "900 core, +2000 mem, 115W PL"),
            benchmark("DynexSolve", kh(3.55), 63.0, 0.75, "SRBMiner", "medium Dynex OC"),
            benchmark("Xelishashv3", kh(4.76), 52.0, 0.40, "Rigel", "low-voltage Xelis tune"),
            benchmark("NexaPow", mh(58.30), 89.0, 0.90, "Rigel", "2350 core lock, 5000 mem"),
            benchmark("Qhash", gh(0.89), 61.0, 0.50, "Rigel", "Qubitcoin medium benchmark"),
            benchmark("SHA3x", gh(0.29), 63.0, 0.45, "Rigel", "Tari SHA3x medium benchmark"),
            benchmark("AbelHash", mh(33.39), 70.0, 0.55, "Rigel", "900 core lock, +2500 mem"),
            benchmark("KarlsenHashV2", mh(22.36), 68.0, 0.55, "lolMiner", "900 core, +2000 mem"),
            benchmark("Cortex", 2.90, 75.0, 1.10, "GMiner", "inferred from 3060/3070 CTXC ladder"),
        ],
    }
}

fn rtx_4090_profile() -> MiningRigProfile {
    MiningRigProfile {
        id: "rtx-4090",
        name: "NVIDIA RTX 4090 24GB",
        vendor: "NVIDIA",
        kind: RigKind::Gpu,
        memory_gb: Some(24.0),
        source: "Hashrate.no benchmark pages",
        tuning_summary: "High-end Ada profile with capped power limits and large memory offsets.",
        fallback_power_watts: 280.0,
        benchmarks: vec![
            benchmark("DiscoveryProxy", 2.8, 240.0, 0.45, "minefit market proxy", "synthetic high-end GPU discovery baseline"),
            benchmark("SHA256", gh(9.80), 285.0, 0.20, "cgminer CUDA proxy estimate", "software SHA256 theoretical 4090 path"),
            benchmark("Ethash", mh(127.0), 249.0, 0.40, "lolMiner", "+2500 mem, 250W PL"),
            benchmark("Etchash", mh(127.0), 249.0, 0.40, "lolMiner", "+2500 mem, 250W PL"),
            benchmark("Autolykos", mh(285.0), 131.0, 0.30, "Rigel", "high-efficiency ERG tune"),
            benchmark("KawPow", mh(64.35), 275.0, 0.95, "Rigel", "aggressive KawPow core tune"),
            benchmark("FiroPow", mh(61.72), 282.0, 1.05, "Rigel", "aggressive ProgPow-family tune"),
            benchmark("FishHash", mh(73.06), 442.0, 0.80, "lolMiner", "IRON dual-mining medium benchmark"),
            benchmark("Cortex", 7.44, 266.0, 1.10, "GMiner", "CTXC benchmark table"),
            benchmark("CuckooCycle", 17.60, 268.0, 0.70, "lolMiner", "AE benchmark table"),
            benchmark("Xelishashv3", kh(32.54), 263.0, 0.55, "Rigel", "XEL medium benchmark"),
            benchmark("NexaPow", mh(210.0), 240.0, 0.95, "Rigel", "inferred from 4090/4060 class scaling"),
            benchmark("Qhash", gh(3.20), 210.0, 0.55, "Rigel", "inferred from Ada class scaling"),
            benchmark("KarlsenHashV2", mh(82.58), 240.0, 0.55, "lolMiner", "KLS benchmark table"),
            benchmark("Cuckaroo29", 10.90, 180.0, 0.60, "lolMiner", "inferred from 4060/4090 ratio"),
        ],
    }
}

fn rx_6700_xt_profile() -> MiningRigProfile {
    MiningRigProfile {
        id: "rx-6700-xt",
        name: "AMD RX 6700 XT 12GB",
        vendor: "AMD",
        kind: RigKind::Gpu,
        memory_gb: Some(12.0),
        source: "Hashrate.no benchmark pages",
        tuning_summary: "Undervolted RDNA2 profile using low core lock and Samsung memory timings.",
        fallback_power_watts: 140.0,
        benchmarks: vec![
            benchmark("DiscoveryProxy", 1.35, 125.0, 0.45, "minefit market proxy", "synthetic AMD GPU discovery baseline"),
            benchmark("SHA256", gh(2.60), 138.0, 0.25, "OpenCL SHA256 proxy estimate", "software SHA256 theoretical RDNA2 path"),
            benchmark("Ethash", mh(46.95), 98.0, 0.35, "TeamRedMiner", "1250 core, 675 mv, mem lock 1050"),
            benchmark("Etchash", mh(46.95), 98.0, 0.35, "TeamRedMiner", "1250 core, 675 mv, mem lock 1050"),
            benchmark("Autolykos", mh(106.41), 90.0, 0.30, "TeamRedMiner", "ERG medium OC"),
            benchmark("KawPow", mh(27.51), 131.0, 0.85, "TeamRedMiner", "RDNA2 KawPow tune"),
            benchmark("FiroPow", mh(27.10), 158.0, 0.95, "TeamRedMiner", "FiroPow benchmark table"),
            benchmark("KarlsenHashV2", mh(33.62), 162.0, 0.65, "lolMiner", "KLS benchmark table"),
            benchmark("NexaPow", mh(88.0), 145.0, 0.90, "BzMiner", "inferred from 4060 Ti class scaling"),
            benchmark("Xelishashv3", kh(9.82), 147.0, 0.55, "SRBMiner", "XEL benchmark table"),
            benchmark("Cortex", 2.27, 167.0, 1.20, "GMiner", "CTXC benchmark table"),
            benchmark("Cuckaroo29", 4.88, 93.0, 0.55, "lolMiner", "Tari C29 benchmark table"),
            benchmark("SHA3x", gh(0.27), 54.0, 0.45, "lolMiner", "Tari SHA3x benchmark table"),
            benchmark("Qhash", gh(1.05), 88.0, 0.45, "Rigel", "inferred from AMD/ETC ratio"),
        ],
    }
}

fn antminer_s19_pro_profile() -> MiningRigProfile {
    MiningRigProfile {
        id: "s19-pro",
        name: "Bitmain Antminer S19 Pro 110T",
        vendor: "Bitmain",
        kind: RigKind::Asic,
        memory_gb: None,
        source: "BITMAIN support specifications",
        tuning_summary: "Factory SHA256 profile.",
        fallback_power_watts: 3250.0,
        benchmarks: vec![benchmark(
            "SHA256",
            th(110.0),
            3250.0,
            0.20,
            "Bitmain stock firmware",
            "Factory 110 TH/s profile",
        )],
    }
}

fn antminer_l9_profile() -> MiningRigProfile {
    MiningRigProfile {
        id: "l9",
        name: "Bitmain Antminer L9 16G",
        vendor: "Bitmain",
        kind: RigKind::Asic,
        memory_gb: None,
        source: "BITMAIN support specifications",
        tuning_summary: "Factory Scrypt profile.",
        fallback_power_watts: 3360.0,
        benchmarks: vec![benchmark(
            "Scrypt",
            gh(16.0),
            3360.0,
            0.20,
            "Bitmain stock firmware",
            "Factory 16 GH/s profile",
        )],
    }
}

fn iceriver_ks5m_profile() -> MiningRigProfile {
    MiningRigProfile {
        id: "ks5m",
        name: "IceRiver KS5M 15TH",
        vendor: "IceRiver",
        kind: RigKind::Asic,
        memory_gb: None,
        source: "IceRiver official product page",
        tuning_summary: "Factory KHeavyHash profile.",
        fallback_power_watts: 3400.0,
        benchmarks: vec![benchmark(
            "KHeavyHash",
            th(15.0),
            3400.0,
            0.20,
            "IceRiver stock firmware",
            "Factory 15 TH/s profile",
        )],
    }
}

fn intel_i7_14700f_proxy_profile() -> MiningRigProfile {
    MiningRigProfile {
        id: "i7-14700f",
        name: "Intel Core i7-14700F",
        vendor: "Intel",
        kind: RigKind::Cpu,
        memory_gb: None,
        source: "Hashrate.no i9-13900K RandomX benchmark pages used as a conservative proxy",
        tuning_summary: "Proxy CPU profile for 14th-gen Intel desktop chips when no exact validated 14700F page is available.",
        fallback_power_watts: 125.0,
        benchmarks: vec![
            benchmark(
                "DiscoveryProxy",
                0.65,
                105.0,
                0.50,
                "minefit market proxy",
                "synthetic desktop CPU discovery baseline",
            ),
            benchmark(
                "RandomX",
                kh(13.48),
                125.0,
                0.40,
                "Hashrate.no validated Intel RandomX proxy",
                "13900K-class RandomX reference applied conservatively",
            ),
            benchmark(
                "SHA256",
                mh(92.0),
                125.0,
                0.25,
                "cpuminer SHA256 estimate",
                "software SHA256 theoretical desktop CPU path",
            ),
        ],
    }
}

fn ryzen_5_7600_profile() -> MiningRigProfile {
    MiningRigProfile {
        id: "ryzen-5-7600",
        name: "AMD Ryzen 5 7600",
        vendor: "AMD",
        kind: RigKind::Cpu,
        memory_gb: None,
        source: "Hashrate.no CPU benchmark pages",
        tuning_summary: "Desktop CPU reference tuned around efficient RandomX, AstroBWTv3, and VerusHash operation.",
        fallback_power_watts: 65.0,
        benchmarks: vec![
            benchmark("DiscoveryProxy", 0.50, 58.0, 0.45, "minefit market proxy", "synthetic Ryzen 5 discovery baseline"),
            benchmark("SHA256", mh(55.0), 65.0, 0.25, "cpuminer SHA256 estimate", "software SHA256 theoretical Ryzen 5 path"),
            benchmark("RandomX", kh(8.24), 65.0, 0.35, "Hashrate.no verified CPU benchmark", "desktop RandomX reference"),
            benchmark("AstroBWTv3", kh(15.30), 65.0, 0.45, "Hashrate.no verified CPU benchmark", "DERO-oriented CPU reference"),
            benchmark("VerusHash", mh(21.43), 65.0, 0.40, "Hashrate.no verified CPU benchmark", "balanced VerusHash desktop reference"),
        ],
    }
}

fn ryzen_9_7950x3d_profile() -> MiningRigProfile {
    MiningRigProfile {
        id: "ryzen-9-7950x3d",
        name: "AMD Ryzen 9 7950X3D",
        vendor: "AMD",
        kind: RigKind::Cpu,
        memory_gb: None,
        source: "Hashrate.no CPU benchmark pages",
        tuning_summary: "High-cache desktop CPU profile spanning RandomX, AstroBWTv3, VerusHash, Ghostrider, Randomscash, and CryptoNightTurtle coverage.",
        fallback_power_watts: 120.0,
        benchmarks: vec![
            benchmark("DiscoveryProxy", 0.95, 110.0, 0.45, "minefit market proxy", "synthetic X3D discovery baseline"),
            benchmark("SHA256", mh(148.0), 120.0, 0.25, "cpuminer SHA256 estimate", "software SHA256 theoretical X3D path"),
            benchmark("RandomX", kh(22.50), 120.0, 0.35, "Hashrate.no verified CPU benchmark", "7950X3D RandomX reference"),
            benchmark("AstroBWTv3", kh(35.19), 115.0, 0.45, "Hashrate.no verified CPU benchmark", "7950X3D DERO reference"),
            benchmark("VerusHash", mh(58.94), 120.0, 0.40, "Hashrate.no verified CPU benchmark", "7950X3D VerusHash reference"),
            benchmark("Ghostrider", hs(6460.0), 120.0, 0.55, "Hashrate.no verified CPU benchmark", "7950X3D Ghostrider reference"),
            benchmark("Randomscash", kh(19.10), 130.0, 0.50, "Hashrate.no verified CPU benchmark", "SCASH tuned CPU reference"),
            benchmark("CryptoNightTurtle", kh(32.71), 102.0, 0.45, "Hashrate.no verified CPU benchmark", "CryptoNightTurtle tuned CPU reference"),
        ],
    }
}

fn amd_epyc_9754_profile() -> MiningRigProfile {
    MiningRigProfile {
        id: "epyc-9754",
        name: "AMD EPYC 9754",
        vendor: "AMD",
        kind: RigKind::Cpu,
        memory_gb: None,
        source: "Hashrate.no CPU benchmark pages",
        tuning_summary: "Dense-core server CPU profile for XelisHashv3 plus high-throughput RandomX, AstroBWTv3, and VerusHash workloads.",
        fallback_power_watts: 360.0,
        benchmarks: vec![
            benchmark("DiscoveryProxy", 2.2, 320.0, 0.45, "minefit market proxy", "synthetic server CPU discovery baseline"),
            benchmark("SHA256", mh(430.0), 360.0, 0.25, "cpuminer SHA256 estimate", "software SHA256 theoretical server CPU path"),
            benchmark("RandomX", kh(65.77), 360.0, 0.35, "Hashrate.no verified CPU benchmark", "server-class RandomX reference"),
            benchmark("AstroBWTv3", kh(176.0), 360.0, 0.45, "Hashrate.no verified CPU benchmark", "server-class DERO reference"),
            benchmark("VerusHash", mh(272.16), 360.0, 0.40, "Hashrate.no verified CPU benchmark", "server-class VerusHash reference"),
            benchmark("XelisHashv3", kh(94.50), 400.0, 0.50, "Hashrate.no verified CPU benchmark", "server-class Xelis CPU reference"),
        ],
    }
}

fn generic_gpu_profile(specs: &SystemSpecs) -> MiningRigProfile {
    MiningRigProfile {
        id: "generic-gpu",
        name: "Generic GPU Reference",
        vendor: "Generic",
        kind: RigKind::Gpu,
        memory_gb: specs.gpu_vram_gb,
        source: "WhatToMine reference rig fallback",
        tuning_summary: "Fallback profile when no curated benchmark matches the detected GPU.",
        fallback_power_watts: 140.0,
        benchmarks: vec![
            benchmark("DiscoveryProxy", 1.0, 120.0, 0.50, "minefit market proxy", "synthetic generic GPU discovery baseline"),
            benchmark(
                "SHA256",
                gh(1.20),
                140.0,
                0.30,
                "generic OpenCL/CUDA estimate",
                "software SHA256 fallback path",
            ),
        ],
    }
}

fn generic_cpu_profile(_specs: &SystemSpecs) -> MiningRigProfile {
    MiningRigProfile {
        id: "generic-cpu",
        name: "Generic CPU Reference",
        vendor: "CPU",
        kind: RigKind::Cpu,
        memory_gb: None,
        source: "Detected CPU fallback",
        tuning_summary: "Fallback profile when no curated CPU benchmark matches the detected processor.",
        fallback_power_watts: 95.0,
        benchmarks: vec![
            benchmark("DiscoveryProxy", 0.45, 88.0, 0.50, "minefit market proxy", "synthetic generic CPU discovery baseline"),
            benchmark(
                "SHA256",
                mh(40.0),
                95.0,
                0.30,
                "generic cpuminer estimate",
                "software SHA256 fallback path",
            ),
        ],
    }
}

fn benchmark(
    algorithm: &'static str,
    hashrate_hs: f64,
    power_watts: f64,
    reject_rate_pct: f64,
    miner: &'static str,
    tuning: &'static str,
) -> AlgorithmBenchmark {
    AlgorithmBenchmark {
        algorithm,
        hashrate_hs,
        power_watts,
        reject_rate_pct,
        miner,
        tuning,
    }
}

fn normalize_id(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

fn hs(value: f64) -> f64 {
    value
}

fn kh(value: f64) -> f64 {
    value * 1_000.0
}

fn mh(value: f64) -> f64 {
    value * 1_000_000.0
}

fn gh(value: f64) -> f64 {
    value * 1_000_000_000.0
}

fn th(value: f64) -> f64 {
    value * 1_000_000_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_profiles_keep_sha256_software_paths() {
        assert!(rtx_4060_profile().benchmark_for("SHA256").is_some());
        assert!(intel_i7_14700f_proxy_profile().benchmark_for("SHA256").is_some());
    }
}
