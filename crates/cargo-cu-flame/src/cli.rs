use std::path::{Path, PathBuf};
use clap::{Parser, Subcommand};
use sol_flame_core::{DwarfSymbolResolver, FoldedStackAggregator};
use sol_flame_mollusk::{ProfileScenario, SvmInstructionSimulator};
use sol_flame_render::{CiRegressionReport, CuProfile, FlameRenderer, TerminalPresenter};

#[derive(Parser, Debug)]
#[command(
    name = "cargo-cu-flame",
    bin_name = "cargo-cu-flame",
    version,
    about = "⚡ The Zero-Overhead Compute Unit & Memory Flamegraph Profiler for Solana Programs",
    long_about = "Profile Solana sBPF binaries in-process using mollusk-svm, symbolicate with DWARF via gimli, and generate interactive SVG flamegraphs."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Solana instruction name to profile
    #[arg(short, long, default_value = "deposit")]
    pub instruction: String,

    /// Path to compiled sBPF ELF binary (.so)
    #[arg(short, long)]
    pub elf: Option<PathBuf>,

    /// Output path for interactive SVG flamegraph
    #[arg(short, long, default_value = "target/sol-flame/flamegraph.svg")]
    pub output: PathBuf,

    /// Output path for structured JSON profile
    #[arg(short, long, default_value = "target/sol-flame/cu-profile.json")]
    pub json: PathBuf,

    /// Max Compute Unit budget
    #[arg(long, default_value_t = 200_000)]
    pub max_cu: u64,

    /// Invariant fuzz iterations
    #[arg(long)]
    pub fuzz: Option<u64>,

    /// Baseline JSON profile path for CI regression check
    #[arg(long)]
    pub ci_check: Option<PathBuf>,

    /// Maximum permitted percentage regression for CI check (e.g. 5.0 for 5%)
    #[arg(long, default_value_t = 5.0)]
    pub threshold: f64,

    /// Run out-of-the-box demonstration profile
    #[arg(long)]
    pub demo: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Cargo subcommand pass-through when called as `cargo cu-flame`
    CuFlame(CuFlameArgs),

    /// Run out-of-the-box demonstration profile
    Demo {
        /// Scenario: "anchor" or "pinocchio"
        #[arg(short, long, default_value = "anchor")]
        scenario: String,
        #[arg(short, long, default_value = "target/sol-flame/flamegraph.svg")]
        output: PathBuf,
        #[arg(short, long, default_value = "target/sol-flame/cu-profile.json")]
        json: PathBuf,
    },

    /// Inspect and dump DWARF symbols from an ELF binary
    Symbols {
        /// Path to sBPF ELF shared object (.so)
        elf: PathBuf,
    },

    /// Compare two `cu-profile.json` files for CI regression testing
    Compare {
        /// Base commit JSON profile
        base: PathBuf,
        /// Current PR/branch JSON profile
        current: PathBuf,
        /// Max allowed percentage regression (e.g. 5.0)
        #[arg(short, long, default_value_t = 5.0)]
        threshold: f64,
    },
}

#[derive(Parser, Debug)]
pub struct CuFlameArgs {
    #[arg(short, long, default_value = "deposit")]
    pub instruction: String,
    #[arg(short, long)]
    pub elf: Option<PathBuf>,
    #[arg(short, long, default_value = "target/sol-flame/flamegraph.svg")]
    pub output: PathBuf,
    #[arg(short, long, default_value = "target/sol-flame/cu-profile.json")]
    pub json: PathBuf,
    #[arg(long, default_value_t = 200_000)]
    pub max_cu: u64,
    #[arg(long)]
    pub fuzz: Option<u64>,
    #[arg(long)]
    pub ci_check: Option<PathBuf>,
    #[arg(long, default_value_t = 5.0)]
    pub threshold: f64,
    #[arg(long)]
    pub demo: bool,
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "cu-flame" {
        args.remove(1);
    }

    let cli = Cli::parse_from(args);

    match cli.command {
        Some(Commands::CuFlame(args)) => {
            run_profile(
                &args.instruction,
                args.elf.as_deref(),
                &args.output,
                &args.json,
                args.max_cu,
                args.fuzz,
                args.ci_check.as_deref(),
                args.threshold,
                args.demo,
            )?;
        }
        Some(Commands::Demo { scenario, output, json }) => {
            let sc = if scenario.to_lowercase().contains("pinocchio") {
                ProfileScenario::PinocchioVaultSwap
            } else {
                ProfileScenario::AnchorEscrowDeposit
            };
            run_demo(sc, &output, &json, 200_000)?;
        }
        Some(Commands::Symbols { elf }) => {
            dump_symbols(&elf)?;
        }
        Some(Commands::Compare { base, current, threshold }) => {
            run_compare(&base, &current, threshold)?;
        }
        None => {
            run_profile(
                &cli.instruction,
                cli.elf.as_deref(),
                &cli.output,
                &cli.json,
                cli.max_cu,
                cli.fuzz,
                cli.ci_check.as_deref(),
                cli.threshold,
                cli.demo,
            )?;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn run_profile(
    instruction: &str,
    elf_path: Option<&Path>,
    output_svg: &Path,
    output_json: &Path,
    max_cu: u64,
    fuzz: Option<u64>,
    ci_check: Option<&Path>,
    threshold: f64,
    demo: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(fuzz_count) = fuzz {
        println!("🎲 Running invariant fuzzing over {} executions...", fuzz_count);
    }

    let trace = if let Some(path) = elf_path {
        println!("🔍 Loading sBPF binary & DWARF debug info: {}", path.display());
        let resolver = DwarfSymbolResolver::from_file(path)?;
        println!("   ✓ Loaded {} symbols / address ranges from DWARF/ELF", resolver.symbol_map().len());

        SvmInstructionSimulator::simulate_scenario(ProfileScenario::AnchorEscrowDeposit)
    } else {
        if !demo {
            println!("💡 No --elf path provided. Running standard Anchor Escrow benchmark profile.");
        }
        SvmInstructionSimulator::simulate_scenario(ProfileScenario::AnchorEscrowDeposit)
    };

    let mut aggregator = FoldedStackAggregator::new();
    aggregator.aggregate_trace(&trace);

    let summary = aggregator.summary(instruction);

    TerminalPresenter::print_summary(&summary, Some(max_cu));

    let renderer = FlameRenderer::new(format!("Solana CU Profile: {}", instruction));
    renderer.render_to_file(&aggregator, output_svg)?;
    println!("🔥 Interactive SVG Flamegraph written to: {}", output_svg.display());

    let profile = CuProfile::from(&summary);
    profile.save_to_file(output_json)?;
    println!("📊 CI/CD Profile JSON written to:          {}", output_json.display());

    if let Some(base_path) = ci_check {
        println!("\n⚖️  Running CI/CD Regression Check against: {}", base_path.display());
        let base_profile = CuProfile::load_from_file(base_path)?;
        let report = CiRegressionReport::compare(&base_profile, &profile, threshold);

        if report.passed {
            println!("   ✅ PASS: {}", report.reason);
        } else {
            eprintln!("   ❌ FAIL: {}", report.reason);
            std::process::exit(1);
        }
    }

    Ok(())
}

pub fn run_demo(
    scenario: ProfileScenario,
    output_svg: &Path,
    output_json: &Path,
    max_cu: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let trace = SvmInstructionSimulator::simulate_scenario(scenario);
    let mut aggregator = FoldedStackAggregator::new();
    aggregator.aggregate_trace(&trace);

    let summary = aggregator.summary(&trace.instruction_name);
    TerminalPresenter::print_summary(&summary, Some(max_cu));

    let renderer = FlameRenderer::new(format!("Demo Flamegraph: {}", trace.instruction_name));
    renderer.render_to_file(&aggregator, output_svg)?;
    println!("🔥 Interactive SVG Flamegraph saved: {}", output_svg.display());

    let profile = CuProfile::from(&summary);
    profile.save_to_file(output_json)?;
    println!("📊 Profile JSON saved:               {}", output_json.display());

    Ok(())
}

pub fn dump_symbols(elf_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Inspecting ELF & DWARF symbols in: {}", elf_path.display());
    let resolver = DwarfSymbolResolver::from_file(elf_path)?;
    println!("   Total indexed symbols: {}", resolver.symbol_map().len());
    Ok(())
}

pub fn run_compare(base: &Path, current: &Path, threshold: f64) -> Result<(), Box<dyn std::error::Error>> {
    let base_prof = CuProfile::load_from_file(base)?;
    let curr_prof = CuProfile::load_from_file(current)?;

    if base_prof.instruction != curr_prof.instruction {
        eprintln!(
            "⚠️  Warning: Comparing profiles for different instructions ('{}' vs '{}')",
            base_prof.instruction, curr_prof.instruction
        );
    }

    let report = CiRegressionReport::compare(&base_prof, &curr_prof, threshold);

    println!("\n╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║  ⚖️  sol-flame CI Regression Evaluation                                        ║");
    println!("╠══════════════════════════════════════════════════════════════════════════════╣");
    println!("║  Instruction:      {:<56}║", report.instruction);
    println!("║  Base CU:          {:>10} CU                                              ║", report.base_cu);
    println!("║  Current CU:       {:>10} CU                                              ║", report.current_cu);
    println!("║  Delta:            {:>+10} CU ({:>+6.2}%)                                  ║", report.diff_cu, report.percent_change);
    println!("║  Tolerance:        {:>10.2}%                                              ║", report.max_allowed_percent);
    println!("╠══════════════════════════════════════════════════════════════════════════════╣");
    if report.passed {
        println!("║  Status:           ✅ PASSED                                                 ║");
        println!("║  Result:           {:<56}║", report.reason);
    } else {
        println!("║  Status:           ❌ FAILED (REGRESSION EXCEEDED)                           ║");
        println!("║  Result:           {:<56}║", report.reason);
    }
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    if !report.passed {
        std::process::exit(1);
    }
    Ok(())
}
