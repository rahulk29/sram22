use derive_builder::Builder;
use layout21::raw::{Dir, Int, Point, Span};
use serde::{Deserialize, Serialize};

#[derive(Builder)]
pub struct Grid {
    line: Int,
    space: Int,
    center: Point,
    grid: Int,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum TrackLocator {
    /// The track nearest a position.
    Nearest,
    /// The track nearest a position that starts beyond that position.
    StartsBeyond,
    /// The track nearest a position that ends before that position.
    EndsBefore,
}

impl Grid {
    /// The `i`-th track running in the given direction.
    pub fn track(&self, dir: Dir, i: isize) -> Span {
        let start = self.center.coord(!dir) - self.line / 2;
        let tstart = start + i * (self.line + self.space);

        assert_eq!(tstart % self.grid, 0);
        debug_assert_eq!(self.line % self.grid, 0);

        Span::new(tstart, tstart + self.line)
    }

    /// The `i`-th horizontal (East to West / West to East) track.
    #[inline]
    pub fn htrack(&self, i: isize) -> Span {
        self.track(Dir::Horiz, i)
    }

    /// The `i`-th vertical (North to South / South to North) track.
    #[inline]
    pub fn vtrack(&self, i: isize) -> Span {
        self.track(Dir::Vert, i)
    }

    /// Gets the index of the track in the given direction nearest to `pos`.
    pub fn get_track_index(&self, dir: Dir, pos: Int, loc: TrackLocator) -> Int {
        let m = self.line + self.space;
        let mut idx = round(pos - self.center.coord(!dir), m) / m;
        println!("pos = {pos}, idx = {idx}");

        // FIXME: should not need a loop here...
        let idx = loop {
            let track = self.track(dir, idx);
            match loc {
                TrackLocator::Nearest => break idx,
                TrackLocator::StartsBeyond => {
                    if pos > track.start() {
                        idx += 1;
                    } else {
                        break idx;
                    }
                }
                TrackLocator::EndsBefore => {
                    if track.stop() <= pos {
                        break idx;
                    } else {
                        idx -= 1;
                    }
                }
            }
        };

        let span = self.track(dir, idx);
        match loc {
            TrackLocator::EndsBefore => {
                debug_assert!(span.stop() <= pos);
            }
            TrackLocator::StartsBeyond => {
                debug_assert!(span.start() >= pos);
            }
            _ => (),
        };

        idx
    }

    /// Gets the track in the given direction nearest to `pos`.
    pub fn get_track(&self, dir: Dir, pos: Int, loc: TrackLocator) -> Span {
        self.track(dir, self.get_track_index(dir, pos, loc))
    }

    #[inline]
    pub fn builder() -> GridBuilder {
        GridBuilder::default()
    }
}

pub(crate) fn round(x: Int, multiple: Int) -> Int {
    let a = (x / multiple) * multiple;
    let b = a + multiple;
    if x - a > b - x {
        b
    } else {
        a
    }
}
