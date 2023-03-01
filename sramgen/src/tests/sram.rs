use std::collections::HashSet;

use crate::config::sram::SramConfig;
use crate::plan::extract::ExtractionResult;
use crate::plan::{execute_plan, generate_plan, ExecutePlanParams};
use crate::tests::test_work_dir;
use crate::Result;

macro_rules! generate_sram_test {
    ( $num_words:expr, $data_width:expr, $mux_ratio:expr, $write_size:expr, ControlMode::Simple ) => {
        paste::paste! {
            #[test]
            fn [<test_sram_ $num_words x $data_width m $mux_ratio w $write_size _simple>]() -> Result<()> {
                crate::tests::sram::test_sram(&crate::config::sram::SramConfig {
                    num_words: $num_words,
                    data_width: $data_width,
                    mux_ratio: $mux_ratio,
                    write_size: $write_size,
                    control: crate::config::sram::ControlMode::Simple,
                })
            }
        }
    };
    ( $num_words:expr, $data_width:expr, $mux_ratio:expr, $write_size:expr, ControlMode::ReplicaV1 ) => {
        paste::paste! {
            #[test]
            fn [<test_sram_ $num_words x $data_width m $mux_ratio w $write_size _replica_v1>]() -> Result<()> {
                crate::tests::sram::test_sram(&crate::config::sram::SramConfig {
                    num_words: $num_words,
                    data_width: $data_width,
                    mux_ratio: $mux_ratio,
                    write_size: $write_size,
                    control: crate::config::sram::ControlMode::ReplicaV1,
                    #[cfg(feature = "commercial")]
                    pex_level: Some(calibre::pex::PexLevel::Rc),
                })
            }
        }
    };
}

pub(crate) use generate_sram_test;

pub(crate) fn test_sram(config: &SramConfig) -> Result<()> {
    let plan = generate_plan(ExtractionResult {}, config)?;
    let name = &plan.sram_params.name;

    let work_dir = test_work_dir(name);

    execute_plan(ExecutePlanParams {
        work_dir: &work_dir,
        plan: &plan,
        tasks: &HashSet::new(),
        ctx: None,
        pex_level: Some(calibre::pex::PexLevel::Rc),
    })?;

    Ok(())
}

// Mux ratio 2 is not supported for now; we still need to add address buffers.

// Small SRAMS for testing
generate_sram_test!(32, 8, 2, 8, ControlMode::ReplicaV1);
generate_sram_test!(32, 8, 2, 4, ControlMode::ReplicaV1);
generate_sram_test!(32, 32, 2, 4, ControlMode::ReplicaV1);
generate_sram_test!(32, 32, 2, 8, ControlMode::ReplicaV1);
generate_sram_test!(32, 32, 2, 16, ControlMode::ReplicaV1);
generate_sram_test!(64, 2, 4, 2, ControlMode::ReplicaV1);
generate_sram_test!(64, 8, 4, 4, ControlMode::ReplicaV1);

// 1 kbyte, 64-bit word width
generate_sram_test!(128, 64, 2, 8, ControlMode::ReplicaV1);
generate_sram_test!(128, 64, 4, 8, ControlMode::ReplicaV1);
generate_sram_test!(128, 64, 4, 16, ControlMode::ReplicaV1);
generate_sram_test!(128, 64, 4, 32, ControlMode::ReplicaV1);
generate_sram_test!(128, 64, 8, 8, ControlMode::ReplicaV1);
generate_sram_test!(128, 64, 2, 64, ControlMode::ReplicaV1);

// 1 kbyte, 32-bit word width
generate_sram_test!(256, 32, 2, 8, ControlMode::ReplicaV1);
generate_sram_test!(256, 32, 4, 8, ControlMode::ReplicaV1);
generate_sram_test!(256, 32, 8, 8, ControlMode::ReplicaV1);
generate_sram_test!(256, 32, 4, 32, ControlMode::ReplicaV1);

// 16 kbyte
generate_sram_test!(4096, 32, 8, 8, ControlMode::ReplicaV1);
