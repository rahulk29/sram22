use crate::config::sram::SramConfig;
use crate::plan::extract::ExtractionResult;
use crate::plan::{execute_plan, generate_plan};
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
}

pub(crate) use generate_sram_test;

pub(crate) fn test_sram(config: &SramConfig) -> Result<()> {
    let plan = generate_plan(ExtractionResult {}, config)?;
    let name = &plan.sram_params.name;

    let work_dir = test_work_dir(name);
    execute_plan(&work_dir, &plan, None)?;

    #[cfg(feature = "calibre")]
    {
        crate::verification::calibre::run_sram_drc(&work_dir, name)?;
        crate::verification::calibre::run_sram_lvs(&work_dir, name)?;
        #[cfg(feature = "pex")]
        crate::verification::calibre::run_sram_pex(&work_dir, name)?;
    }

    #[cfg(feature = "spectre")]
    crate::verification::spectre::run_sram_spectre(&plan.sram_params, &work_dir, name)?;

    Ok(())
}

generate_sram_test!(32, 8, 2, 8, ControlMode::Simple);
generate_sram_test!(64, 16, 2, 16, ControlMode::Simple);
generate_sram_test!(64, 16, 2, 8, ControlMode::Simple);
generate_sram_test!(64, 16, 2, 4, ControlMode::Simple);
generate_sram_test!(64, 16, 2, 2, ControlMode::Simple);
generate_sram_test!(128, 8, 4, 8, ControlMode::Simple);
generate_sram_test!(128, 8, 4, 2, ControlMode::Simple);
generate_sram_test!(256, 4, 8, 4, ControlMode::Simple);
generate_sram_test!(256, 4, 8, 2, ControlMode::Simple);
generate_sram_test!(256, 32, 2, 32, ControlMode::Simple);
generate_sram_test!(128, 64, 2, 64, ControlMode::Simple);
generate_sram_test!(128, 64, 2, 32, ControlMode::Simple);
generate_sram_test!(128, 64, 2, 2, ControlMode::Simple);
