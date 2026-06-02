use std::{
    collections::hash_map::DefaultHasher,
    env,
    fs::{self, File},
    hash::{Hash, Hasher},
    io::{self, BufRead, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    process,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use anton::{
    board::Board,
    evaluation::{EVAL_PARAMS, EvalValue, FeatureVectorTrace, evaluate, trace::FEATURE_COUNT},
    movegen::MoveGenerator,
};

const CACHE_MAGIC: &[u8; 8] = b"ANTTUNE1";
const CACHE_VERSION: u32 = 2;
const DEFAULT_VALIDATION_PERCENT: u8 = 10;
const DEFAULT_SEED: u64 = 0;
const DEFAULT_K_MIN: f64 = 0.0001;
const DEFAULT_K_MAX: f64 = 0.01;
const DEFAULT_K_STEPS: usize = 100;
const DEFAULT_LR_SCAN_EPOCHS: usize = 5;
const DEFAULT_TUNE_MAX_EPOCHS: usize = 500;
const DEFAULT_PATIENCE: usize = 10;
const DEFAULT_MIN_DELTA: f64 = 0.0000001;
const DEFAULT_ADAGRAD_EPS: f64 = 1e-8;
const DEFAULT_BATCH_SIZE: usize = 65_536;
const DEFAULT_LR_LIST: &[f64] = &[0.1, 0.3, 1.0, 3.0, 10.0, 30.0, 100.0];
static TERMINATE_AFTER_EPOCH: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WeightCategory {
    name: &'static str,
    shape: (usize, usize),
}

const WEIGHT_LAYOUT: &[WeightCategory] = &[
    WeightCategory {
        name: "material mg",
        shape: (1, 6),
    },
    WeightCategory {
        name: "material eg",
        shape: (1, 6),
    },
    WeightCategory {
        name: "pawn psqt mg",
        shape: (8, 8),
    },
    WeightCategory {
        name: "knight psqt mg",
        shape: (8, 8),
    },
    WeightCategory {
        name: "bishop psqt mg",
        shape: (8, 8),
    },
    WeightCategory {
        name: "rook psqt mg",
        shape: (8, 8),
    },
    WeightCategory {
        name: "queen psqt mg",
        shape: (8, 8),
    },
    WeightCategory {
        name: "king psqt mg",
        shape: (8, 8),
    },
    WeightCategory {
        name: "pawn psqt eg",
        shape: (8, 8),
    },
    WeightCategory {
        name: "knight psqt eg",
        shape: (8, 8),
    },
    WeightCategory {
        name: "bishop psqt eg",
        shape: (8, 8),
    },
    WeightCategory {
        name: "rook psqt eg",
        shape: (8, 8),
    },
    WeightCategory {
        name: "queen psqt eg",
        shape: (8, 8),
    },
    WeightCategory {
        name: "king psqt eg",
        shape: (8, 8),
    },
    WeightCategory {
        name: "isolated pawn mg",
        shape: (1, 8),
    },
    WeightCategory {
        name: "isolated pawn eg",
        shape: (1, 8),
    },
    WeightCategory {
        name: "doubled pawn mg",
        shape: (1, 8),
    },
    WeightCategory {
        name: "doubled pawn eg",
        shape: (1, 8),
    },
    WeightCategory {
        name: "backward pawn mg",
        shape: (1, 8),
    },
    WeightCategory {
        name: "backward pawn eg",
        shape: (1, 8),
    },
    WeightCategory {
        name: "connected pawn mg",
        shape: (8, 8),
    },
    WeightCategory {
        name: "connected pawn eg",
        shape: (8, 8),
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Split {
    Train,
    Validation,
}

#[derive(Clone, Debug, PartialEq)]
struct Sample {
    split: Split,
    result: f32,
    source_eval: i32,
    features: Vec<(u16, f32)>,
}

#[derive(Clone, Debug, PartialEq)]
struct DatasetRow {
    fen: String,
    result: f32,
    source_eval: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct DatasetStats {
    valid: u64,
    skipped: u64,
    train: u64,
    validation: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct SkipStats {
    parse: u64,
    position: u64,
    examples: Vec<String>,
}

impl SkipStats {
    fn record_parse(&mut self, line_idx: u64, err: String) {
        self.parse += 1;
        self.record_example(line_idx, err);
    }

    fn record_position(&mut self, line_idx: u64, err: String) {
        self.position += 1;
        self.record_example(line_idx, err);
    }

    fn record_example(&mut self, line_idx: u64, err: String) {
        if self.examples.len() < 5 {
            self.examples.push(format!("line {line_idx}: {err}"));
        }
    }
}

#[derive(Clone, Debug)]
struct CacheHeader {
    dataset_path: String,
    dataset_len: u64,
    dataset_modified_secs: u64,
    feature_count: u32,
    validation_percent: u8,
    seed: u64,
    stats: DatasetStats,
}

#[derive(Clone, Debug)]
struct CommonOptions {
    dataset: PathBuf,
    cache: PathBuf,
    output: PathBuf,
    validation_percent: u8,
    seed: u64,
    batch_size: usize,
}

#[derive(Clone, Debug)]
enum Command {
    FindK {
        common: CommonOptions,
        k_min: f64,
        k_max: f64,
        k_steps: usize,
    },
    ScanLr {
        common: CommonOptions,
        k: f64,
        learning_rates: Vec<f64>,
        epochs: usize,
        eps: f64,
        train: TrainMask,
    },
    Tune {
        common: CommonOptions,
        k: f64,
        learning_rate: f64,
        max_epochs: usize,
        patience: usize,
        min_delta: f64,
        eps: f64,
        train: TrainMask,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Mse {
    train: f64,
    validation: f64,
}

#[derive(Clone, Debug)]
struct TuneResult {
    weights: Vec<f64>,
    best_epoch: usize,
    best_train_mse: f64,
    best_validation_mse: f64,
    epochs_run: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TrainMask {
    train: Vec<bool>,
}

impl TrainMask {
    fn none() -> Self {
        Self {
            train: vec![false; FEATURE_COUNT],
        }
    }

    fn parse(spec: Option<String>) -> Result<Self, String> {
        let Some(spec) = spec else {
            return Ok(Self::none());
        };
        let mut mask = Self::none();
        for raw_part in spec.split(',') {
            let part = raw_part.trim();
            if part.is_empty() {
                continue;
            }
            mask.train_part(part)?;
        }
        Ok(mask)
    }

    fn train_part(&mut self, part: &str) -> Result<(), String> {
        if part == "all" {
            self.train.fill(true);
            return Ok(());
        }

        if let Some((start, end)) = part.split_once("..") {
            let start = parse_weight_idx(start.trim())?;
            let end = parse_weight_idx(end.trim())?;
            if start > end {
                return Err(format!("invalid --train-weights range {part}"));
            }
            for idx in start..=end {
                self.train_idx(idx)?;
            }
            return Ok(());
        }

        if let Ok(idx) = part.parse::<usize>() {
            return self.train_idx(idx);
        }

        let Some((offset, category)) = weight_category_by_name(part) else {
            return Err(format!("unknown --train-weights entry {part:?}"));
        };
        let len = category.shape.0 * category.shape.1;
        for idx in offset..offset + len {
            self.train[idx] = true;
        }
        Ok(())
    }

    fn train_idx(&mut self, idx: usize) -> Result<(), String> {
        if idx >= self.train.len() {
            return Err(format!(
                "--train-weights index {idx} out of range 0..{}",
                self.train.len() - 1
            ));
        }
        self.train[idx] = true;
        Ok(())
    }

    fn should_train(&self, idx: usize) -> bool {
        self.train[idx]
    }

    fn count(&self) -> usize {
        self.train.iter().filter(|&&train| train).count()
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    install_termination_handler();
    let command = parse_args(env::args().skip(1).collect())?;

    match command {
        Command::FindK {
            common,
            k_min,
            k_max,
            k_steps,
        } => {
            let header = ensure_cache(&common)?;
            let weights = initial_weights();
            let result = find_best_k(
                &common.cache,
                &weights,
                k_min,
                k_max,
                k_steps,
                common.batch_size,
            )?;
            let report = format!(
                "best_k {:.12}\ntrain_mse {:.12}\nvalidation_mse {:.12}\nvalid {}\nskipped {}\ntrain_samples {}\nvalidation_samples {}\n",
                result.0,
                result.1.train,
                result.1.validation,
                header.stats.valid,
                header.stats.skipped,
                header.stats.train,
                header.stats.validation,
            );
            print!("{report}");
            fs::write(&common.output, report)
                .map_err(|err| format!("failed to write report: {err}"))?;
        }
        Command::ScanLr {
            common,
            k,
            learning_rates,
            epochs,
            eps,
            train,
        } => {
            ensure_cache(&common)?;
            let mut rows = Vec::new();
            let mut report = String::new();
            let line = format!("trainable_weights {}\n", train.count());
            print!("{line}");
            report.push_str(&line);
            for lr in learning_rates {
                let mut weights = initial_weights();
                let mut acc = vec![0.0; FEATURE_COUNT];
                let mut best = Mse {
                    train: f64::INFINITY,
                    validation: f64::INFINITY,
                };
                println!("learning_rate {lr:.6}");
                report.push_str(&format!("learning_rate {lr:.6}\n"));
                for epoch in 1..=epochs {
                    train_one_epoch(
                        &common.cache,
                        &mut weights,
                        &mut acc,
                        k,
                        lr,
                        eps,
                        common.batch_size,
                        &train,
                    )?;
                    let mse = compute_mse(&common.cache, &weights, k, common.batch_size)?;
                    if mse.validation < best.validation {
                        best = mse;
                    }
                    let line = format!(
                        "epoch {epoch} train_mse {:.12} validation_mse {:.12}\n",
                        mse.train, mse.validation
                    );
                    print!("{line}");
                    report.push_str(&line);
                }
                rows.push((lr, best));
                report.push('\n');
            }
            rows.sort_by(|a, b| a.1.validation.total_cmp(&b.1.validation));
            report.push_str("summary\n");
            println!("summary");
            for (lr, mse) in rows {
                let line = format!(
                    "learning_rate {:.6} best_train_mse {:.12} best_validation_mse {:.12}\n",
                    lr, mse.train, mse.validation
                );
                print!("{line}");
                report.push_str(&line);
            }
            fs::write(&common.output, report)
                .map_err(|err| format!("failed to write report: {err}"))?;
        }
        Command::Tune {
            common,
            k,
            learning_rate,
            max_epochs,
            patience,
            min_delta,
            eps,
            train,
        } => {
            let header = ensure_cache(&common)?;
            let result = tune(
                &common.cache,
                k,
                learning_rate,
                max_epochs,
                patience,
                min_delta,
                eps,
                common.batch_size,
                &train,
            )?;
            write_weights(
                &common.output,
                &result,
                &header,
                k,
                learning_rate,
                max_epochs,
                patience,
                min_delta,
                train.count(),
            )?;
            println!(
                "wrote {} best_epoch {} train_mse {:.12} validation_mse {:.12}",
                common.output.display(),
                result.best_epoch,
                result.best_train_mse,
                result.best_validation_mse
            );
        }
    }

    Ok(())
}

#[cfg(unix)]
fn install_termination_handler() {
    const SIGINT: i32 = 2;
    const SIGTERM: i32 = 15;

    unsafe extern "C" {
        fn signal(signum: i32, handler: extern "C" fn(i32)) -> usize;
    }

    extern "C" fn handler(_signum: i32) {
        TERMINATE_AFTER_EPOCH.store(true, Ordering::SeqCst);
    }

    unsafe {
        signal(SIGINT, handler);
        signal(SIGTERM, handler);
    }
}

#[cfg(not(unix))]
fn install_termination_handler() {}

fn termination_requested() -> bool {
    TERMINATE_AFTER_EPOCH.load(Ordering::SeqCst)
}

fn parse_args(args: Vec<String>) -> Result<Command, String> {
    let Some(mode) = args.first() else {
        return Err(usage());
    };
    let mut parser = ArgParser::new(args[1..].to_vec());

    match mode.as_str() {
        "find-k" => {
            let dataset = parser.path("--dataset")?;
            let output = parser
                .path_opt("--output")
                .unwrap_or_else(|| PathBuf::from("k_report.txt"));
            let common = common_options(&mut parser, dataset, output)?;
            let k_min = parser.f64_opt("--k-min")?.unwrap_or(DEFAULT_K_MIN);
            let k_max = parser.f64_opt("--k-max")?.unwrap_or(DEFAULT_K_MAX);
            let k_steps = parser.usize_opt("--k-steps")?.unwrap_or(DEFAULT_K_STEPS);
            parser.finish()?;
            if k_min <= 0.0 || k_max <= k_min || k_steps == 0 {
                return Err("invalid K search range".to_string());
            }
            Ok(Command::FindK {
                common,
                k_min,
                k_max,
                k_steps,
            })
        }
        "scan-lr" => {
            let dataset = parser.path("--dataset")?;
            let output = parser
                .path_opt("--output")
                .unwrap_or_else(|| PathBuf::from("lr_report.txt"));
            let common = common_options(&mut parser, dataset, output)?;
            let k = parser.f64("--k")?;
            let learning_rates = parser
                .f64_list_opt("--learning-rates")?
                .unwrap_or_else(|| DEFAULT_LR_LIST.to_vec());
            let epochs = parser
                .usize_opt("--epochs")?
                .unwrap_or(DEFAULT_LR_SCAN_EPOCHS);
            let eps = parser
                .f64_opt("--adagrad-eps")?
                .unwrap_or(DEFAULT_ADAGRAD_EPS);
            let train = TrainMask::parse(parser.take("--train-weights"))?;
            parser.finish()?;
            Ok(Command::ScanLr {
                common,
                k,
                learning_rates,
                epochs,
                eps,
                train,
            })
        }
        "tune" => {
            let dataset = parser.path("--dataset")?;
            let output = parser
                .path_opt("--output")
                .unwrap_or_else(|| PathBuf::from("tuned_weights.rs"));
            let common = common_options(&mut parser, dataset, output)?;
            let k = parser.f64("--k")?;
            let learning_rate = parser.f64("--learning-rate")?;
            let max_epochs = parser
                .usize_opt("--max-epochs")?
                .unwrap_or(DEFAULT_TUNE_MAX_EPOCHS);
            let patience = parser.usize_opt("--patience")?.unwrap_or(DEFAULT_PATIENCE);
            let min_delta = parser.f64_opt("--min-delta")?.unwrap_or(DEFAULT_MIN_DELTA);
            let eps = parser
                .f64_opt("--adagrad-eps")?
                .unwrap_or(DEFAULT_ADAGRAD_EPS);
            let train = TrainMask::parse(parser.take("--train-weights"))?;
            parser.finish()?;
            Ok(Command::Tune {
                common,
                k,
                learning_rate,
                max_epochs,
                patience,
                min_delta,
                eps,
                train,
            })
        }
        _ => Err(usage()),
    }
}

fn common_options(
    parser: &mut ArgParser,
    dataset: PathBuf,
    output: PathBuf,
) -> Result<CommonOptions, String> {
    let cache = parser
        .path_opt("--cache")
        .unwrap_or_else(|| default_cache_path(&dataset));
    let validation_percent = parser
        .u8_opt("--validation-percent")?
        .unwrap_or(DEFAULT_VALIDATION_PERCENT);
    let seed = parser.u64_opt("--seed")?.unwrap_or(DEFAULT_SEED);
    let batch_size = parser
        .usize_opt("--batch-size")?
        .unwrap_or(DEFAULT_BATCH_SIZE);
    if validation_percent > 100 {
        return Err("--validation-percent must be <= 100".to_string());
    }
    if batch_size == 0 {
        return Err("--batch-size must be positive".to_string());
    }
    Ok(CommonOptions {
        dataset,
        cache,
        output,
        validation_percent,
        seed,
        batch_size,
    })
}

fn usage() -> String {
    "usage: texel_tuner <find-k|scan-lr|tune> --dataset <path> [options]\n\
     choose tuned weights with: --train-weights \"all\" or --train-weights \"0,10..20,pawn psqt mg\""
        .to_string()
}

struct ArgParser {
    args: Vec<String>,
}

impl ArgParser {
    fn new(args: Vec<String>) -> Self {
        Self { args }
    }

    fn path(&mut self, name: &str) -> Result<PathBuf, String> {
        self.take(name)
            .map(PathBuf::from)
            .ok_or_else(|| format!("missing required {name}"))
    }

    fn path_opt(&mut self, name: &str) -> Option<PathBuf> {
        self.take(name).map(PathBuf::from)
    }

    fn f64(&mut self, name: &str) -> Result<f64, String> {
        self.take(name)
            .ok_or_else(|| format!("missing required {name}"))?
            .parse()
            .map_err(|_| format!("invalid {name}"))
    }

    fn f64_opt(&mut self, name: &str) -> Result<Option<f64>, String> {
        self.take(name)
            .map(|value| value.parse().map_err(|_| format!("invalid {name}")))
            .transpose()
    }

    fn f64_list_opt(&mut self, name: &str) -> Result<Option<Vec<f64>>, String> {
        self.take(name)
            .map(|value| {
                value
                    .split(',')
                    .map(|part| part.parse().map_err(|_| format!("invalid {name}")))
                    .collect()
            })
            .transpose()
    }

    fn usize_opt(&mut self, name: &str) -> Result<Option<usize>, String> {
        self.take(name)
            .map(|value| value.parse().map_err(|_| format!("invalid {name}")))
            .transpose()
    }

    fn u8_opt(&mut self, name: &str) -> Result<Option<u8>, String> {
        self.take(name)
            .map(|value| value.parse().map_err(|_| format!("invalid {name}")))
            .transpose()
    }

    fn u64_opt(&mut self, name: &str) -> Result<Option<u64>, String> {
        self.take(name)
            .map(|value| value.parse().map_err(|_| format!("invalid {name}")))
            .transpose()
    }

    fn take(&mut self, name: &str) -> Option<String> {
        let idx = self.args.iter().position(|arg| arg == name)?;
        self.args.remove(idx);
        if idx >= self.args.len() {
            return Some(String::new());
        }
        Some(self.args.remove(idx))
    }

    fn finish(self) -> Result<(), String> {
        if self.args.is_empty() {
            Ok(())
        } else {
            Err(format!("unknown arguments: {}", self.args.join(" ")))
        }
    }
}

fn default_cache_path(dataset: &Path) -> PathBuf {
    let mut path = dataset.as_os_str().to_os_string();
    path.push(".anton-tune-cache");
    PathBuf::from(path)
}

fn ensure_cache(common: &CommonOptions) -> Result<CacheHeader, String> {
    let expected = expected_cache_header(common)?;
    if let Ok(header) = read_cache_header(&common.cache) {
        if cache_matches(&header, &expected) {
            println!("using cache {}", common.cache.display());
            return Ok(header);
        }
    }

    println!("building cache {}", common.cache.display());
    build_cache(common, expected)
}

fn expected_cache_header(common: &CommonOptions) -> Result<CacheHeader, String> {
    let metadata = fs::metadata(&common.dataset)
        .map_err(|err| format!("failed to stat dataset {}: {err}", common.dataset.display()))?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    Ok(CacheHeader {
        dataset_path: common.dataset.to_string_lossy().into_owned(),
        dataset_len: metadata.len(),
        dataset_modified_secs: modified,
        feature_count: FEATURE_COUNT as u32,
        validation_percent: common.validation_percent,
        seed: common.seed,
        stats: DatasetStats::default(),
    })
}

fn cache_matches(actual: &CacheHeader, expected: &CacheHeader) -> bool {
    actual.dataset_path == expected.dataset_path
        && actual.dataset_len == expected.dataset_len
        && actual.dataset_modified_secs == expected.dataset_modified_secs
        && actual.feature_count == expected.feature_count
        && actual.validation_percent == expected.validation_percent
        && actual.seed == expected.seed
}

fn build_cache(common: &CommonOptions, mut header: CacheHeader) -> Result<CacheHeader, String> {
    let dataset = File::open(&common.dataset)
        .map_err(|err| format!("failed to open dataset {}: {err}", common.dataset.display()))?;
    let mut writer = BufWriter::new(
        File::create(&common.cache)
            .map_err(|err| format!("failed to create cache {}: {err}", common.cache.display()))?,
    );
    write_header(&mut writer, &header)
        .map_err(|err| format!("failed to write cache header: {err}"))?;

    let mut progress = CacheProgress::new(header.dataset_len);
    let mut skips = SkipStats::default();
    let movegen = MoveGenerator::new();
    let mut reader = BufReader::new(dataset);
    let mut line = String::new();
    let mut line_idx = 0_u64;
    loop {
        line.clear();
        let bytes_read = reader
            .read_line(&mut line)
            .map_err(|err| format!("failed to read dataset line {}: {err}", line_idx + 1))?;
        if bytes_read == 0 {
            break;
        }
        line_idx += 1;
        progress.advance(bytes_read as u64, &header.stats);
        let row = match parse_dataset_line(&line) {
            Ok(row) => row,
            Err(err) => {
                header.stats.skipped += 1;
                skips.record_parse(line_idx, err);
                continue;
            }
        };
        let sample = match sample_from_row(&row, common.validation_percent, common.seed, &movegen) {
            Ok(sample) => sample,
            Err(err) => {
                header.stats.skipped += 1;
                skips.record_position(line_idx, err);
                continue;
            }
        };
        match sample.split {
            Split::Train => header.stats.train += 1,
            Split::Validation => header.stats.validation += 1,
        }
        header.stats.valid += 1;
        write_sample(&mut writer, &sample)
            .map_err(|err| format!("failed to write cache sample: {err}"))?;
    }
    progress.finish(&header.stats);

    writer
        .flush()
        .map_err(|err| format!("failed to flush cache: {err}"))?;
    drop(writer);

    let mut file = File::options()
        .write(true)
        .open(&common.cache)
        .map_err(|err| format!("failed to reopen cache header: {err}"))?;
    write_header(&mut file, &header)
        .map_err(|err| format!("failed to update cache header: {err}"))?;
    println!(
        "cache built valid {} skipped {} train {} validation {}",
        header.stats.valid, header.stats.skipped, header.stats.train, header.stats.validation
    );
    if header.stats.skipped > 0 {
        println!(
            "skip reasons parse {} position {}",
            skips.parse, skips.position
        );
        for example in skips.examples {
            println!("skip example: {example}");
        }
    }
    Ok(header)
}

fn parse_dataset_line(line: &str) -> Result<DatasetRow, String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 8 {
        return Err("expected 8 fields".to_string());
    }
    let fen = parts[0..6].join(" ");
    let result_token = parts[6]
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(parts[6]);
    let result = match result_token {
        "0" | "0.0" => 0.0,
        "0.5" => 0.5,
        "1" | "1.0" => 1.0,
        _ => return Err("invalid result".to_string()),
    };
    let source_eval = parts[7].parse().map_err(|_| "invalid eval".to_string())?;
    Ok(DatasetRow {
        fen,
        result,
        source_eval,
    })
}

struct CacheProgress {
    total_bytes: u64,
    read_bytes: u64,
    last_draw: Instant,
}

impl CacheProgress {
    fn new(total_bytes: u64) -> Self {
        Self {
            total_bytes,
            read_bytes: 0,
            last_draw: Instant::now() - Duration::from_secs(1),
        }
    }

    fn advance(&mut self, bytes: u64, stats: &DatasetStats) {
        self.read_bytes = self.read_bytes.saturating_add(bytes);
        if self.last_draw.elapsed() >= Duration::from_millis(250) {
            self.draw(stats, false);
            self.last_draw = Instant::now();
        }
    }

    fn finish(&mut self, stats: &DatasetStats) {
        self.read_bytes = self.total_bytes;
        self.draw(stats, true);
        eprintln!();
    }

    fn draw(&self, stats: &DatasetStats, done: bool) {
        let width: usize = 30;
        let ratio = if self.total_bytes == 0 {
            1.0
        } else {
            (self.read_bytes as f64 / self.total_bytes as f64).clamp(0.0, 1.0)
        };
        let filled = (ratio * width as f64).round() as usize;
        let bar = format!(
            "{}{}",
            "#".repeat(filled),
            "-".repeat(width.saturating_sub(filled))
        );
        let suffix = if done { " done" } else { "" };
        eprint!(
            "\rcache [{bar}] {:>6.2}% valid {} skipped {}{}",
            ratio * 100.0,
            stats.valid,
            stats.skipped,
            suffix
        );
        let _ = io::stderr().flush();
    }
}

fn sample_from_row(
    row: &DatasetRow,
    validation_percent: u8,
    seed: u64,
    movegen: &MoveGenerator,
) -> Result<Sample, String> {
    let board = Board::from_fen(&row.fen).map_err(|err| format!("invalid fen: {err}"))?;
    let mut trace = FeatureVectorTrace::new();
    evaluate(&board, movegen, &mut trace);
    let features = trace
        .tapered_features(board.state.game_phase)
        .into_iter()
        .enumerate()
        .filter_map(|(idx, value)| {
            if value == 0.0 {
                None
            } else {
                Some((idx as u16, value as f32))
            }
        })
        .collect();
    Ok(Sample {
        split: split_for(&row.fen, validation_percent, seed),
        result: row.result,
        source_eval: row.source_eval,
        features,
    })
}

fn split_for(fen: &str, validation_percent: u8, seed: u64) -> Split {
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    fen.hash(&mut hasher);
    if hasher.finish() % 100 < validation_percent as u64 {
        Split::Validation
    } else {
        Split::Train
    }
}

fn write_header(writer: &mut impl Write, header: &CacheHeader) -> io::Result<()> {
    writer.write_all(CACHE_MAGIC)?;
    write_u32(writer, CACHE_VERSION)?;
    write_u32(writer, header.feature_count)?;
    writer.write_all(&[header.validation_percent])?;
    write_u64(writer, header.seed)?;
    write_u64(writer, header.dataset_len)?;
    write_u64(writer, header.dataset_modified_secs)?;
    write_u64(writer, header.stats.valid)?;
    write_u64(writer, header.stats.skipped)?;
    write_u64(writer, header.stats.train)?;
    write_u64(writer, header.stats.validation)?;
    write_string(writer, &header.dataset_path)?;
    Ok(())
}

fn read_cache_header(path: &Path) -> io::Result<CacheHeader> {
    let mut reader = BufReader::new(File::open(path)?);
    read_header(&mut reader)
}

fn read_header(reader: &mut impl Read) -> io::Result<CacheHeader> {
    let mut magic = [0; 8];
    reader.read_exact(&mut magic)?;
    if &magic != CACHE_MAGIC {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "bad cache magic",
        ));
    }
    let version = read_u32(reader)?;
    if version != CACHE_VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "bad cache version",
        ));
    }
    let feature_count = read_u32(reader)?;
    let mut percent = [0];
    reader.read_exact(&mut percent)?;
    let seed = read_u64(reader)?;
    let dataset_len = read_u64(reader)?;
    let dataset_modified_secs = read_u64(reader)?;
    let valid = read_u64(reader)?;
    let skipped = read_u64(reader)?;
    let train = read_u64(reader)?;
    let validation = read_u64(reader)?;
    let dataset_path = read_string(reader)?;
    Ok(CacheHeader {
        dataset_path,
        dataset_len,
        dataset_modified_secs,
        feature_count,
        validation_percent: percent[0],
        seed,
        stats: DatasetStats {
            valid,
            skipped,
            train,
            validation,
        },
    })
}

fn write_sample(writer: &mut impl Write, sample: &Sample) -> io::Result<()> {
    writer.write_all(&[match sample.split {
        Split::Train => 0,
        Split::Validation => 1,
    }])?;
    write_f32(writer, sample.result)?;
    write_i32(writer, sample.source_eval)?;
    write_u16(writer, sample.features.len() as u16)?;
    for &(idx, value) in &sample.features {
        write_u16(writer, idx)?;
        write_f32(writer, value)?;
    }
    Ok(())
}

fn read_sample(reader: &mut impl Read) -> io::Result<Option<Sample>> {
    let mut split = [0];
    match reader.read_exact(&mut split) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(err) => return Err(err),
    }
    let result = read_f32(reader)?;
    let source_eval = read_i32(reader)?;
    let feature_len = read_u16(reader)? as usize;
    let mut features = Vec::with_capacity(feature_len);
    for _ in 0..feature_len {
        features.push((read_u16(reader)?, read_f32(reader)?));
    }
    let split = match split[0] {
        0 => Split::Train,
        1 => Split::Validation,
        _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid split")),
    };
    Ok(Some(Sample {
        split,
        result,
        source_eval,
        features,
    }))
}

fn for_each_batch(
    cache: &Path,
    batch_size: usize,
    mut f: impl FnMut(&[Sample]) -> Result<(), String>,
) -> Result<(), String> {
    let file = File::open(cache).map_err(|err| format!("failed to open cache: {err}"))?;
    let mut reader = BufReader::new(file);
    read_header(&mut reader).map_err(|err| format!("failed to read cache header: {err}"))?;
    let mut batch = Vec::with_capacity(batch_size);
    loop {
        match read_sample(&mut reader)
            .map_err(|err| format!("failed to read cache sample: {err}"))?
        {
            Some(sample) => {
                batch.push(sample);
                if batch.len() == batch_size {
                    f(&batch)?;
                    batch.clear();
                }
            }
            None => break,
        }
    }
    if !batch.is_empty() {
        f(&batch)?;
    }
    Ok(())
}

fn initial_weights() -> Vec<f64> {
    EVAL_PARAMS
        .weights()
        .into_iter()
        .map(|weight| weight as f64)
        .collect()
}

fn find_best_k(
    cache: &Path,
    weights: &[f64],
    k_min: f64,
    k_max: f64,
    k_steps: usize,
    batch_size: usize,
) -> Result<(f64, Mse), String> {
    let mut best = (
        0.0,
        Mse {
            train: f64::INFINITY,
            validation: f64::INFINITY,
        },
    );
    for k in logspace(k_min, k_max, k_steps) {
        let mse = compute_mse(cache, weights, k, batch_size)?;
        println!(
            "k {:.12} train_mse {:.12} validation_mse {:.12}",
            k, mse.train, mse.validation
        );
        if mse.train < best.1.train {
            best = (k, mse);
        }
    }
    Ok(best)
}

fn logspace(min: f64, max: f64, steps: usize) -> Vec<f64> {
    if steps == 1 {
        return vec![min];
    }
    let min_ln = min.ln();
    let max_ln = max.ln();
    (0..steps)
        .map(|idx| {
            let t = idx as f64 / (steps - 1) as f64;
            (min_ln + (max_ln - min_ln) * t).exp()
        })
        .collect()
}

fn compute_mse(cache: &Path, weights: &[f64], k: f64, batch_size: usize) -> Result<Mse, String> {
    let mut train_loss = 0.0;
    let mut validation_loss = 0.0;
    let mut train_count = 0_u64;
    let mut validation_count = 0_u64;
    for_each_batch(cache, batch_size, |batch| {
        for sample in batch {
            let loss = sample_loss(sample, weights, k);
            match sample.split {
                Split::Train => {
                    train_loss += loss;
                    train_count += 1;
                }
                Split::Validation => {
                    validation_loss += loss;
                    validation_count += 1;
                }
            }
        }
        Ok(())
    })?;
    Ok(Mse {
        train: mean_or_inf(train_loss, train_count),
        validation: mean_or_inf(validation_loss, validation_count),
    })
}

fn mean_or_inf(total: f64, count: u64) -> f64 {
    if count == 0 {
        f64::INFINITY
    } else {
        total / count as f64
    }
}

fn sample_loss(sample: &Sample, weights: &[f64], k: f64) -> f64 {
    let p = predict(sample, weights, k);
    let error = sample.result as f64 - p;
    error * error
}

fn predict(sample: &Sample, weights: &[f64], k: f64) -> f64 {
    sigmoid(k * dot(weights, &sample.features))
}

fn dot(weights: &[f64], features: &[(u16, f32)]) -> f64 {
    features
        .iter()
        .map(|&(idx, value)| weights[idx as usize] * value as f64)
        .sum()
}

fn sigmoid(x: f64) -> f64 {
    if x >= 0.0 {
        1.0 / (1.0 + (-x).exp())
    } else {
        let exp = x.exp();
        exp / (1.0 + exp)
    }
}

fn train_one_epoch(
    cache: &Path,
    weights: &mut [f64],
    acc: &mut [f64],
    k: f64,
    learning_rate: f64,
    eps: f64,
    batch_size: usize,
    train: &TrainMask,
) -> Result<(), String> {
    for_each_batch(cache, batch_size, |batch| {
        for sample in batch.iter().filter(|sample| sample.split == Split::Train) {
            adagrad_update(sample, weights, acc, k, learning_rate, eps, train);
        }
        Ok(())
    })
}

fn adagrad_update(
    sample: &Sample,
    weights: &mut [f64],
    acc: &mut [f64],
    k: f64,
    learning_rate: f64,
    eps: f64,
    train: &TrainMask,
) {
    let p = predict(sample, weights, k);
    let error = sample.result as f64 - p;
    let common = -2.0 * error * k * p * (1.0 - p);
    for &(idx, value) in &sample.features {
        let idx = idx as usize;
        if !train.should_train(idx) {
            continue;
        }
        let grad = common * value as f64;
        acc[idx] += grad * grad;
        weights[idx] -= learning_rate * grad / (acc[idx].sqrt() + eps);
    }
}

fn tune(
    cache: &Path,
    k: f64,
    learning_rate: f64,
    max_epochs: usize,
    patience: usize,
    min_delta: f64,
    eps: f64,
    batch_size: usize,
    train: &TrainMask,
) -> Result<TuneResult, String> {
    let mut weights = initial_weights();
    let mut acc = vec![0.0; FEATURE_COUNT];
    let mut best_weights = weights.clone();
    let mut best_epoch = 0;
    let mut best_train_mse = f64::INFINITY;
    let mut best_validation_mse = f64::INFINITY;
    let mut stale_epochs = 0;
    let mut epochs_run = 0;

    for epoch in 1..=max_epochs {
        train_one_epoch(
            cache,
            &mut weights,
            &mut acc,
            k,
            learning_rate,
            eps,
            batch_size,
            train,
        )?;
        let mse = compute_mse(cache, &weights, k, batch_size)?;
        epochs_run = epoch;
        println!(
            "epoch {epoch} train_mse {:.12} validation_mse {:.12}",
            mse.train, mse.validation
        );
        if mse.validation < best_validation_mse - min_delta {
            best_validation_mse = mse.validation;
            best_train_mse = mse.train;
            best_epoch = epoch;
            best_weights.clone_from(&weights);
            stale_epochs = 0;
        } else {
            stale_epochs += 1;
            if stale_epochs >= patience {
                break;
            }
        }
        if termination_requested() {
            eprintln!("termination requested; stopping after epoch {epoch}");
            break;
        }
    }

    Ok(TuneResult {
        weights: best_weights,
        best_epoch,
        best_train_mse,
        best_validation_mse,
        epochs_run,
    })
}

fn write_weights(
    path: &Path,
    result: &TuneResult,
    header: &CacheHeader,
    k: f64,
    learning_rate: f64,
    max_epochs: usize,
    patience: usize,
    min_delta: f64,
    trainable_weight_count: usize,
) -> Result<(), String> {
    let mut output = String::new();
    output.push_str("use super::EvalValue;\n\n");
    output.push_str(&format!(
        "// Generated by texel_tuner\n// k: {:.12}\n// learning_rate: {:.6}\n// best_epoch: {}\n// epochs_run: {}\n// max_epochs: {}\n// patience: {}\n// min_delta: {:.12}\n// trainable_weights: {}\n// train_mse: {:.12}\n// validation_mse: {:.12}\n// valid: {}\n// skipped: {}\n// train_samples: {}\n// validation_samples: {}\n// validation_percent: {}\n// seed: {}\n\n",
        k,
        learning_rate,
        result.best_epoch,
        result.epochs_run,
        max_epochs,
        patience,
        min_delta,
        trainable_weight_count,
        result.best_train_mse,
        result.best_validation_mse,
        header.stats.valid,
        header.stats.skipped,
        header.stats.train,
        header.stats.validation,
        header.validation_percent,
        header.seed,
    ));
    output.push_str("#[rustfmt::skip]\n");
    output.push_str(&format!(
        "pub const WEIGHTS: [EvalValue; {}] = [\n",
        result.weights.len()
    ));
    push_weight_layout(&mut output, &result.weights)?;
    output.push_str("];\n\n");
    output.push_str(&format!("pub const WEIGHT_COUNT: usize = WEIGHTS.len();\n"));
    fs::write(path, output).map_err(|err| format!("failed to write weights: {err}"))
}

fn push_weight_layout(output: &mut String, weights: &[f64]) -> Result<(), String> {
    let expected = WEIGHT_LAYOUT
        .iter()
        .map(|category| category.shape.0 * category.shape.1)
        .sum::<usize>();
    if expected != weights.len() {
        return Err(format!(
            "weight layout size {expected} does not match weight count {}",
            weights.len()
        ));
    }

    let mut offset = 0;
    for category in WEIGHT_LAYOUT {
        let len = category.shape.0 * category.shape.1;
        push_weight_block(output, category, &weights[offset..offset + len]);
        offset += len;
    }

    Ok(())
}

fn weight_category_by_name(name: &str) -> Option<(usize, &'static WeightCategory)> {
    let mut offset = 0;
    for category in WEIGHT_LAYOUT {
        let len = category.shape.0 * category.shape.1;
        if category.name == name {
            return Some((offset, category));
        }
        offset += len;
    }
    None
}

fn parse_weight_idx(value: &str) -> Result<usize, String> {
    value
        .parse()
        .map_err(|_| format!("invalid --train-weights index {value:?}"))
}

fn push_weight_block(output: &mut String, category: &WeightCategory, weights: &[f64]) {
    output.push_str(&format!("    // {}\n", category.name));
    for row in 0..category.shape.0 {
        output.push_str("    ");
        let start = row * category.shape.1;
        for weight in &weights[start..start + category.shape.1] {
            output.push_str(&format!("{:>6},", round_weight(*weight)));
        }
        output.push('\n');
    }
    output.push('\n');
}

fn round_weight(weight: f64) -> EvalValue {
    weight
        .round()
        .clamp(EvalValue::MIN as f64, EvalValue::MAX as f64) as EvalValue
}

fn write_string(writer: &mut impl Write, value: &str) -> io::Result<()> {
    write_u32(writer, value.len() as u32)?;
    writer.write_all(value.as_bytes())
}

fn read_string(reader: &mut impl Read) -> io::Result<String> {
    let len = read_u32(reader)? as usize;
    let mut bytes = vec![0; len];
    reader.read_exact(&mut bytes)?;
    String::from_utf8(bytes).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

fn write_u16(writer: &mut impl Write, value: u16) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

fn read_u16(reader: &mut impl Read) -> io::Result<u16> {
    let mut bytes = [0; 2];
    reader.read_exact(&mut bytes)?;
    Ok(u16::from_le_bytes(bytes))
}

fn write_u32(writer: &mut impl Write, value: u32) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

fn read_u32(reader: &mut impl Read) -> io::Result<u32> {
    let mut bytes = [0; 4];
    reader.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn write_u64(writer: &mut impl Write, value: u64) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

fn read_u64(reader: &mut impl Read) -> io::Result<u64> {
    let mut bytes = [0; 8];
    reader.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

fn write_i32(writer: &mut impl Write, value: i32) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

fn read_i32(reader: &mut impl Read) -> io::Result<i32> {
    let mut bytes = [0; 4];
    reader.read_exact(&mut bytes)?;
    Ok(i32::from_le_bytes(bytes))
}

fn write_f32(writer: &mut impl Write, value: f32) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

fn read_f32(reader: &mut impl Read) -> io::Result<f32> {
    let mut bytes = [0; 4];
    reader.read_exact(&mut bytes)?;
    Ok(f32::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_valid_dataset_line() {
        let row =
            parse_dataset_line("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 0.5 12")
                .unwrap();
        assert_eq!(
            row.fen,
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
        );
        assert_eq!(row.result, 0.5);
        assert_eq!(row.source_eval, 12);
    }

    #[test]
    fn parses_bracketed_result() {
        let row =
            parse_dataset_line("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 [0.5] 12")
                .unwrap();
        assert_eq!(row.result, 0.5);
        assert_eq!(row.source_eval, 12);
    }

    #[test]
    fn parses_decimal_win_loss_results() {
        let win = parse_dataset_line("8/8/8/8/8/8/8/8 w - - 0 1 [1.0] 42").unwrap();
        let loss = parse_dataset_line("8/8/8/8/8/8/8/8 w - - 0 1 [0.0] -42").unwrap();
        assert_eq!(win.result, 1.0);
        assert_eq!(loss.result, 0.0);
    }

    #[test]
    fn rejects_invalid_result() {
        assert!(parse_dataset_line("8/8/8/8/8/8/8/8 w - - 0 1 0.25 0").is_err());
    }

    #[test]
    fn rejects_missing_eval() {
        assert!(parse_dataset_line("8/8/8/8/8/8/8/8 w - - 0 1 1").is_err());
    }

    #[test]
    fn parses_extra_whitespace() {
        let row = parse_dataset_line("  8/8/8/8/8/8/8/8   w  -  -  0  1  1  -3  ").unwrap();
        assert_eq!(row.result, 1.0);
        assert_eq!(row.source_eval, -3);
    }

    #[test]
    fn split_is_deterministic() {
        let fen = "8/8/8/8/8/8/8/8 w - - 0 1";
        assert_eq!(split_for(fen, 10, 7), split_for(fen, 10, 7));
    }

    #[test]
    fn split_percent_extremes_are_predictable() {
        let fen = "8/8/8/8/8/8/8/8 w - - 0 1";
        assert_eq!(split_for(fen, 0, 0), Split::Train);
        assert_eq!(split_for(fen, 100, 0), Split::Validation);
    }

    #[test]
    fn sigmoid_zero_is_half() {
        assert_eq!(sigmoid(0.0), 0.5);
    }

    #[test]
    fn mse_matches_hand_computed_sample() {
        let sample = Sample {
            split: Split::Train,
            result: 1.0,
            source_eval: 0,
            features: vec![(0, 1.0)],
        };
        let loss = sample_loss(&sample, &[0.0], 0.1);
        assert!((loss - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn adagrad_moves_positive_feature_toward_win() {
        let sample = Sample {
            split: Split::Train,
            result: 1.0,
            source_eval: 0,
            features: vec![(0, 1.0)],
        };
        let mut weights = vec![0.0];
        let mut acc = vec![0.0];
        adagrad_update(
            &sample,
            &mut weights,
            &mut acc,
            0.1,
            1.0,
            1e-8,
            &TrainMask::parse(Some("all".to_string())).unwrap(),
        );
        assert!(weights[0] > 0.0);
    }

    #[test]
    fn adagrad_only_updates_trainable_weights() {
        let sample = Sample {
            split: Split::Train,
            result: 1.0,
            source_eval: 0,
            features: vec![(0, 1.0), (1, 1.0)],
        };
        let train = TrainMask::parse(Some("1".to_string())).unwrap();
        let mut weights = vec![0.0, 0.0];
        let mut acc = vec![0.0, 0.0];

        adagrad_update(&sample, &mut weights, &mut acc, 0.1, 1.0, 1e-8, &train);

        assert_eq!(weights[0], 0.0);
        assert_eq!(acc[0], 0.0);
        assert!(weights[1] > 0.0);
        assert!(acc[1] > 0.0);
    }

    #[test]
    fn train_mask_defaults_to_no_trainable_weights() {
        let sample = Sample {
            split: Split::Train,
            result: 1.0,
            source_eval: 0,
            features: vec![(0, 1.0)],
        };
        let train = TrainMask::parse(None).unwrap();
        let mut weights = vec![0.0];
        let mut acc = vec![0.0];

        adagrad_update(&sample, &mut weights, &mut acc, 0.1, 1.0, 1e-8, &train);

        assert_eq!(train.count(), 0);
        assert_eq!(weights[0], 0.0);
        assert_eq!(acc[0], 0.0);
    }

    #[test]
    fn train_mask_parses_indexes_ranges_categories_and_all() {
        let mask = TrainMask::parse(Some("0,2..4,material eg".to_string())).unwrap();

        assert!(mask.should_train(0));
        assert!(!mask.should_train(1));
        assert!(mask.should_train(2));
        assert!(mask.should_train(3));
        assert!(mask.should_train(4));
        for idx in 6..12 {
            assert!(mask.should_train(idx));
        }
        assert_eq!(mask.count(), 10);

        let all = TrainMask::parse(Some("all".to_string())).unwrap();
        assert_eq!(all.count(), FEATURE_COUNT);
    }

    #[test]
    fn cache_sample_round_trips() {
        let sample = Sample {
            split: Split::Validation,
            result: 0.5,
            source_eval: -20,
            features: vec![(1, 1.0), (9, -0.5)],
        };
        let mut bytes = Vec::new();
        write_sample(&mut bytes, &sample).unwrap();
        let decoded = read_sample(&mut bytes.as_slice()).unwrap().unwrap();
        assert_eq!(decoded, sample);
    }

    #[test]
    fn cache_file_round_trips_batches() {
        let path = temp_path("cache-file-round-trip");
        let header = test_header();
        let sample = Sample {
            split: Split::Train,
            result: 1.0,
            source_eval: 22,
            features: vec![(0, 1.0)],
        };
        {
            let mut writer = BufWriter::new(File::create(&path).unwrap());
            write_header(&mut writer, &header).unwrap();
            write_sample(&mut writer, &sample).unwrap();
            writer.flush().unwrap();
        }

        let mut samples = Vec::new();
        for_each_batch(&path, 1, |batch| {
            samples.extend_from_slice(batch);
            Ok(())
        })
        .unwrap();

        assert_eq!(samples, vec![sample]);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn logspace_includes_endpoints() {
        let values = logspace(0.01, 1.0, 3);
        assert!((values[0] - 0.01).abs() < 1e-12);
        assert!((values[2] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn lr_scan_fresh_state_behavior() {
        let sample = Sample {
            split: Split::Train,
            result: 1.0,
            source_eval: 0,
            features: vec![(0, 1.0)],
        };
        let mut weights_a = vec![0.0];
        let mut acc_a = vec![0.0];
        let mut weights_b = vec![0.0];
        let mut acc_b = vec![0.0];
        let train = TrainMask::parse(Some("all".to_string())).unwrap();
        adagrad_update(&sample, &mut weights_a, &mut acc_a, 0.1, 1.0, 1e-8, &train);
        adagrad_update(&sample, &mut weights_b, &mut acc_b, 0.1, 1.0, 1e-8, &train);
        assert_eq!(weights_a, weights_b);
        assert_eq!(acc_a, acc_b);
    }

    #[test]
    fn best_weight_restoration_prefers_best_epoch() {
        let result = TuneResult {
            weights: vec![3.2],
            best_epoch: 2,
            best_train_mse: 0.1,
            best_validation_mse: 0.2,
            epochs_run: 4,
        };
        assert_eq!(result.best_epoch, 2);
        assert_eq!(round_weight(result.weights[0]), 3);
    }

    #[test]
    fn weight_block_formatter_adds_category_comment() {
        let mut output = String::new();
        let category = WeightCategory {
            name: "material mg",
            shape: (1, 2),
        };
        push_weight_block(&mut output, &category, &[1.2, -2.6]);
        assert!(output.contains("// material mg"));
        assert!(output.contains("     1,    -3,"));
    }

    #[test]
    fn early_stopping_stops_after_patience() {
        let path = temp_path("early-stopping");
        let mut header = test_header();
        header.stats.validation = 1;
        header.stats.valid = 1;
        let sample = Sample {
            split: Split::Validation,
            result: 0.5,
            source_eval: 0,
            features: vec![(0, 1.0)],
        };
        {
            let mut writer = BufWriter::new(File::create(&path).unwrap());
            write_header(&mut writer, &header).unwrap();
            write_sample(&mut writer, &sample).unwrap();
            writer.flush().unwrap();
        }

        let result = tune(
            &path,
            0.1,
            1.0,
            10,
            2,
            0.0,
            1e-8,
            1,
            &TrainMask::parse(Some("all".to_string())).unwrap(),
        )
        .unwrap();

        assert_eq!(result.best_epoch, 1);
        assert_eq!(result.epochs_run, 3);
        let _ = fs::remove_file(path);
    }

    fn test_header() -> CacheHeader {
        CacheHeader {
            dataset_path: "test".to_string(),
            dataset_len: 0,
            dataset_modified_secs: 0,
            feature_count: FEATURE_COUNT as u32,
            validation_percent: 10,
            seed: 0,
            stats: DatasetStats::default(),
        }
    }

    fn temp_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("anton-texel-tuner-{name}-{nanos}"))
    }
}
