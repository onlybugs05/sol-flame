use sol_flame_core::symbols::{FrameCategory, ResolvedFrame, SourceLocation};
use sol_flame_core::trace::ExecutionTrace;
use crate::tracer::MolluskComputeTracer;

/// Standard benchmark scenarios showcasing real Solana programs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileScenario {
    /// Anchor Escrow: Demonstrates heavy Borsh account deserialization (59% CU), CPI to SPL Token, and math.
    AnchorEscrowDeposit,
    /// Pinocchio Vault: Demonstrates zero-copy account parsing reducing CU consumption by >75%.
    PinocchioVaultSwap,
    /// High-Memory Borsh Vector: Allocates 26KB on heap, triggering the 75% heap overflow warning.
    HighHeapWarning,
}

/// In-process SVM execution simulation & profile generator.
/// Allows developers and grant reviewers to profile Solana instructions immediately
/// without requiring a live validator or external test cluster.
pub struct SvmInstructionSimulator;

impl SvmInstructionSimulator {
    /// Execute a simulated profile scenario with exact CU accounting.
    pub fn simulate_scenario(scenario: ProfileScenario) -> ExecutionTrace {
        match scenario {
            ProfileScenario::AnchorEscrowDeposit => Self::simulate_anchor_escrow_deposit(),
            ProfileScenario::PinocchioVaultSwap => Self::simulate_pinocchio_vault_swap(),
            ProfileScenario::HighHeapWarning => Self::simulate_high_heap_warning(),
        }
    }

    fn simulate_anchor_escrow_deposit() -> ExecutionTrace {
        let mut tracer = MolluskComputeTracer::new("deposit", 200_000);

        // 1. Program Entrypoint (Total: 48,190 CU)
        let entry = ResolvedFrame {
            symbol: "entrypoint".to_string(),
            demangled: "entrypoint".to_string(),
            location: Some(SourceLocation {
                file: "programs/anchor-escrow/src/lib.rs".to_string(),
                line: 12,
                column: 1,
            }),
            category: FrameCategory::Native,
            is_inlined: false,
        };
        tracer.push_frame(0x1000, entry);
        tracer.on_heap_alloc(512);

        // 1.1 Instruction Router
        let router = ResolvedFrame {
            symbol: "anchor_escrow::process_instruction".to_string(),
            demangled: "anchor_escrow::process_instruction".to_string(),
            location: Some(SourceLocation {
                file: "programs/anchor-escrow/src/lib.rs".to_string(),
                line: 28,
                column: 5,
            }),
            category: FrameCategory::Native,
            is_inlined: false,
        };
        tracer.push_frame(0x1100, router);

        // 1.2 Borsh Deserialization of Accounts (28,540 CU total)
        // UserState::try_from (14,200 CU)
        let deser_user = ResolvedFrame {
            symbol: "anchor_lang::AccountDeserialize::try_deserialize_user_state".to_string(),
            demangled: "anchor_lang::AccountDeserialize::try_deserialize (UserState)".to_string(),
            location: Some(SourceLocation {
                file: "anchor-lang/src/account.rs".to_string(),
                line: 18,
                column: 4,
            }),
            category: FrameCategory::Deserialization,
            is_inlined: false,
        };
        tracer.push_frame(0x2000, deser_user);
        tracer.on_heap_alloc(1800);
        // Step remaining CU down by 14,200
        let cur = tracer.remaining_cu();
        tracer.on_step(0x2000, 0x95, cur - 14_200);

        // VaultState::try_from (14,340 CU)
        let deser_vault = ResolvedFrame {
            symbol: "anchor_lang::AccountDeserialize::try_deserialize_vault_state".to_string(),
            demangled: "anchor_lang::AccountDeserialize::try_deserialize (VaultState)".to_string(),
            location: Some(SourceLocation {
                file: "anchor-lang/src/account.rs".to_string(),
                line: 18,
                column: 4,
            }),
            category: FrameCategory::Deserialization,
            is_inlined: false,
        };
        tracer.push_frame(0x2100, deser_vault);
        tracer.on_heap_alloc(1988);
        let cur = tracer.remaining_cu();
        tracer.on_step(0x2100, 0x95, cur - 14_340);

        // 1.3 CPI Token Transfer to SPL Token (18,450 CU)
        tracer.on_cpi_start("spl_token", Some("transfer"));
        tracer.consume_cu(18_450);
        tracer.on_cpi_end();

        // 1.4 Native Math / Fee calculation (1,200 CU)
        let math_frame = ResolvedFrame {
            symbol: "anchor_escrow::math::calculate_deposit_fee".to_string(),
            demangled: "anchor_escrow::math::calculate_deposit_fee".to_string(),
            location: Some(SourceLocation {
                file: "programs/anchor-escrow/src/math.rs".to_string(),
                line: 42,
                column: 1,
            }),
            category: FrameCategory::Native,
            is_inlined: false,
        };
        tracer.push_frame(0x3000, math_frame);
        let cur = tracer.remaining_cu();
        tracer.on_step(0x3000, 0x95, cur - 1_200);

        // Pop instruction router & entrypoint
        tracer.pop_frame(0x1100);
        tracer.pop_frame(0x1000);

        tracer.finish()
    }

    fn simulate_pinocchio_vault_swap() -> ExecutionTrace {
        let mut tracer = MolluskComputeTracer::new("swap", 200_000);

        let entry = ResolvedFrame {
            symbol: "entrypoint".to_string(),
            demangled: "entrypoint".to_string(),
            location: Some(SourceLocation {
                file: "programs/pinocchio-vault/src/lib.rs".to_string(),
                line: 8,
                column: 1,
            }),
            category: FrameCategory::Native,
            is_inlined: false,
        };
        tracer.push_frame(0x1000, entry);

        // Pinocchio zero-copy account view: uses only 620 CU instead of 28,540!
        let zero_copy = ResolvedFrame {
            symbol: "pinocchio::account_info::AccountView::load".to_string(),
            demangled: "pinocchio::account_info::AccountView::load (zero-copy)".to_string(),
            location: Some(SourceLocation {
                file: "pinocchio/src/account.rs".to_string(),
                line: 14,
                column: 5,
            }),
            category: FrameCategory::Native,
            is_inlined: false,
        };
        tracer.push_frame(0x1050, zero_copy);
        let cur = tracer.remaining_cu();
        tracer.on_step(0x1050, 0x95, cur - 620);

        // CPI transfer: 18,450 CU
        tracer.on_cpi_start("spl_token_2022", Some("transfer_checked"));
        tracer.consume_cu(18_450);
        tracer.on_cpi_end();

        // Swap math: 1,480 CU
        let math = ResolvedFrame {
            symbol: "pinocchio_vault::math::constant_product_curve".to_string(),
            demangled: "pinocchio_vault::math::constant_product_curve".to_string(),
            location: Some(SourceLocation {
                file: "programs/pinocchio-vault/src/math.rs".to_string(),
                line: 60,
                column: 1,
            }),
            category: FrameCategory::Native,
            is_inlined: false,
        };
        tracer.push_frame(0x2000, math);
        let cur = tracer.remaining_cu();
        tracer.on_step(0x2000, 0x95, cur - 1_480);

        tracer.pop_frame(0x1000);
        tracer.finish()
    }

    fn simulate_high_heap_warning() -> ExecutionTrace {
        let mut tracer = MolluskComputeTracer::new("process_large_batch", 200_000);

        let entry = ResolvedFrame {
            symbol: "entrypoint".to_string(),
            demangled: "entrypoint".to_string(),
            location: None,
            category: FrameCategory::Native,
            is_inlined: false,
        };
        tracer.push_frame(0x1000, entry);

        // Allocate 26,000 bytes on heap (exceeding 24KB threshold)
        tracer.on_heap_alloc(26_000);
        tracer.consume_cu(35_000);

        tracer.pop_frame(0x1000);
        tracer.finish()
    }
}
