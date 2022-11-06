pub mod read;
pub mod write;

#[cfg(test)]
mod tests {
    use pdkprims::tech::sky130;

    use crate::tech::BITCELL_WIDTH;
    use crate::utils::test_path;
    use crate::Result;

    use super::read::*;
    use super::write::*;
    

    #[test]
    fn test_sky130_column_read_mux() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_column_read_mux")?;
        draw_read_mux(&mut lib)?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }

    #[test]
    fn test_sky130_column_read_mux_array() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_column_read_mux_array")?;
        draw_read_mux_array(&mut lib, 32, 2)?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }

    #[test]
    fn test_sky130_column_read_mux_8_array() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_column_read_mux_8_array")?;
        draw_read_mux_array(&mut lib, 32, 8)?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }

    #[test]
    fn test_sky130_column_read_mux_2_array() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_column_read_mux_2_array")?;
        draw_read_mux_array(&mut lib, 32, 2)?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }

    #[test]
    fn test_sky130_column_write_mux() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_column_write_mux")?;
        draw_write_mux(
            &mut lib,
            WriteMuxParams {
                width: BITCELL_WIDTH,
                wmask: false,
            },
        )?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }

    #[test]
    fn test_sky130_column_write_mux_array_m2() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_column_write_mux_array_m2")?;
        draw_write_mux_array(&mut lib, 32, 2, 1)?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }

    #[test]
    fn test_sky130_column_write_mux_array_m4() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_column_write_mux_array_m4")?;
        draw_write_mux_array(&mut lib, 32, 4, 1)?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }

    #[test]
    fn test_sky130_column_write_mux_array_m8() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_column_write_mux_array_m8")?;
        draw_write_mux_array(&mut lib, 32, 8, 1)?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }

    #[test]
    fn test_sky130_column_write_mux_array_m4w4() -> Result<()> {
        let mut lib = sky130::pdk_lib("test_sky130_column_write_mux_array_m4w4")?;
        draw_write_mux_array(&mut lib, 128, 4, 4)?;

        lib.save_gds(test_path(&lib))?;

        Ok(())
    }
}
