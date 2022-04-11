use vlsir::circuit::{connection::Stype, Connection, Signal, Slice};

use self::conns::{conn_width, get_conn};

pub fn signal(name: impl Into<String>) -> Signal {
    Signal {
        name: name.into(),
        width: 1,
    }
}

pub fn sig_conn(sig: &Signal) -> Connection {
    Connection {
        stype: Some(Stype::Sig(sig.to_owned())),
    }
}

pub struct BusConnection(Connection);

impl BusConnection {
    pub fn width(&self) -> usize {
        let width = conn_width(&self.0);
        assert!(width >= 0);
        width as usize
    }

    pub fn get(&self, idx: usize) -> Option<Slice> {
        get_conn(&self.0, idx)
    }
}

impl From<Connection> for BusConnection {
    fn from(c: Connection) -> Self {
        Self(c)
    }
}

impl From<BusConnection> for Connection {
    fn from(c: BusConnection) -> Self {
        c.0
    }
}

pub struct BusConnectionIter<'a> {
    conn: &'a Connection,
}

pub mod conns {
    use vlsir::circuit::{connection::Stype, Concat, Connection, Signal, Slice};

    pub fn get_sig(s: &Signal, idx: usize) -> Option<Slice> {
        let idx = idx as i64;
        if idx < s.width {
            Some(Slice {
                signal: s.name.to_string(),
                top: idx,
                bot: idx,
            })
        } else {
            None
        }
    }

    pub fn get_slice(s: &Slice, idx: usize) -> Option<Slice> {
        let idx = idx as i64;
        if s.top - idx < s.bot {
            None
        } else {
            Some(Slice {
                signal: s.signal.to_string(),
                top: s.top - idx,
                bot: s.top - idx,
            })
        }
    }

    pub fn get_concat(c: &Concat, mut idx: usize) -> Option<Slice> {
        for part in &c.parts {
            let width = conn_width(&part);
            assert!(width >= 0);
            let width = width as usize;
            if idx < width {
                return match part.stype.as_ref().unwrap() {
                    Stype::Sig(s) => get_sig(&s, idx),
                    Stype::Slice(s) => get_slice(&s, idx),
                    Stype::Concat(c) => get_concat(&c, idx),
                };
            }
            idx -= width;
        }
        None
    }

    pub fn get_conn(conn: &Connection, idx: usize) -> Option<Slice> {
        match conn.stype.as_ref().unwrap() {
            Stype::Sig(s) => get_sig(&s, idx),
            Stype::Slice(s) => get_slice(&s, idx),
            Stype::Concat(c) => get_concat(&c, idx),
        }
    }

    pub fn conn_width(conn: &Connection) -> i64 {
        match conn.stype.as_ref().unwrap() {
            Stype::Sig(s) => s.width,
            Stype::Slice(s) => s.top - s.bot + 1,
            Stype::Concat(c) => c.parts.iter().map(conn_width).sum(),
        }
    }
}

/// Calculates log2(x). Not at all efficient or optimized.
///
/// Behavior when x is not a power of 2 is undefined.
///
/// x must be strictly positive.
///
/// # Examples
///
/// ```rust
/// use sramgen::utils::log2;
/// assert_eq!(log2(1), 0);
/// assert_eq!(log2(2), 1);
/// assert_eq!(log2(4), 2);
/// assert_eq!(log2(8), 3);
/// assert_eq!(log2(16), 4);
/// assert_eq!(log2(32), 5);
/// assert_eq!(log2(64), 6);
/// assert_eq!(log2(128), 7);
/// assert_eq!(log2(256), 8);
/// assert_eq!(log2(512), 9);
/// assert_eq!(log2(1024), 10);
/// ```
pub fn log2(mut x: usize) -> usize {
    assert!(x >= 1);

    let mut ctr = 0;
    while x > 1 {
        x = x >> 1;
        ctr += 1;
    }
    ctr
}

#[cfg(test)]
mod tests {
    use crate::utils::log2;

    #[test]
    fn test_log2() {
        assert_eq!(log2(1), 0);
        assert_eq!(log2(2), 1);
        assert_eq!(log2(4), 2);
        assert_eq!(log2(8), 3);
        assert_eq!(log2(16), 4);
        assert_eq!(log2(32), 5);
        assert_eq!(log2(64), 6);
        assert_eq!(log2(128), 7);
        assert_eq!(log2(256), 8);
        assert_eq!(log2(512), 9);
        assert_eq!(log2(1024), 10);
    }

    #[test]
    #[should_panic]
    fn test_log2_of_zero() {
        let _ = log2(0);
    }
}
