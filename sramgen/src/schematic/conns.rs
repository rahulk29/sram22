use std::collections::HashMap;
use vlsir::circuit::connection::Stype;
use vlsir::circuit::{port, Concat, Connection, Port, Signal, Slice};

pub fn signal(name: impl Into<String>) -> Signal {
    Signal {
        name: name.into(),
        width: 1,
    }
}

pub fn bus(name: impl Into<String>, width: i64) -> Signal {
    Signal {
        name: name.into(),
        width,
    }
}

pub fn sig_conn(sig: &Signal) -> Connection {
    Connection {
        stype: Some(Stype::Sig(sig.to_owned())),
    }
}

#[derive(Debug)]
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

pub fn port_inout(s: &Signal) -> Port {
    Port {
        signal: Some(s.to_owned()),
        direction: port::Direction::Inout as i32,
    }
}
pub fn port_input(s: &Signal) -> Port {
    Port {
        signal: Some(s.to_owned()),
        direction: port::Direction::Input as i32,
    }
}
pub fn port_output(s: &Signal) -> Port {
    Port {
        signal: Some(s.to_owned()),
        direction: port::Direction::Output as i32,
    }
}

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
        let width = conn_width(part);
        assert!(width >= 0);
        let width = width as usize;
        if idx < width {
            return match part.stype.as_ref().unwrap() {
                Stype::Sig(s) => get_sig(s, idx),
                Stype::Slice(s) => get_slice(s, idx),
                Stype::Concat(c) => get_concat(c, idx),
            };
        }
        idx -= width;
    }
    None
}

pub fn get_conn(conn: &Connection, idx: usize) -> Option<Slice> {
    match conn.stype.as_ref().unwrap() {
        Stype::Sig(s) => get_sig(s, idx),
        Stype::Slice(s) => get_slice(s, idx),
        Stype::Concat(c) => get_concat(c, idx),
    }
}

pub fn conn_width(conn: &Connection) -> i64 {
    match conn.stype.as_ref().unwrap() {
        Stype::Sig(s) => s.width,
        Stype::Slice(s) => s.top - s.bot + 1,
        Stype::Concat(c) => c.parts.iter().map(conn_width).sum(),
    }
}

pub fn conn_slice(signal: impl Into<String>, top: i64, bot: i64) -> Connection {
    Connection {
        stype: Some(Stype::Slice(Slice {
            signal: signal.into(),
            top,
            bot,
        })),
    }
}

pub fn conn_map(map: HashMap<&str, Connection>) -> HashMap<String, Connection> {
    map.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}
