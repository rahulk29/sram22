use std::collections::HashMap;

use derive_builder::Builder;
use layout21::raw::{Cell, Dir, Element, Instance, Int, Point, Rect};
use layout21::utils::Ptr;
use pdkprims::{LayerIdx, Pdk};

use super::route::grid::{Grid, TrackLocator};
use super::route::Router;

pub struct PowerStrapGen {
    h_metal: LayerIdx,
    v_metal: LayerIdx,
    h_line: Int,
    h_space: Int,
    v_line: Int,
    v_space: Int,
    pdk: Pdk,
    enclosure: Rect,

    router: Router,
    ctr: usize,
    blockages: HashMap<LayerIdx, Vec<Rect>>,
    vdd_targets: HashMap<LayerIdx, Vec<Rect>>,
    gnd_targets: HashMap<LayerIdx, Vec<Rect>>,
}

#[derive(Builder)]
pub struct PowerStrapOpts {
    h_metal: LayerIdx,
    v_metal: LayerIdx,
    h_line: Int,
    h_space: Int,
    v_line: Int,
    v_space: Int,
    pdk: Pdk,
    #[builder(setter(into))]
    name: String,
    #[builder(setter(into))]
    enclosure: Rect,
}

impl PowerStrapGen {
    pub fn new(opts: PowerStrapOpts) -> Self {
        Self {
            h_metal: opts.h_metal,
            v_metal: opts.v_metal,
            h_line: opts.h_line,
            h_space: opts.h_space,
            v_line: opts.v_line,
            v_space: opts.v_space,
            pdk: opts.pdk.clone(),
            enclosure: opts.enclosure,

            router: Router::new(opts.name, opts.pdk),
            ctr: 0,
            blockages: HashMap::new(),
            vdd_targets: HashMap::new(),
            gnd_targets: HashMap::new(),
        }
    }

    pub fn add_vdd_target(&mut self, layer: LayerIdx, rect: Rect) {
        let targets = self.vdd_targets.entry(layer).or_insert(Vec::new());
        targets.push(rect);
    }

    pub fn add_gnd_target(&mut self, layer: LayerIdx, rect: Rect) {
        let targets = self.gnd_targets.entry(layer).or_insert(Vec::new());
        targets.push(rect);
    }

    pub fn add_blockage(&mut self, layer: LayerIdx, rect: Rect) {
        let blockages = self.blockages.entry(layer).or_insert(Vec::new());
        blockages.push(rect);
    }

    pub fn generate(mut self) -> crate::Result<Instance> {
        let h_grid = Grid::builder()
            .line(self.h_line)
            .space(self.h_space)
            .center(Point::zero())
            .grid(self.pdk.grid())
            .build()?;

        let v_grid = Grid::builder()
            .line(self.v_line)
            .space(self.v_space)
            .center(Point::zero())
            .grid(self.pdk.grid())
            .build()?;

        let h_start = h_grid.get_track_index(
            Dir::Horiz,
            self.enclosure.bottom(),
            TrackLocator::EndsBefore,
        );
        let h_end =
            h_grid.get_track_index(Dir::Horiz, self.enclosure.top(), TrackLocator::StartsBeyond);

        let v_start =
            v_grid.get_track_index(Dir::Vert, self.enclosure.left(), TrackLocator::EndsBefore);
        let v_end = h_grid.get_track_index(
            Dir::Vert,
            self.enclosure.right(),
            TrackLocator::StartsBeyond,
        );

        assert!(h_end > h_start);

        for i in h_start..=h_end {
            let vspan = h_grid.htrack(i);
            let hspan = self.enclosure.span(Dir::Horiz);
            let rect = Rect::span_builder()
                .with(Dir::Vert, vspan)
                .with(Dir::Horiz, hspan)
                .build();
            self.router.trace(rect, self.h_metal);
        }

        for i in v_start..=v_end {
            let hspan = v_grid.htrack(i);
            let vspan = self.enclosure.span(Dir::Vert);
            let rect = Rect::span_builder()
                .with(Dir::Vert, vspan)
                .with(Dir::Horiz, hspan)
                .build();
            self.router.trace(rect, self.v_metal);
        }

        Ok(self.router.finish())
    }
}

impl PowerStrapOpts {
    #[inline]
    pub fn builder() -> PowerStrapOptsBuilder {
        PowerStrapOptsBuilder::default()
    }
}
