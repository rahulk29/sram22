use std::path::Path;

use crate::Result;

use anyhow::anyhow;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use tera::{Context, Tera};

lazy_static! {
    pub static ref TEMPLATES: std::result::Result<Tera, tera::Error> =
        Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*.v"));
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Sram1RwParams {
    module_name: String,
    num_words: usize,
    data_width: usize,
    addr_width: usize,
}

pub fn generate_1rw_verilog(params: Sram1RwParams) -> Result<String> {
    assert_eq!(params.num_words, 1 << params.addr_width);
    Ok(TEMPLATES
        .as_ref()
        .map_err(|e| anyhow!("Failed to render templates: {e}"))?
        .render("sram_1rw.v", &Context::from_serialize(params)?)?)
}

pub fn save_1rw_verilog(path: impl AsRef<Path>, params: Sram1RwParams) -> Result<()> {
    let verilog = generate_1rw_verilog(params)?;

    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, verilog)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::utils::test_verilog_path;

    use super::*;

    #[test]
    fn test_verilog_sram_16x16m2() -> Result<(), Box<dyn std::error::Error>> {
        let name = String::from("sramgen_sram_16x16m2");
        save_1rw_verilog(
            test_verilog_path(&name),
            Sram1RwParams {
                module_name: name,
                num_words: 32,
                data_width: 8,
                addr_width: 5,
            },
        )
        .unwrap();
        Ok(())
    }

    #[test]
    fn test_verilog_sram_32x32m2() -> Result<(), Box<dyn std::error::Error>> {
        let name = String::from("sramgen_sram_32x32m2");
        save_1rw_verilog(
            test_verilog_path(&name),
            Sram1RwParams {
                module_name: name,
                num_words: 64,
                data_width: 16,
                addr_width: 6,
            },
        )?;
        Ok(())
    }

    #[test]
    fn test_verilog_sram_32x32m4() -> Result<(), Box<dyn std::error::Error>> {
        let name = String::from("sramgen_sram_32x32m4");
        save_1rw_verilog(
            test_verilog_path(&name),
            Sram1RwParams {
                module_name: name,
                num_words: 128,
                data_width: 8,
                addr_width: 7,
            },
        )?;
        Ok(())
    }

    #[test]
    fn test_verilog_sram_32x32m8() -> Result<(), Box<dyn std::error::Error>> {
        let name = String::from("sramgen_sram_32x32m8");
        save_1rw_verilog(
            test_verilog_path(&name),
            Sram1RwParams {
                module_name: name,
                num_words: 256,
                data_width: 4,
                addr_width: 8,
            },
        )?;
        Ok(())
    }
}
