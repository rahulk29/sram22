use std::fmt::Display;
use std::ops;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct Distance {
    nm: i64,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct Area {
    nm2: i64,
}

impl Display for Distance {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}nm", self.nm)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct Vec2 {
    pub x: Distance,
    pub y: Distance,
}

impl Vec2 {
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

    pub fn as_internal(&self, nm_per_internal: i64) -> (i64, i64) {
        (
            self.x.as_internal(nm_per_internal),
            self.y.as_internal(nm_per_internal),
        )
    }
}

impl Display for Vec2 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl Distance {
    pub fn from_nm(nm: i64) -> Self {
        Self { nm }
    }
    pub fn from_um(um: i64) -> Self {
        Self { nm: 1_000 * um }
    }
    pub fn from_mm(mm: i64) -> Self {
        Self { nm: 1_000_000 * mm }
    }
    pub fn from_meters(meters: i64) -> Self {
        Self {
            nm: 1_000_000_000 * meters,
        }
    }
    pub fn from_internal(internal: i64, nm_per_internal: i64) -> Self {
        Self {
            nm: nm_per_internal * internal,
        }
    }
    pub fn from_lambdas(lambda: i64, nm_per_lambda: i64) -> Self {
        Self {
            nm: nm_per_lambda * lambda,
        }
    }
    pub fn as_lambdas(&self, nm_per_lambda: i64) -> i64 {
        self.nm / nm_per_lambda
    }

    pub fn as_internal(&self, nm_per_internal: i64) -> i64 {
        self.nm / nm_per_internal
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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Rect {
    pub ll: Vec2,
    pub ur: Vec2,
}

impl Rect {
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
}
