use std::collections::HashMap;

use vlsir::circuit::connection::Stype;
use vlsir::circuit::{port, Connection, ExternalModule, Port, Signal, Slice};
use vlsir::reference::To;
use vlsir::{Module, QualifiedName, Reference};

use crate::save_bin;
use crate::tech::all_external_modules;

use self::conns::{conn_width, get_conn};

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

pub mod conns {
    use vlsir::circuit::connection::Stype;
    use vlsir::circuit::{Concat, Connection, Signal, Slice};

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
}

pub fn simple_ext_module(
    domain: impl Into<String>,
    name: impl Into<String>,
    ports: &[&str],
) -> ExternalModule {
    let ports = ports
        .iter()
        .map(|&n| Port {
            signal: Some(signal(n)),
            direction: port::Direction::Inout as i32,
        })
        .collect::<Vec<_>>();

    ExternalModule {
        name: Some(QualifiedName {
            domain: domain.into(),
            name: name.into(),
        }),
        desc: "An external module".to_string(),
        ports,
        parameters: vec![],
    }
}

pub fn conn_map(map: HashMap<&str, Connection>) -> HashMap<String, Connection> {
    map.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

pub fn local_reference(name: impl Into<String>) -> Option<Reference> {
    Some(Reference {
        to: Some(To::Local(name.into())),
    })
}

pub fn save_modules(name: &str, modules: Vec<Module>) -> Result<(), Box<dyn std::error::Error>> {
    let ext_modules = all_external_modules();
    let pkg = vlsir::circuit::Package {
        domain: format!("sramgen_{}", name),
        desc: "Sramgen generated cells".to_string(),
        modules,
        ext_modules,
    };

    save_bin(name, pkg)?;

    Ok(())
}

#[cfg(test)]
use pdkprims::PdkLib;
#[cfg(test)]
use std::path::PathBuf;

pub const TEST_BUILD_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/build");

#[cfg(test)]
pub(crate) fn test_gds_path(lib: &PdkLib) -> PathBuf {
    let mut path = PathBuf::from(TEST_BUILD_PATH);
    path.push(format!("gds/{}.gds", &lib.lib.name));
    path
}

#[cfg(test)]
pub(crate) fn test_lef_path(lib: &PdkLib) -> PathBuf {
    let mut path = PathBuf::from(TEST_BUILD_PATH);
    path.push(format!("lef/{}.lef", &lib.lib.name));
    path
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
        x >>= 1;
        ctr += 1;
    }
    ctr
}

#[cfg(test)]
use std::fmt::Debug;

#[cfg(test)]
pub(crate) fn panic_on_err<E: Debug>(e: E) -> ! {
    println!("ERROR: {e:?}");
    panic!("ERROR: {e:?}");
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
