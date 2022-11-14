use crate::layout::guard_ring::*;
use crate::utils::test_gds_path;
use crate::Result;
use layout21::raw::{Point, Rect};
use pdkprims::tech::sky130;

#[test]
fn square_200um() -> Result<()> {
    let mut lib = sky130::pdk_lib("test_guard_ring_square_200um")?;
    draw_guard_ring(
        &mut lib,
        GuardRingParams {
            enclosure: Rect::new(Point::zero(), Point::new(200_000, 200_000)),
            prefix: "test_guard_ring_square_200um".to_string(),
        },
    )?;

    lib.save_gds(test_gds_path(&lib))?;

    Ok(())
}
