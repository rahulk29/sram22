use crate::config::{ControlMode, SramConfig};

use crate::Result;

use super::generate_test;

#[test]
fn test_sram_8x32m2w8_simple() -> Result<()> {
    generate_test(&SramConfig {
        num_words: 32,
        data_width: 8,
        mux_ratio: 2,
        write_size: 8,
        control: ControlMode::Simple,
    })
}

#[test]
fn test_sram_16x64m2w16_simple() -> Result<()> {
    generate_test(&SramConfig {
        num_words: 64,
        data_width: 16,
        mux_ratio: 2,
        write_size: 16,
        control: ControlMode::Simple,
    })
}

#[test]
fn test_sram_16x64m2w8_simple() -> Result<()> {
    generate_test(&SramConfig {
        num_words: 64,
        data_width: 16,
        mux_ratio: 2,
        write_size: 8,
        control: ControlMode::Simple,
    })
}

#[test]
fn test_sram_16x64m2w4_simple() -> Result<()> {
    generate_test(&SramConfig {
        num_words: 64,
        data_width: 16,
        mux_ratio: 2,
        write_size: 4,
        control: ControlMode::Simple,
    })
}

#[test]
fn test_sram_16x64m2w2_simple() -> Result<()> {
    generate_test(&SramConfig {
        num_words: 64,
        data_width: 16,
        mux_ratio: 2,
        write_size: 2,
        control: ControlMode::Simple,
    })
}

#[test]
fn test_sram_8x128m4w8_simple() -> Result<()> {
    generate_test(&SramConfig {
        num_words: 128,
        data_width: 8,
        mux_ratio: 4,
        write_size: 8,
        control: ControlMode::Simple,
    })
}

#[test]
fn test_sram_8x128m4w2_simple() -> Result<()> {
    generate_test(&SramConfig {
        num_words: 128,
        data_width: 8,
        mux_ratio: 4,
        write_size: 2,
        control: ControlMode::Simple,
    })
}

#[test]
fn test_sram_4x256m8w4_simple() -> Result<()> {
    generate_test(&SramConfig {
        num_words: 256,
        data_width: 4,
        mux_ratio: 8,
        write_size: 4,
        control: ControlMode::Simple,
    })
}

#[test]
fn test_sram_4x256m8w2_simple() -> Result<()> {
    generate_test(&SramConfig {
        num_words: 256,
        data_width: 4,
        mux_ratio: 8,
        write_size: 2,
        control: ControlMode::Simple,
    })
}

#[test]
fn test_sram_32x256m2w32_simple() -> Result<()> {
    generate_test(&SramConfig {
        num_words: 256,
        data_width: 32,
        mux_ratio: 2,
        write_size: 32,
        control: ControlMode::Simple,
    })
}

#[test]
fn test_sram_64x128m2w64_simple() -> Result<()> {
    generate_test(&SramConfig {
        num_words: 128,
        data_width: 64,
        mux_ratio: 2,
        write_size: 64,
        control: ControlMode::Simple,
    })
}

#[test]
fn test_sram_64x128m2w32_simple() -> Result<()> {
    generate_test(&SramConfig {
        num_words: 128,
        data_width: 64,
        mux_ratio: 2,
        write_size: 32,
        control: ControlMode::Simple,
    })
}

#[test]
fn test_sram_64x128m2w2_simple() -> Result<()> {
    generate_test(&SramConfig {
        num_words: 128,
        data_width: 64,
        mux_ratio: 2,
        write_size: 2,
        control: ControlMode::Simple,
    })
}
