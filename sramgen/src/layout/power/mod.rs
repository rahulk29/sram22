use std::collections::HashMap;

use derive_builder::Builder;
use layout21::raw::{BoundBoxTrait, Dir, Instance, Int, Point, Rect, Span};

use pdkprims::{LayerIdx, Pdk};

use super::route::grid::{Grid, TrackLocator};
use super::route::{Router, Trace};

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

pub struct PowerStraps {
    pub instance: Instance,
    pub left: Vec<(PowerSource, Rect)>,
    pub right: Vec<(PowerSource, Rect)>,
    pub bottom: Vec<(PowerSource, Rect)>,
    pub top: Vec<(PowerSource, Rect)>,

    pub h_traces: Vec<(PowerSource, Rect)>,
    pub v_traces: Vec<(PowerSource, Rect)>,
}

impl PowerStrapGen {
    pub fn new(opts: &PowerStrapOpts) -> Self {
        Self {
            h_metal: opts.h_metal,
            v_metal: opts.v_metal,
            h_line: opts.h_line,
            h_space: opts.h_space,
            v_line: opts.v_line,
            v_space: opts.v_space,
            pdk: opts.pdk.clone(),
            enclosure: opts.enclosure,

            router: Router::new(&opts.name, opts.pdk.clone()),
            blockages: HashMap::new(),
            vdd_targets: HashMap::new(),
            gnd_targets: HashMap::new(),
        }
    }

    pub fn set_enclosure(&mut self, enclosure: impl Into<Rect>) {
        self.enclosure = enclosure.into();
    }

    pub fn add_vdd_target(&mut self, layer: LayerIdx, rect: Rect) {
        let targets = self.vdd_targets.entry(layer).or_default();
        targets.push(rect);
    }

    pub fn add_gnd_target(&mut self, layer: LayerIdx, rect: Rect) {
        let targets = self.gnd_targets.entry(layer).or_default();
        targets.push(rect);
    }

    pub fn add_blockage(&mut self, layer: LayerIdx, rect: impl Into<Rect>) {
        let blockages = self.blockages.entry(layer).or_default();
        blockages.push(rect.into());
    }

    #[inline]
    pub fn add_padded_blockage(&mut self, layer: LayerIdx, rect: impl Into<Rect>) {
        self.add_blockage(layer, rect.into().expand(self.router.cfg().space(layer)));
    }

    pub fn generate(mut self) -> crate::Result<PowerStraps> {
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
        let v_end = v_grid.get_track_index(
            Dir::Vert,
            self.enclosure.right(),
            TrackLocator::StartsBeyond,
        );

        let grid_bounds = Rect::new(
            Point::new(
                v_grid.vtrack(v_start).start(),
                h_grid.htrack(h_start).start(),
            ),
            Point::new(v_grid.vtrack(v_end).stop(), h_grid.htrack(h_end).stop()),
        );

        let mut state = GenState {
            h_grid,
            v_grid,
            h_trace_range: (h_start, h_end),
            v_trace_range: (v_start, v_end),
        };

        assert!(h_end > h_start);

        let mut h_traces = self.draw_traces(&mut state, Dir::Horiz);
        let mut v_traces = self.draw_traces(&mut state, Dir::Vert);

        self.connect_traces(&mut h_traces, &mut v_traces);
        let h_rects = h_traces.iter().map(|(src, trace)| (*src, trace.rect()));
        let v_rects = v_traces.iter().map(|(src, trace)| (*src, trace.rect()));
        let left = h_rects
            .clone()
            .filter(|(_, rect)| rect.left() == grid_bounds.left())
            .collect::<Vec<_>>();
        let right = h_rects
            .filter(|(_, rect)| rect.right() == grid_bounds.right())
            .collect::<Vec<_>>();
        let bottom = v_rects
            .clone()
            .filter(|(_, rect)| rect.bottom() == grid_bounds.bottom())
            .collect::<Vec<_>>();
        let top = v_rects
            .filter(|(_, rect)| rect.top() == grid_bounds.top())
            .collect::<Vec<_>>();

        assert!(left.len() >= 2);
        assert!(right.len() >= 2);
        assert!(bottom.len() >= 2);
        assert!(top.len() >= 2);

        Ok(PowerStraps {
            instance: self.router.finish(),
            left,
            right,
            bottom,
            top,
            h_traces: h_traces
                .into_iter()
                .map(|(src, trace)| (src, trace.rect()))
                .collect(),
            v_traces: v_traces
                .into_iter()
                .map(|(src, trace)| (src, trace.rect()))
                .collect(),
        })
    }

    fn draw_traces(&mut self, state: &mut GenState, dir: Dir) -> Vec<(PowerSource, Trace)> {
        // Variables starting with an x generally represent quantities
        // for the transverse direction (ie. !dir).
        let (start, end) = state.trace_range(dir);
        let (xstart, xend) = state.trace_range(!dir);
        let metal = self.metal_layer(dir);

        assert!(end >= start);
        assert!(xend >= xstart);

        let mut traces = Vec::with_capacity((end - start + 1) as usize);

        for i in start..=end {
            let source = self.idx_to_source(i);
            let mut trace_span = None;
            let xspan = state.grid(dir).track(dir, i);
            for j in xstart..xend {
                let span = Span::new(
                    state.grid(!dir).track(!dir, j).start(),
                    state.grid(!dir).track(!dir, j + 1).stop(),
                );
                let rect = Rect::span_builder()
                    .with(dir, span)
                    .with(!dir, xspan)
                    .build();
                if !self.is_blocked(metal, rect) {
                    match trace_span {
                        Some(s) => trace_span = Some(Span::merge([s, span])),
                        None => trace_span = Some(span),
                    };
                } else if let Some(span) = trace_span {
                    let rect = Rect::span_builder()
                        .with(dir, span)
                        .with(!dir, xspan)
                        .build();
                    let mut trace = self.router.trace(rect, metal);
                    self.contact_targets(metal - 1, source, &mut trace);
                    traces.push((source, trace));
                    trace_span = None;
                }

                if j == xend - 1 && trace_span.is_some() {
                    let span = trace_span.unwrap();
                    let rect = Rect::span_builder()
                        .with(dir, span)
                        .with(!dir, xspan)
                        .build();
                    let mut trace = self.router.trace(rect, metal);
                    self.contact_targets(metal - 1, source, &mut trace);
                    traces.push((source, trace));
                    trace_span = None;
                }

                /*
                if h_trace.is_some() && v_trace.is_some() && h_source == v_source {
                    // FIXME assumes the vertical metal is above the horizontal metal
                    v_trace.unwrap().contact_down(h_trace.unwrap().rect());
                }
                */
            }
        }

        traces
    }

    fn connect_traces(&mut self, h: &mut [(PowerSource, Trace)], v: &mut [(PowerSource, Trace)]) {
        for (asrc, atrace) in h.iter_mut() {
            for (bsrc, btrace) in v.iter_mut() {
                if asrc != bsrc {
                    continue;
                }

                if !atrace.rect().intersection(&btrace.rect().into()).is_empty() {
                    if self.h_metal > self.v_metal {
                        atrace.contact_down(btrace.rect());
                    } else {
                        btrace.contact_down(atrace.rect());
                    }
                }
            }
        }
    }

    fn metal_layer(&self, dir: Dir) -> LayerIdx {
        match dir {
            Dir::Horiz => self.h_metal,
            Dir::Vert => self.v_metal,
        }
    }

    fn idx_to_source(&self, idx: isize) -> PowerSource {
        match idx.rem_euclid(2) {
            0 => PowerSource::Vdd,
            1 => PowerSource::Gnd,
            _ => unreachable!(),
        }
    }

    fn is_blocked(&self, layer: LayerIdx, rect: Rect) -> bool {
        let bbox = rect.into();
        if let Some(blockages) = self.blockages.get(&layer) {
            for block in blockages {
                if !block.intersection(&bbox).is_empty() {
                    return true;
                }
            }
        }
        false
    }

    fn contact_targets(&self, layer: LayerIdx, source: PowerSource, trace: &mut Trace) {
        // TODO need to track which targets have been hit.
        let rect = trace.rect();
        let shorter_dir = rect.shorter_dir();
        let short_width = rect.span(shorter_dir).length();
        for target in self.targets(source, layer) {
            let intersection = rect.intersection(&target.bbox());
            if !intersection.is_empty() {
                let intersection = intersection.into_rect();
                if intersection.span(shorter_dir).length() >= short_width / 2 {
                    trace.contact_down(*target);
                }
            }
        }
    }

    #[inline]
    fn targets(&self, source: PowerSource, layer: LayerIdx) -> &Vec<Rect> {
        match source {
            PowerSource::Vdd => self.vdd_targets.get(&layer).unwrap_or(&EMPTY_VEC),
            PowerSource::Gnd => self.gnd_targets.get(&layer).unwrap_or(&EMPTY_VEC),
        }
    }
}

static EMPTY_VEC: Vec<Rect> = Vec::new();

struct GenState {
    h_grid: Grid,
    v_grid: Grid,
    h_trace_range: (isize, isize),
    v_trace_range: (isize, isize),
}

impl GenState {
    #[inline]
    fn grid(&self, dir: Dir) -> &Grid {
        match dir {
            Dir::Vert => &self.v_grid,
            Dir::Horiz => &self.h_grid,
        }
    }

    #[inline]
    fn trace_range(&self, dir: Dir) -> (isize, isize) {
        match dir {
            Dir::Horiz => self.h_trace_range,
            Dir::Vert => self.v_trace_range,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum PowerSource {
    Vdd,
    Gnd,
}

impl PowerStrapOpts {
    #[inline]
    pub fn builder() -> PowerStrapOptsBuilder {
        PowerStrapOptsBuilder::default()
    }
}
