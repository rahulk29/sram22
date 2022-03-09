use crate::error::Result;
use magic_vlsi::MagicInstance;

use crate::config::TechConfig;

use super::PrechargeSize;

pub fn generate_precharge(
    _m: &mut MagicInstance,
    _tc: &TechConfig,
    _params: PrechargeSize,
) -> Result<()> {
    unimplemented!()
}
