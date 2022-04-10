use vlsir::circuit::{connection::Stype, Connection, Signal};

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
