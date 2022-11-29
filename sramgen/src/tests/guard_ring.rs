use crate::layout::guard_ring::*;
use crate::paths::out_gds;
use crate::tests::test_work_dir;
use crate::Result;
use layout21::raw::{Point, Rect};
use pdkprims::tech::sky130;

#[test]
fn square_200um() -> Result<()> {
    let name = "sramgen_guard_ring_square_200um";
    let mut lib = sky130::pdk_lib(name)?;
    draw_guard_ring(
        &mut lib,
        &GuardRingParams {
            name: name.to_string(),
            enclosure: Rect::new(Point::zero(), Point::new(200_000, 200_000)),
        },
    )?;

    let work_dir = test_work_dir(name);
    lib.save_gds(out_gds(work_dir, name))?;

    Ok(())
}
