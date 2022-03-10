use std::fmt::Display;
use std::ops;

use crate::Direction;

#[derive(Default, Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct Distance {
    nm: i64,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct Area {
    nm2: i64,
}

impl Area {
    pub fn from_nm2(nm2: i64) -> Self {
        Self { nm2 }
    }
}

impl Display for Distance {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}nm", self.nm)
    }
}

impl Display for Rect {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Rect(ll: {}, ur: {})", self.ll, self.ur)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct Vec2 {
    pub x: Distance,
    pub y: Distance,
}

impl Vec2 {
    pub fn zero() -> Self {
        Self {
            x: Distance::zero(),
            y: Distance::zero(),
        }
    }

    pub fn new(x: Distance, y: Distance) -> Self {
        Self { x, y }
    }

    pub fn from_internal(x: i64, y: i64, nm_per_internal: i64) -> Self {
        Self {
            x: Distance::from_internal(x, nm_per_internal),
            y: Distance::from_internal(y, nm_per_internal),
        }
    }

    pub fn from_lambdas(x: i64, y: i64, nm_per_lambda: i64) -> Self {
        Self {
            x: Distance::from_lambdas(x, nm_per_lambda),
            y: Distance::from_lambdas(y, nm_per_lambda),
        }
    }

    pub fn from_nm(x: i64, y: i64) -> Self {
        Self {
            x: Distance::from_nm(x),
            y: Distance::from_nm(y),
        }
    }

    pub fn from_mm(x: i64, y: i64) -> Self {
        Self {
            x: Distance::from_mm(x),
            y: Distance::from_mm(y),
        }
    }

    pub fn from_um(x: i64, y: i64) -> Self {
        Self {
            x: Distance::from_um(x),
            y: Distance::from_um(y),
        }
    }

    pub fn as_internal(&self, nm_per_internal: i64) -> (i64, i64) {
        (
            self.x.as_internal(nm_per_internal),
            self.y.as_internal(nm_per_internal),
        )
    }
}

impl ops::Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl ops::Sub for Vec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Display for Vec2 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl Distance {
    #[inline]
    pub fn zero() -> Self {
        Self { nm: 0 }
    }

    #[inline]
    pub fn from_nm(nm: i64) -> Self {
        Self { nm }
    }

    #[inline]
    pub fn from_um(um: i64) -> Self {
        Self { nm: 1_000 * um }
    }

    #[inline]
    pub fn from_mm(mm: i64) -> Self {
        Self { nm: 1_000_000 * mm }
    }

    #[inline]
    pub fn from_meters(meters: i64) -> Self {
        Self {
            nm: 1_000_000_000 * meters,
        }
    }

    #[inline]
    pub fn from_internal(internal: i64, nm_per_internal: i64) -> Self {
        Self {
            nm: nm_per_internal * internal,
        }
    }

    #[inline]
    pub fn from_lambdas(lambda: i64, nm_per_lambda: i64) -> Self {
        Self {
            nm: nm_per_lambda * lambda,
        }
    }

    #[inline]
    pub fn as_lambdas(&self, nm_per_lambda: i64) -> i64 {
        self.nm / nm_per_lambda
    }

    #[inline]
    pub fn as_internal(&self, nm_per_internal: i64) -> i64 {
        self.nm / nm_per_internal
    }

    #[inline]
    pub fn nm(&self) -> i64 {
        self.nm
    }

    pub fn round_to(&self, other: Self) -> Self {
        let opts = [
            (self.nm / other.nm) * other.nm,
            (self.nm / other.nm + 1) * other.nm,
            (self.nm / other.nm - 1) * other.nm,
        ];
        let d = opts
            .into_iter()
            .min_by_key(|x| (x - self.nm).abs())
            .unwrap();
        Self::from_nm(d)
    }

    pub fn round_up_to(&self, other: Self) -> Self {
        if self.nm % other.nm == 0 {
            *self
        } else {
            let x = (self.nm / other.nm + 1) * other.nm;
            Self::from_nm(x)
        }
    }

    pub fn center_grid(a: Distance, b: Distance, grid: Distance) -> Self {
        (a + b).round_to(grid) / 2
    }
}

impl ops::Add for Distance {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            nm: self.nm + other.nm,
        }
    }
}

impl ops::Sub for Distance {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self {
            nm: self.nm - other.nm,
        }
    }
}

impl ops::Mul<i64> for Distance {
    type Output = Self;
    fn mul(self, other: i64) -> Self {
        Self {
            nm: self.nm * other,
        }
    }
}

impl ops::Mul<u64> for Distance {
    type Output = Self;
    fn mul(self, other: u64) -> Self {
        Self {
            nm: self.nm * other as i64,
        }
    }
}

impl ops::Mul<usize> for Distance {
    type Output = Self;
    fn mul(self, other: usize) -> Self {
        Self {
            nm: self.nm * other as i64,
        }
    }
}

impl ops::Div for Distance {
    type Output = i64;
    fn div(self, other: Self) -> i64 {
        self.nm / other.nm
    }
}

impl ops::Div<i64> for Distance {
    type Output = Self;
    fn div(self, other: i64) -> Self {
        Self {
            nm: self.nm / other,
        }
    }
}

impl ops::Mul<Distance> for i64 {
    type Output = Distance;
    fn mul(self, other: Distance) -> Distance {
        Distance {
            nm: self * other.nm,
        }
    }
}

impl ops::Mul for Distance {
    type Output = Area;
    fn mul(self, other: Distance) -> Area {
        Area {
            nm2: self.nm * other.nm,
        }
    }
}

impl ops::Neg for Distance {
    type Output = Self;
    fn neg(self) -> Self {
        Self { nm: -self.nm }
    }
}

impl ops::AddAssign for Distance {
    fn add_assign(&mut self, rhs: Self) {
        self.nm += rhs.nm;
    }
}

impl ops::SubAssign for Distance {
    fn sub_assign(&mut self, rhs: Self) {
        self.nm -= rhs.nm;
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Rect {
    pub ll: Vec2,
    pub ur: Vec2,
}

impl Rect {
    pub fn zero() -> Self {
        Self {
            ll: Vec2::zero(),
            ur: Vec2::zero(),
        }
    }
    pub fn from_nm(llx: i64, lly: i64, urx: i64, ury: i64) -> Self {
        assert!(urx >= llx);
        assert!(ury >= lly);
        Self {
            ll: Vec2::from_nm(llx, lly),
            ur: Vec2::from_nm(urx, ury),
        }
    }

    pub fn from_dist(llx: Distance, lly: Distance, urx: Distance, ury: Distance) -> Self {
        Self {
            ll: Vec2::new(llx, lly),
            ur: Vec2::new(urx, ury),
        }
    }

    pub fn center_wh(
        cx: Distance,
        cy: Distance,
        width: Distance,
        height: Distance,
        grid: Distance,
    ) -> Self {
        assert_eq!(width.nm() % 2, 0);
        assert_eq!(height.nm() % 2, 0);

        let ll = Vec2::new(cx - width / 2, cy - height / 2);
        let ur = Vec2::new(cx + width / 2, cy + height / 2);

        assert_eq!(ll.x.nm() % grid.nm(), 0);
        assert_eq!(ll.y.nm() % grid.nm(), 0);
        assert_eq!(ur.x.nm() % grid.nm(), 0);
        assert_eq!(ur.y.nm() % grid.nm(), 0);
        let res = Self { ll, ur };

        assert_eq!(res.width(), width);
        assert_eq!(res.height(), height);

        res
    }

    pub fn center_x(&self, grid: Distance) -> Distance {
        (self.ll.x + self.ur.x).round_to(2 * grid) / 2
    }

    pub fn center_y(&self, grid: Distance) -> Distance {
        (self.ll.y + self.ur.y).round_to(2 * grid) / 2
    }

    pub fn ll_wh(llx: Distance, lly: Distance, width: Distance, height: Distance) -> Self {
        let ll = Vec2::new(llx, lly);
        let ur = Vec2::new(width, height) + ll;
        Self { ll, ur }
    }

    pub fn lr_wh(lrx: Distance, lry: Distance, width: Distance, height: Distance) -> Self {
        let ll = Vec2::new(lrx - width, lry);
        let ur = Vec2::new(lrx, lry + height);
        Self { ll, ur }
    }

    pub fn ul_wh(ulx: Distance, uly: Distance, width: Distance, height: Distance) -> Self {
        let ll = Vec2::new(ulx, uly - height);
        let ur = Vec2::new(ulx + width, uly);
        Self { ll, ur }
    }

    pub fn ur_wh(urx: Distance, ury: Distance, width: Distance, height: Distance) -> Self {
        let ur = Vec2::new(urx, ury);
        let ll = ur - Vec2::new(width, height);
        Self { ll, ur }
    }

    pub fn from_internal(llx: i64, lly: i64, urx: i64, ury: i64, nm_per_internal: i64) -> Self {
        assert!(urx >= llx);
        assert!(ury >= lly);
        Self {
            ll: Vec2::from_internal(llx, lly, nm_per_internal),
            ur: Vec2::from_internal(urx, ury, nm_per_internal),
        }
    }
    pub fn width(&self) -> Distance {
        self.ur.x - self.ll.x
    }
    pub fn height(&self) -> Distance {
        self.ur.y - self.ll.y
    }
    pub fn area(&self) -> Area {
        self.width() * self.height()
    }
    pub fn as_internal(&self, nm_per_internal: i64) -> (i64, i64, i64, i64) {
        let (llx, lly) = self.ll.as_internal(nm_per_internal);
        let (urx, ury) = self.ur.as_internal(nm_per_internal);
        (llx, lly, urx, ury)
    }

    pub fn grow_border(&self, dist: Distance) -> Self {
        Self::from_dist(
            self.ll.x - dist,
            self.ll.y - dist,
            self.ur.x + dist,
            self.ur.y + dist,
        )
    }

    /// Changes the width of this rectangle
    /// without changing the position of the
    /// left edge
    pub fn set_width(&mut self, w: Distance) -> &mut Self {
        self.ur = self.ll + Vec2::new(w, self.height());
        self
    }

    /// Changes the width of this rectangle
    /// without changing the position of the
    /// right edge
    pub fn set_width_from_right(&mut self, w: Distance) -> &mut Self {
        self.ll = self.ur - Vec2::new(w, self.height());
        self
    }

    pub fn grow(&mut self, dir: Direction, dist: Distance) -> &mut Self {
        match dir {
            Direction::Up => self.ur.y += dist,
            Direction::Down => self.ll.y -= dist,
            Direction::Right => self.ur.x += dist,
            Direction::Left => self.ll.x -= dist,
        }
        self
    }

    #[inline]
    pub fn left_edge(&self) -> Distance {
        self.ll.x
    }

    #[inline]
    pub fn right_edge(&self) -> Distance {
        self.ur.x
    }

    #[inline]
    pub fn top_edge(&self) -> Distance {
        self.ur.y
    }

    #[inline]
    pub fn bottom_edge(&self) -> Distance {
        self.ll.y
    }

    pub fn overlap(&self, other: Rect) -> Self {
        Self::from_dist(
            Distance::max(self.ll.x, other.ll.x),
            Distance::max(self.ll.y, other.ll.y),
            Distance::min(self.ur.x, other.ur.x),
            Distance::min(self.ur.y, other.ur.y),
        )
    }

    pub fn shrink(&mut self, dir: Direction, dist: Distance) -> &mut Self {
        match dir {
            Direction::Up => self.ur.y -= dist,
            Direction::Down => self.ll.y += dist,
            Direction::Right => self.ur.x -= dist,
            Direction::Left => self.ll.x += dist,
        }
        self
    }

    pub fn translate(&mut self, dir: Direction, dist: Distance) -> &mut Self {
        match dir {
            Direction::Up => {
                self.ll.y += dist;
                self.ur.y += dist;
            }
            Direction::Down => {
                self.ll.y -= dist;
                self.ur.y -= dist;
            }
            Direction::Right => {
                self.ll.x += dist;
                self.ur.x += dist;
            }
            Direction::Left => {
                self.ll.x -= dist;
                self.ur.x -= dist;
            }
        }
        self
    }

    pub fn try_align_center(&self, other: Rect, grid: Distance) -> Self {
        let left = (2 * other.left_edge() + other.width() - self.width()).round_to(2 * grid) / 2;
        let bot = (2 * other.bottom_edge() + other.height() - self.height()).round_to(2 * grid) / 2;
        Self::ll_wh(left, bot, self.width(), self.height())
    }

    pub fn try_align_center_x(&self, other: Rect, grid: Distance) -> Self {
        let left = (2 * other.left_edge() + other.width() - self.width()).round_to(2 * grid) / 2;
        Self::ll_wh(left, self.bottom_edge(), self.width(), self.height())
    }

    pub fn try_align_center_y(&self, other: Rect, grid: Distance) -> Self {
        let bot = (2 * other.bottom_edge() + other.height() - self.height()).round_to(2 * grid) / 2;
        Self::ll_wh(self.left_edge(), bot, self.width(), self.height())
    }

    #[inline]
    pub fn ll(&self) -> Vec2 {
        self.ll
    }

    #[inline]
    pub fn ur(&self) -> Vec2 {
        self.ur
    }

    #[inline]
    pub fn lr(&self) -> Vec2 {
        Vec2::new(self.ur.x, self.ll.y)
    }

    #[inline]
    pub fn ul(&self) -> Vec2 {
        Vec2::new(self.ll.x, self.ur.y)
    }

    pub fn btcxw(bot: Distance, top: Distance, cx: Distance, w: Distance) -> Self {
        assert!(w >= Distance::zero());
        let llx = cx - w / 2;
        let urx = cx + w / 2;
        assert_eq!(urx - llx, w);
        Self {
            ll: Vec2::new(llx, bot),
            ur: Vec2::new(urx, top),
        }
    }

    pub fn lrcyh(left: Distance, right: Distance, cy: Distance, h: Distance) -> Self {
        assert!(h >= Distance::zero());
        let lly = cy - h / 2;
        let ury = cy + h / 2;
        assert_eq!(ury - lly, h);
        Self {
            ll: Vec2::new(left, lly),
            ur: Vec2::new(right, ury),
        }
    }

    pub fn lbrh(left: Distance, bot: Distance, right: Distance, h: Distance) -> Self {
        assert!(h >= Distance::zero());
        Self {
            ll: Vec2::new(left, bot),
            ur: Vec2::new(right, bot + h),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_equality() {
        assert_eq!(Distance::from_nm(2_000), Distance::from_um(2));
        assert_eq!(Distance::from_meters(4), Distance::from_mm(4_000));
        assert_eq!(Distance::from_nm(39_000_000_000), Distance::from_meters(39));
        assert_eq!(Distance::from_mm(5), Distance::from_um(5_000));
    }

    #[test]
    fn test_distance_ops() {
        let d1 = Distance::from_um(4);
        let d2 = Distance::from_nm(200);

        let sum = Distance::from_nm(4_000 + 200);
        let diff = Distance::from_nm(4_000 - 200);

        let product = Area { nm2: 200 * 4_000 };

        assert_eq!(d1 + d2, sum);
        assert_eq!(d2 + d1, sum);
        assert_eq!(d1 - d2, diff);
        assert_eq!(d2 - d1, -diff);
        assert_eq!(d1 * d2, product);
        assert_eq!(d2 * d1, product);
    }

    #[test]
    fn test_distance_conversion() {
        let nm_per_internal = 10;
        let nm_per_lambda = 20;

        for i in -40..=40 {
            assert_eq!(
                Distance::from_internal(2 * i, nm_per_internal),
                Distance::from_lambdas(i, nm_per_lambda)
            );
        }

        let nm_per_internal = 100;
        let nm_per_lambda = 300;

        for i in -40..=40 {
            assert_eq!(
                Distance::from_internal(3 * i, nm_per_internal),
                Distance::from_lambdas(i, nm_per_lambda)
            );
        }

        let nm_per_internal = 20;
        let nm_per_lambda = 30;

        for i in (-40..=40).step_by(2) {
            assert_eq!(
                Distance::from_internal(3 * i / 2, nm_per_internal),
                Distance::from_lambdas(i, nm_per_lambda)
            );
        }
    }

    #[test]
    fn test_rect_basic() {
        let nm_per_internal = 10;
        let rect = Rect::from_internal(0, 50, 100, 200, nm_per_internal);
        assert_eq!(rect.width(), Distance::from_um(1));
        assert_eq!(rect.height(), Distance::from_nm(1_500));
        assert_eq!(rect.area(), Area::from_nm2(100 * 150 * 10 * 10));
    }

    #[test]
    #[should_panic]
    fn test_rect_invalid_bounds_1() {
        let nm_per_internal = 10;
        Rect::from_internal(200, 0, 100, 100, nm_per_internal);
    }

    #[test]
    #[should_panic]
    fn test_rect_invalid_bounds_2() {
        let nm_per_internal = 10;
        Rect::from_internal(200, 80, 300, 70, nm_per_internal);
    }

    #[test]
    #[should_panic]
    fn test_rect_invalid_bounds_3() {
        let nm_per_internal = 10;
        Rect::from_internal(200, 400, 100, 399, nm_per_internal);
    }
}
