use cell::{InstanceCell, LayoutCell, LayoutCellRef, LayoutPort};
use error::{MagicError, Result, StartMagicError};
use std::{
    fmt::Display,
    io::{Read, Write},
    net::TcpStream,
    os::unix::prelude::{AsRawFd, FromRawFd},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use units::{Distance, Rect, Vec2};

pub mod cell;
pub mod error;
pub mod units;

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
    pub fn build(self) -> std::result::Result<MagicInstance, StartMagicError> {
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
    nm_per_lambda: i64,
    nm_per_internal: i64,
}

const MAGIC_SOCKET_SCRIPT: &[u8] = include_bytes!("serversock.tcl");

impl MagicInstance {
    fn new(builder: MagicInstanceBuilder) -> std::result::Result<Self, StartMagicError> {
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


        #[cfg(debug_assertions)]
        {
            let f = std::fs::File::create("magic.log")?;
            let fd_out = f.as_raw_fd();

            cmd.stdin(Stdio::piped())
                .stdout(unsafe { Stdio::from_raw_fd(fd_out) })
                .stderr(unsafe { Stdio::from_raw_fd(fd_out) });
        }

        #[cfg(not(debug_assertions))]
        {
            cmd.stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| StartMagicError::Spawn(Box::new(e)))?;
        let mut stdin = child.stdin.take().ok_or_else(|| {
            StartMagicError::Connect(String::from("failed to obtain handle to magic stdin"))
        })?;

        writeln!(&mut stdin, "set svcPort {}", builder.port)?;

        stdin.write_all(MAGIC_SOCKET_SCRIPT)?;

        let addr = format!("127.0.0.1:{}", builder.port);

        let mut backoff_ms = 1;
        let mut num_attempts = 0;
        let stream = loop {
            if let Ok(s) = TcpStream::connect(&addr) {
                break Ok(s);
            } else {
                if num_attempts > 10 {
                    break Err(StartMagicError::Connect(String::from(
                        "timed out attempting to connect to magic process",
                    )));
                }
                std::thread::sleep(Duration::from_millis(backoff_ms));
                backoff_ms *= 2;
                num_attempts += 1;
            }
        }?;

        let mut res = Self {
            child,
            stream,
            nm_per_lambda: 0,
            nm_per_internal: 0,
        };
        res.update_units()?;

        Ok(res)
    }

    /// The getcell command creates subcell instances within
    /// the current edit cell. By default, with only the cellname
    /// given, an orientation of zero is assumed, and the cell
    /// is placed such that the lower-left corner of the cell's
    /// bounding box is placed at the lower-left corner of the
    /// cursor box in the parent cell.
    pub fn getcell(&mut self, cell: &str) -> Result<Rect> {
        writeln!(&mut self.stream, "getcell {}", cell)?;
        read_line(&mut self.stream)?;
        // Loading a cell can scale the grid, so recalculate units
        self.update_units()?;
        self.select_bbox()
    }

    /// The getcell command creates subcell instances within
    /// the current edit cell. By default, with only the cellname
    /// given, an orientation of zero is assumed, and the cell
    /// is placed such that the lower-left corner of the cell's
    /// bounding box is placed at the lower-left corner of the
    /// cursor box in the parent cell.
    pub fn getcell_name(&mut self, cell: &str) -> Result<String> {
        writeln!(&mut self.stream, "getcell {}", cell)?;
        let cell_name = read_line(&mut self.stream)?.trim().to_string();
        // Loading a cell can scale the grid, so recalculate units
        self.update_units()?;
        Ok(cell_name)
    }

    pub fn place_cell(&mut self, cell: &str, ll: Vec2) -> Result<Rect> {
        self.set_box_values(Rect::ll_wh(ll.x, ll.y, Distance::zero(), Distance::zero()))?;
        self.getcell(cell)
    }

    pub fn place_layout_cell(&mut self, cell: LayoutCellRef, ll: Vec2) -> Result<InstanceCell> {
        self.set_box_values(Rect::ll_wh(ll.x, ll.y, Distance::zero(), Distance::zero()))?;
        let name = self.getcell_name(&cell.name)?;

        Ok(InstanceCell::new(ll, cell, name))
    }

    pub fn flip_cell_x(&mut self, cell: &mut InstanceCell) -> Result<()> {
        self.select_cell(&cell.name)?;
        self.sideways()?;
        cell.sideways();
        Ok(())
    }

    pub fn flip_cell_y(&mut self, cell: &mut InstanceCell) -> Result<()> {
        self.select_cell(&cell.name)?;
        self.upside_down()?;
        cell.upside_down();
        Ok(())
    }

    pub fn rename_cell_pin(&mut self, cell: &InstanceCell, pin: &str, name: &str) -> Result<()> {
        let port = cell.port(pin);
        let bbox = cell.port_bbox(pin);
        self.paint_box(bbox, &port.layer)?;
        self.select_visible()?;
        self.exec_one(&format!("select intersect {}", &port.layer))?;
        self.label_position_layer(name, Direction::Up, &port.layer)?;
        Ok(())
    }

    /// The sideways command flips the selection from left to
    /// right. Flipping is done such that the lower left-hand
    /// corner of the selection remains in the same place
    /// through the flip.
    pub fn sideways(&mut self) -> Result<()> {
        writeln!(&mut self.stream, "sideways")?;
        read_line(&mut self.stream)?;
        Ok(())
    }

    /// Ensures that the cursor box is present.
    ///
    /// Equivalent to running `box 0 0 0 0` in Magic.
    pub fn enable_box(&mut self) -> Result<()> {
        writeln!(&mut self.stream, "box 0 0 0 0")?;
        read_line(&mut self.stream)?;
        Ok(())
    }

    pub fn load(&mut self, cell: &str) -> Result<()> {
        writeln!(&mut self.stream, "load {}", cell)?;
        read_line(&mut self.stream)?;
        Ok(())
    }

    pub fn edit(&mut self, cell: &str) -> Result<()> {
        writeln!(&mut self.stream, "edit {}", cell)?;
        read_line(&mut self.stream)?;
        Ok(())
    }

    pub fn load_layout_cell(&mut self, cell: &str) -> Result<LayoutCellRef> {
        self.load(cell)?;
        self.select_top_cell()?;
        let bbox = self.select_bbox()?;

        let mut idx = self.port_first()?;

        let mut ports = Vec::new();

        while idx != -1 {
            let name = self.port_index_name(idx)?;
            self.findlabel(&name)?;
            self.select_visible()?;
            let bbox = self.box_values()?;
            let layer = self.label_layer()?;

            ports.push(LayoutPort { name, bbox, layer });
            idx = self.port_next(idx)?;
        }

        Ok(Arc::new(LayoutCell {
            name: cell.to_string(),
            bbox,
            ports,
        }))
    }

    pub fn array(&mut self, xsize: u32, ysize: u32) -> Result<()> {
        writeln!(&mut self.stream, "array {} {}", xsize, ysize)?;
        read_line(&mut self.stream)?;
        Ok(())
    }

    /// Return the bounding box of the selection.
    pub fn select_bbox(&mut self) -> Result<Rect> {
        writeln!(&mut self.stream, "select bbox").unwrap();
        let res = read_line(&mut self.stream)?;
        let values: Vec<i64> = res
            .split_whitespace()
            .map(|s| {
                s.parse::<i64>()
                    .map_err(|_| MagicError::UnexpectedOutput("failed to parse i64".to_string()))
            })
            .take(4)
            .collect::<Result<Vec<_>>>()?;

        assert_eq!(values.len(), 4);

        Ok(Rect::from_internal(
            values[0],
            values[1],
            values[2],
            values[3],
            self.nm_per_internal,
        ))
    }

    pub fn copy_dir(&mut self, dir: Direction, distance: Distance) -> Result<()> {
        writeln!(
            &mut self.stream,
            "copy {} {}i",
            dir,
            distance.as_internal(self.nm_per_internal)
        )?;
        read_line(&mut self.stream)?;
        Ok(())
    }

    pub fn set_box_values(&mut self, rect: Rect) -> Result<()> {
        let (llx, lly, urx, ury) = rect.as_internal(self.nm_per_internal);
        writeln!(
            &mut self.stream,
            "box values {} {} {} {}",
            llx, lly, urx, ury
        )?;
        read_line(&mut self.stream)?;
        Ok(())
    }

    pub fn box_values(&mut self) -> Result<Rect> {
        writeln!(&mut self.stream, "box values")?;
        let res = read_line(&mut self.stream)?;
        let values = res
            .split_whitespace()
            .map(|s| {
                s.parse::<i64>()
                    .map_err(|_| MagicError::UnexpectedOutput("failed to parse i64".to_string()))
            })
            .take(4)
            .collect::<Result<Vec<i64>>>()?;

        assert_eq!(values.len(), 4);

        Ok(Rect::from_internal(
            values[0],
            values[1],
            values[2],
            values[3],
            self.nm_per_internal,
        ))
    }

    pub fn set_snap(&mut self, snap_mode: SnapMode) -> Result<()> {
        writeln!(&mut self.stream, "snap {}", snap_mode)?;
        read_line(&mut self.stream)?;
        self.update_units()?;
        Ok(())
    }

    pub fn scalegrid(&mut self, a: i64, b: i64) -> Result<()> {
        writeln!(&mut self.stream, "scalegrid {} {}", a, b)?;
        read_line(&mut self.stream)?;
        self.update_units()?;
        Ok(())
    }

    pub fn snap(&mut self) -> Result<SnapMode> {
        writeln!(&mut self.stream, "snap")?;
        let res = read_line(&mut self.stream)?;
        res.parse::<SnapMode>()
    }

    pub fn paint(&mut self, layer: &str) -> Result<()> {
        writeln!(&mut self.stream, "paint {}", layer)?;
        read_line(&mut self.stream)?;
        Ok(())
    }

    pub fn paint_box(&mut self, rect: Rect, layer: &str) -> Result<()> {
        self.set_box_values(rect)?;
        self.paint(layer)?;
        Ok(())
    }

    pub fn select_clear(&mut self) -> Result<()> {
        writeln!(&mut self.stream, "select clear")?;
        read_line(&mut self.stream)?;
        Ok(())
    }

    pub fn select_top_cell(&mut self) -> Result<()> {
        writeln!(&mut self.stream, "select top cell")?;
        read_line(&mut self.stream)?;
        Ok(())
    }

    pub fn select_visible(&mut self) -> Result<()> {
        writeln!(&mut self.stream, "select visible")?;
        read_line(&mut self.stream)?;
        Ok(())
    }

    pub fn select_cell(&mut self, cell_name: &str) -> Result<()> {
        writeln!(&mut self.stream, "select cell {}", cell_name)?;
        read_line(&mut self.stream)?;
        Ok(())
    }

    pub fn delete(&mut self) -> Result<()> {
        self.exec_one("delete")?;
        Ok(())
    }

    pub fn upside_down(&mut self) -> Result<()> {
        writeln!(&mut self.stream, "upsidedown")?;
        read_line(&mut self.stream)?;
        Ok(())
    }

    pub fn drc_off(&mut self) -> Result<()> {
        writeln!(&mut self.stream, "drc off")?;
        read_line(&mut self.stream)?;
        Ok(())
    }

    pub fn drc_on(&mut self) -> Result<()> {
        writeln!(&mut self.stream, "drc on")?;
        read_line(&mut self.stream)?;
        Ok(())
    }

    pub fn identify(&mut self, id: &str) -> Result<()> {
        writeln!(&mut self.stream, "identify {}", id)?;
        read_line(&mut self.stream)?;
        Ok(())
    }

    pub fn save(&mut self, cell_name: &str) -> Result<()> {
        writeln!(&mut self.stream, "save {}", cell_name)?;
        read_line(&mut self.stream)?;
        Ok(())
    }

    pub fn draw_contacts_y(
        &mut self,
        contact_type: &str,
        region: Rect,
        size: Distance,
        space: Distance,
    ) -> Result<u64> {
        let d1 = region.height() + space;
        let d2 = size + space;
        let num_contacts: i64 = d1 / d2;

        assert!(num_contacts > 0);
        let num_contacts = num_contacts as u64;

        let mut curr_y = region.ll.y;
        for _ in 0..num_contacts {
            let contact_box = Rect::from_dist(region.ll.x, curr_y, region.ur.x, curr_y + size);
            self.contact(contact_box, contact_type)?;
            curr_y = curr_y + size + space;
        }

        Ok(num_contacts)
    }

    pub fn contact(&mut self, rect: Rect, contact_type: &str) -> Result<()> {
        self.set_box_values(rect)?;
        writeln!(&mut self.stream, "paint {}", contact_type)?;
        read_line(&mut self.stream)?;
        Ok(())
    }

    pub fn exec_one(&mut self, cmd: &str) -> Result<String> {
        writeln!(&mut self.stream, "{}", cmd)?;
        read_line(&mut self.stream)
    }

    /// (a, b) indicates a lambdas = b internal units
    pub fn tech_lambda(&mut self) -> Result<(i64, i64)> {
        writeln!(&mut self.stream, "tech lambda")?;
        let res = read_line(&mut self.stream)?;
        let values: Vec<i64> = res
            .split_whitespace()
            .map(|s| {
                s.parse::<i64>()
                    .map_err(|_| MagicError::UnexpectedOutput("failed to parse i64".to_string()))
            })
            .take(2)
            .collect::<Result<Vec<_>>>()?;
        assert_eq!(values.len(), 2);
        Ok((values[0], values[1]))
    }

    fn update_units(&mut self) -> Result<()> {
        self.exec_one("box 0um 0um 1um 1um")?;
        let res = self.exec_one("box width")?;
        let internal_width = res.trim().parse::<i64>().map_err(parse_int_error)?;
        let nm_per_internal = 1_000 / internal_width;
        assert_eq!(nm_per_internal * internal_width, 1_000);
        self.nm_per_internal = nm_per_internal;
        assert_ne!(nm_per_internal, 0);
        let (a, b) = self.tech_lambda()?;
        self.nm_per_lambda = (b * self.nm_per_internal) / a;
        assert_eq!(self.nm_per_lambda * a, b * self.nm_per_internal);
        self.exec_one("undo")?;
        Ok(())
    }

    pub fn get_nm_per_internal(&mut self) -> Result<i64> {
        if self.nm_per_internal == 0 {
            self.update_units()?;
        }
        Ok(self.nm_per_internal)
    }

    pub fn get_nm_per_lambda(&mut self) -> Result<i64> {
        if self.nm_per_lambda == 0 {
            self.update_units()?;
        }
        Ok(self.nm_per_lambda)
    }

    pub fn label(&mut self, label: &str) -> Result<()> {
        self.exec_one(&format!("label {}", label)).map(|_| ())
    }

    pub fn label_position(&mut self, label: &str, position: Direction) -> Result<()> {
        self.exec_one(&format!("label {} {}", label, position))
            .map(|_| ())
    }

    pub fn setlabel_text(&mut self, text: &str) -> Result<()> {
        self.exec_one(&format!("setlabel text {}", text))
            .map(|_| ())
    }

    pub fn label_text(&mut self) -> Result<String> {
        Ok(self.exec_one("text")?.trim().into())
    }

    pub fn label_position_layer(
        &mut self,
        label: &str,
        position: Direction,
        layer: &str,
    ) -> Result<()> {
        self.select_visible()?;
        self.exec_one(&format!("select intersect {}", &layer))?;
        self.exec_one(&format!("label {} {} {}", label, position, layer))?;
        Ok(())
    }

    pub fn port_make(&mut self, idx: usize) -> Result<()> {
        self.exec_one(&format!("port make {}", idx)).map(|_| ())
    }

    pub fn port_make_default(&mut self) -> Result<()> {
        self.exec_one("port make").map(|_| ())
    }

    pub fn port_renumber(&mut self) -> Result<()> {
        self.exec_one("port renumber").map(|_| ())
    }

    pub fn port_name(&mut self) -> Result<String> {
        self.exec_one("port name")
    }

    pub fn port_index_name(&mut self, idx: i64) -> Result<String> {
        Ok(self
            .exec_one(&format!("port {} name", idx))?
            .trim()
            .to_string())
    }

    pub fn port_first(&mut self) -> Result<i64> {
        let res = self.exec_one("port first")?;
        let first = res.trim().parse::<i64>().map_err(parse_int_error)?;
        Ok(first)
    }

    pub fn port_last(&mut self) -> Result<i64> {
        let res = self.exec_one("port last")?;
        let first = res.trim().parse::<i64>().map_err(parse_int_error)?;
        Ok(first)
    }

    pub fn port_next(&mut self, n: i64) -> Result<i64> {
        let res = self.exec_one(&format!("port {} next", n))?;
        let first = res.trim().parse::<i64>().map_err(parse_int_error)?;
        Ok(first)
    }

    pub fn port_next_after_selection(&mut self) -> Result<i64> {
        let res = self.exec_one("port next")?;
        let first = res.trim().parse::<i64>().map_err(parse_int_error)?;
        Ok(first)
    }

    pub fn label_layer(&mut self) -> Result<String> {
        self.exec_one("setlabel layer").map(|s| s.trim().into())
    }

    pub fn findlabel(&mut self, label: &str) -> Result<()> {
        self.exec_one(&format!("findlabel {}", label)).map(|_| ())
    }

    pub fn findlabel_n(&mut self, label: &str, n: usize) -> Result<()> {
        self.exec_one(&format!("findlabel {} {}", label, n))
            .map(|_| ())
    }

    pub fn findlabel_glob(&mut self, label: &str) -> Result<()> {
        self.exec_one(&format!("findlabel -glob {}", label))
            .map(|_| ())
    }

    pub fn findlabel_glob_n(&mut self, label: &str, n: usize) -> Result<()> {
        self.exec_one(&format!("findlabel -glob {} {}", label, n))
            .map(|_| ())
    }

    pub fn flatten(&mut self, cell: &str) -> Result<()> {
        self.exec_one(&format!("flatten {}", cell)).map(|_| ())
    }

    pub fn flatten_opts(&mut self, opts: &str, cell: &str) -> Result<()> {
        self.exec_one(&format!("flatten {} {}", opts, cell))
            .map(|_| ())
    }
}

fn parse_int_error(e: std::num::ParseIntError) -> MagicError {
    MagicError::UnexpectedOutput(format!("failed to parse integer: {}", e))
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SnapMode {
    Internal,
    Lambda,
    User,
}

impl FromStr for SnapMode {
    type Err = MagicError;
    fn from_str(s: &str) -> Result<Self> {
        match s.trim() {
            "internal" => Ok(Self::Internal),
            "lambda" => Ok(Self::Lambda),
            "user" => Ok(Self::User),
            _ => Err(MagicError::UnexpectedOutput(format!(
                "unknown snap mode: {}",
                s
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Direction {
    Up,
    Down,
    Right,
    Left,
}

#[derive(Debug, Clone, Copy)]
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

impl From<Direction> for &str {
    fn from(d: Direction) -> Self {
        match d {
            Direction::Up => "north",
            Direction::Down => "south",
            Direction::Right => "east",
            Direction::Left => "west",
        }
    }
}

impl Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Direction::Up => write!(f, "north"),
            Direction::Down => write!(f, "south"),
            Direction::Right => write!(f, "east"),
            Direction::Left => write!(f, "west"),
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

impl Display for SnapMode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            SnapMode::Internal => write!(f, "internal"),
            SnapMode::Lambda => write!(f, "lambda"),
            SnapMode::User => write!(f, "user"),
        }
    }
}

fn read_line(conn: &mut TcpStream) -> Result<String> {
    let mut s = String::new();
    let mut bytes = [0; 512];

    loop {
        let sz = conn.read(&mut bytes)?;
        let new_str = std::str::from_utf8(&bytes[..sz])?;
        if let Some(i) = new_str.find('\n') {
            s.push_str(&new_str[..i]);
            break;
        }
        s.push_str(new_str);
    }

    Ok(s)
}

impl Drop for MagicInstance {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU16, Ordering};

    use crate::{units::Rect, Direction, MagicInstanceBuilder, SnapMode};
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
            .build()
            .unwrap();
    }

    #[test]
    fn test_start_magic_with_path() {
        let _instance = MagicInstanceBuilder::new()
            .tech("sky130A")
            .port(get_port())
            .magic("/usr/local/bin/magic")
            .build()
            .unwrap();
    }

    #[test]
    fn test_select_bbox() {
        let mut instance = MagicInstanceBuilder::new()
            .port(get_port())
            .tech("sky130A")
            .cwd("src/")
            .build()
            .unwrap();
        instance.getcell("sram").unwrap();
        instance.select_bbox().unwrap();
    }

    #[test]
    fn test_set_get_box_values() {
        let mut instance = MagicInstanceBuilder::new()
            .port(get_port())
            .tech("sky130A")
            .build()
            .unwrap();

        let test_cases = [
            [0, 0, 0, 0],
            [-12, 4, 45, 67],
            [44, -12, 72, 2],
            [-83, -93, 12, -42],
        ];
        for test_case in test_cases {
            let nm_per_internal = instance.get_nm_per_internal().unwrap();
            let test_box = Rect::from_internal(
                test_case[0],
                test_case[1],
                test_case[2],
                test_case[3],
                nm_per_internal,
            );
            instance.set_box_values(test_box).unwrap();
            let rect = instance.box_values().unwrap();

            let (llx, lly, urx, ury) = rect.as_internal(nm_per_internal);
            assert_eq!(llx, test_case[0]);
            assert_eq!(lly, test_case[1]);
            assert_eq!(urx, test_case[2]);
            assert_eq!(ury, test_case[3]);
        }
    }

    #[test]
    fn test_snap_mode() {
        let mut instance = MagicInstanceBuilder::new()
            .port(get_port())
            .tech("sky130A")
            .build()
            .unwrap();

        let snap_modes = [SnapMode::Internal, SnapMode::Lambda, SnapMode::User];
        for snap_mode in snap_modes {
            instance.set_snap(snap_mode).unwrap();
            assert_eq!(instance.snap().unwrap(), snap_mode);
        }
    }

    #[test]
    fn test_labels_ports() -> Result<(), Box<dyn std::error::Error>> {
        let mut instance = MagicInstanceBuilder::new()
            .port(get_port())
            .tech("sky130A")
            .build()?;

        let layer = "metal1";

        instance.set_box_values(Rect::from_nm(0, 0, 1_000, 1_000))?;
        instance.paint(layer)?;
        instance.label_position_layer("vdd", Direction::Left, layer)?;
        instance.port_make(0)?;

        instance.set_box_values(Rect::from_nm(1_000, 0, 2_000, 1_000))?;
        instance.paint(layer)?;
        instance.label("gnd")?;
        instance.port_make(1)?;

        instance.set_box_values(Rect::from_nm(2_000, 0, 3_000, 1_000))?;
        instance.paint(layer)?;
        instance.label_position("vnb", Direction::Right)?;
        instance.port_make(2)?;

        instance.port_renumber()?;
        Ok(())
    }
}
