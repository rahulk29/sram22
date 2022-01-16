use std::{
    io::{Read, Write},
    net::TcpStream,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio}, time::Duration,
};

/// A builder used to construct a [`MagicInstance`]
///
/// # Example
///
/// ```
/// use magic_vlsi::MagicInstanceBuilder;
/// let mut builder = MagicInstanceBuilder::new().cwd("/path/to/cwd").tech("scmos");
/// ```
pub struct MagicInstanceBuilder {
    cwd: Option<PathBuf>,
    tech: Option<String>,
    magic: Option<PathBuf>,
    port: u16,
}

impl MagicInstanceBuilder {
    /// Creates a new [`MagicInstanceBuilder`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the current working directory in which to start MAGIC.
    pub fn cwd(mut self, cwd: impl AsRef<Path>) -> Self {
        self.cwd = Some(cwd.as_ref().to_owned());
        self
    }

    /// Set the name of the technology for MAGIC to use.
    pub fn tech(mut self, tech: &str) -> Self {
        self.tech = Some(tech.to_owned());
        self
    }

    /// Set a path to the MAGIC binary.
    ///
    /// If not specified, the binary will be found by
    /// searching your operating system's path.
    pub fn magic(mut self, magic: impl AsRef<Path>) -> Self {
        self.magic = Some(magic.as_ref().to_owned());
        self
    }

    /// Set the port to use when communicating with MAGIC.
    ///
    /// Make sure this port is not already in use, either
    /// by another MAGIC instance, or by some other process.
    ///
    /// The default port is 9999.
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Consumes the builder, returning a [`MagicInstance`].
    ///
    /// This will start a MAGIC process in the background.
    /// The child process will listen on the port configured
    /// by the builder.
    pub fn build(self) -> MagicInstance {
        MagicInstance::new(self)
    }
}

impl Default for MagicInstanceBuilder {
    fn default() -> Self {
        Self {
            cwd: None,
            tech: None,
            magic: None,
            port: 9999,
        }
    }
}

/// A handle to a running MAGIC instance.
///
/// Can be created using [`MagicInstanceBuilder`].
///
/// Documentation for most MAGIC functions has
/// been taken from the [MAGIC documentation](http://opencircuitdesign.com/magic/userguide.html).
pub struct MagicInstance {
    child: Child,
    stream: TcpStream,
}

const MAGIC_SOCKET_SCRIPT: &[u8] = include_bytes!("serversock.tcl");

impl MagicInstance {
    fn new(builder: MagicInstanceBuilder) -> Self {
        let mut cmd = match builder.magic {
            Some(magic) => Command::new(magic),
            None => Command::new("magic"),
        };

        cmd.arg("-dnull").arg("-noconsole");

        if let Some(tech) = builder.tech {
            cmd.arg("-T").arg(tech);
        }

        if let Some(cwd) = builder.cwd {
            cmd.current_dir(cwd);
        }

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        let mut child = cmd.spawn().unwrap();
        let mut stdin = child.stdin.take().unwrap();

        writeln!(&mut stdin, "set svcPort {}", builder.port).unwrap();

        stdin.write_all(MAGIC_SOCKET_SCRIPT).unwrap();

        let addr = format!("127.0.0.1:{}", builder.port);

        let mut backoff_ms = 1;
        let mut num_attempts = 0;
        let stream = loop {
            if let Ok(s) = TcpStream::connect(&addr) {
                break s;
            } else {
                std::thread::sleep(Duration::from_millis(backoff_ms));
                backoff_ms *= 2;
                num_attempts += 1;
                if num_attempts > 10 {
                    panic!("failed to connect to MAGIC");
                }
            }
        };

        Self { child, stream }
    }

    /// The getcell command creates subcell instances within
    /// the current edit cell. By default, with only the cellname
    /// given, an orientation of zero is assumed, and the cell
    /// is placed such that the lower-left corner of the cell's
    /// bounding box is placed at the lower-left corner of the
    /// cursor box in the parent cell.
    pub fn getcell(&mut self, cell: &str) {
        writeln!(&mut self.stream, "getcell {}", cell).unwrap();
        let _ = read_line(&mut self.stream);
    }

    /// The sideways command flips the selection from left to
    /// right. Flipping is done such that the lower left-hand
    /// corner of the selection remains in the same place
    /// through the flip.
    pub fn sideways(&mut self) {
        writeln!(&mut self.stream, "sideways").unwrap();
        let _ = read_line(&mut self.stream);
    }

    /// Return the bounding box of the selection.
    pub fn select_bbox(&mut self) {
        writeln!(&mut self.stream, "select bbox").unwrap();
        let _res = read_line(&mut self.stream);
    }

    pub fn set_box_values(&mut self, llx: i64, lly: i64, urx: i64, ury: i64) {
        writeln!(
            &mut self.stream,
            "box values {} {} {} {}",
            llx, lly, urx, ury
        )
        .unwrap();
        let _res = read_line(&mut self.stream);
        println!("done setboxval");
    }

    pub fn box_values(&mut self) -> RectCorners {
        println!("writing");
        writeln!(&mut self.stream, "box values").unwrap();
        println!("done; reading");
        let res = read_line(&mut self.stream);
        println!("done all");
        let values = res
            .split_whitespace()
            .map(|s| s.parse::<i64>().unwrap())
            .take(4)
            .collect::<Vec<i64>>();

        assert_eq!(values.len(), 4);

        RectCorners {
            llx: values[0],
            lly: values[1],
            urx: values[2],
            ury: values[3],
        }
    }
}

pub struct RectCorners {
    pub llx: i64,
    pub lly: i64,
    pub urx: i64,
    pub ury: i64,
}

pub enum Direction {
    Up,
    Down,
    Right,
    Left,
}

pub enum GeoDirection {
    North,
    South,
    East,
    West,
}

impl From<GeoDirection> for Direction {
    fn from(d: GeoDirection) -> Self {
        match d {
            GeoDirection::North => Self::Up,
            GeoDirection::South => Self::Down,
            GeoDirection::East => Self::Right,
            GeoDirection::West => Self::Left,
        }
    }
}

impl From<Direction> for GeoDirection {
    fn from(d: Direction) -> Self {
        match d {
            Direction::Up => Self::North,
            Direction::Down => Self::South,
            Direction::Right => Self::East,
            Direction::Left => Self::West,
        }
    }
}

fn read_line(conn: &mut TcpStream) -> String {
    let mut s = String::new();
    let mut bytes = [0; 512];

    loop {
        let sz = conn.read(&mut bytes).unwrap();
        let new_str = std::str::from_utf8(&bytes[..sz]).unwrap();
        if let Some(i) = new_str.find('\n') {
            s.push_str(&new_str[..i]);
            break;
        }
        s.push_str(new_str);
    }

    s
}

impl Drop for MagicInstance {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU16, Ordering};

    use crate::MagicInstanceBuilder;
    use lazy_static::lazy_static;

    pub fn get_port() -> u16 {
        lazy_static! {
            static ref PORT_COUNTER: AtomicU16 = AtomicU16::new(1024);
        }
        PORT_COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    #[test]
    fn test_builder_api() {
        let _builder = MagicInstanceBuilder::new()
            .cwd("/fake/path/dir")
            .tech("sky130A");
    }

    #[test]
    fn test_start_magic() {
        let _instance = MagicInstanceBuilder::new()
            .tech("sky130A")
            .port(get_port())
            .build();
    }

    #[test]
    fn test_select_bbox() {
        let mut instance = MagicInstanceBuilder::new()
            .port(get_port())
            .tech("sky130A")
            .cwd("src/")
            .build();
        instance.getcell("sram");
        instance.select_bbox();
    }

    #[test]
    fn test_set_get_box_values() {
        let mut instance = MagicInstanceBuilder::new()
            .port(get_port())
            .tech("sky130A")
            .build();

        let test_cases = [
            [0, 0, 0, 0],
            [-12, 4, 45, 67],
            [44, -12, 72, 2],
            [-83, -93, 12, -42],
        ];
        for test_case in test_cases {
            instance.set_box_values(test_case[0], test_case[1], test_case[2], test_case[3]);
            let rect = instance.box_values();

            assert_eq!(rect.llx, test_case[0]);
            assert_eq!(rect.lly, test_case[1]);
            assert_eq!(rect.urx, test_case[2]);
            assert_eq!(rect.ury, test_case[3]);
        }
    }
}
