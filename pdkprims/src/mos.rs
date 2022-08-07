use std::{collections::HashMap, fmt::Display};

use layout21::raw::geom::Dir;
use layout21::{
    raw::{AbstractPort, Cell, LayerKey, LayoutError, Rect},
    utils::Ptr,
};

use serde::{Deserialize, Serialize};

use crate::config::{Int, Uint};

/// MOSFET Types
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MosType {
    /// An n-channel transistor
    Nmos,
    /// A p-channel transistor
    Pmos,
}

impl Default for MosType {
    fn default() -> Self {
        Self::Nmos
    }
}

impl Display for MosType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            MosType::Nmos => write!(f, "nmos"),
            MosType::Pmos => write!(f, "pmos"),
        }
    }
}

/// Different flavors of MOSFETs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Intent {
    /// Ultra low threshold voltage
    Ulvt,
    /// Low threshold voltage
    Lvt,
    /// Standard threshold voltage
    Svt,
    /// High threshold voltage
    Hvt,
    /// Ultra-high threshold voltage
    Uhvt,
    /// A custom transistor flavor; effect depends on
    /// PDK specific transistor generator.
    Custom(String),
}

impl Default for Intent {
    fn default() -> Self {
        Self::Svt
    }
}

impl Display for Intent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Ulvt => write!(f, "ulvt"),
            Self::Lvt => write!(f, "lvt"),
            Self::Svt => write!(f, "svt"),
            Self::Hvt => write!(f, "hvt"),
            Self::Uhvt => write!(f, "uhvt"),
            Self::Custom(ref s) => write!(f, "{}", s),
        }
    }
}

/// A representation of all the layout parameters
/// of a single MOS device.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_builder::Builder)]
pub struct MosDevice {
    /// The type of transistor
    pub mos_type: MosType,
    /// Transistor flavor
    #[builder(default)]
    pub intent: Intent,
    /// The channel length of the transistor. The units must match
    /// those of the `[crate::Pdk]` you will use to draw the device.
    pub length: Int,
    /// The width of a single finger of the transistor. The units must match
    /// those of the `[crate::Pdk]` you will use to draw the device.
    pub width: Int,
    /// The number of fingers to draw
    #[builder(default = "1")]
    pub fingers: Uint,

    /// Omit placing metal contacts on these sources/drains.
    ///
    /// Entered as a list of indices. Index 0 corresponds to the
    /// bottom-most source/drain region. The maximum allowed index
    /// is the number of fingers of the transistor being drawn.
    ///
    /// Usually, this list should never contain 0 or `nf`. Otherwise,
    /// those sources/drains will be floating.
    #[builder(default)]
    pub skip_sd_metal: Vec<usize>,
}

impl MosDevice {
    pub fn builder() -> MosDeviceBuilder {
        MosDeviceBuilder::default()
    }

    pub fn name(&self) -> String {
        format!(
            "{}_{}_{}_{}_{}",
            self.mos_type, self.intent, self.width, self.length, self.fingers
        )
    }

    pub fn skip_sd_metal(&mut self, idx: usize) -> &mut Self {
        self.skip_sd_metal.push(idx);
        self
    }
}

/// Specifies the geometric arrangement of contacts for transistor gates.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GateContactStrategy {
    /// Attempt to place all contacts on one side (usually the left)
    SingleSide,
    /// Alternate contact placement
    Alternate,
    /// ABBA placement
    Abba,
    /// Other; effect depends on layout generator
    Other(String),
}

impl Default for GateContactStrategy {
    fn default() -> Self {
        Self::SingleSide
    }
}

/// Parameters for generating MOSFET layouts
///
/// When multiple devices are given, they will be drawn
/// with shared gates. So all devices must have the same channel length
/// and number of fingers.
#[derive(
    Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_builder::Builder,
)]
pub struct MosParams {
    /// A list of devices to draw.
    pub devices: Vec<MosDevice>,
    /// Direction in which to draw MOSFET gates.
    ///
    /// Note that some processes do not allow transistors
    /// to be rotated 90 degrees.
    pub direction: Dir,
    /// If true, place devices in a deep n-well.
    ///
    /// May not be supported by all processes.
    pub dnw: bool,

    /// Specifies how to place gate contacts
    pub contact_strategy: GateContactStrategy,
}

impl MosParams {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn name(&self) -> String {
        let mut name = String::from("ptx");

        for device in self.devices.iter() {
            name.push_str(&format!("__{}", device.name()));
        }

        name
    }

    #[inline]
    pub fn direction(&mut self, direction: Dir) -> &mut Self {
        self.direction = direction;
        self
    }
    #[inline]
    pub fn dnw(&mut self, dnw: bool) -> &mut Self {
        self.dnw = dnw;
        self
    }
    #[inline]
    pub fn contact_strategy(&mut self, contact_strategy: GateContactStrategy) -> &mut Self {
        self.contact_strategy = contact_strategy;
        self
    }

    pub fn add_device(&mut self, device: MosDevice) -> &mut Self {
        self.devices.push(device);
        self
    }

    pub fn validate(&self) -> Result<(), MosError> {
        if self.devices.is_empty() {
            return Err(MosError::NoDevices);
        }

        let start = &self.devices[0];
        if start.fingers <= 0 {
            return Err(MosError::InvalidNumFingers(start.fingers));
        }

        for device in self.devices.iter().skip(1) {
            if device.length != start.length {
                return Err(MosError::MismatchedLengths);
            } else if device.fingers != start.fingers {
                return Err(MosError::MismatchedFingers);
            }
        }

        Ok(())
    }

    #[inline]
    pub fn length(&self) -> Int {
        self.devices[0].length
    }

    #[inline]
    pub fn fingers(&self) -> Uint {
        self.devices[0].fingers
    }
}

/// Represents the geometric arrangement of
/// a laid-out collection of transistors.
///
/// The transistors are assumed to have a common set of gates,
/// such as in a typical digital inverter layout.
#[derive(Debug, Clone, Eq, PartialEq, derive_builder::Builder)]
pub struct LayoutTransistors {
    /// A pointer to the layout cell.
    pub cell: Ptr<Cell>,
    /// The layer to which device sources/drains are connected.
    pub sd_metal: LayerKey,
    /// The layer to which gates are connected.
    pub gate_metal: LayerKey,
    /// A collection of the positions of source/drain pins.
    ///
    /// `sd_pins[i][j]` is the metal region corresponding to
    /// device `i`'s `j`'th source/drain region. Zero-indexed.
    ///
    /// Dimensions: (# devices) x (# fingers + 1)
    ///
    /// The `Option<Rect>` will be [`None`] if `skip_sd_metal` was
    /// set for the given source/drain region; otherwise,
    /// it will be [`Some`].
    pub sd_pins: Vec<HashMap<Uint, Option<Rect>>>,
    /// A collection of the positions of the gate pins.
    ///
    /// `gate_pins[i]` is the metal region corresponding to
    /// the `i`'th finger of the transistors. Zero-indexed.
    pub gate_pins: Vec<Rect>,
}

impl LayoutTransistors {
    pub fn gate_port(&self, i: Uint) -> Option<AbstractPort> {
        assert!(i >= 0);

        self.get_port(&format!("gate_{}", i))
    }

    pub fn sd_port(&self, i: Uint, j: Uint) -> Option<AbstractPort> {
        assert!(i >= 0);
        assert!(j >= 0);

        self.get_port(&format!("sd_{}_{}", i, j))
    }

    fn get_port(&self, name: &str) -> Option<AbstractPort> {
        let cell = self.cell.read().unwrap();
        let abs = cell.abs.as_ref().unwrap();
        abs.ports.iter().find(|p| p.net == name).map(Clone::clone)
    }

    pub fn sd_pin(&self, i: Uint, j: Uint) -> Option<Rect> {
        assert!(i >= 0);
        assert!(j >= 0);

        *self.sd_pins.get(i as usize)?.get(&j)?
    }

    pub fn gate_pin(&self, i: Uint) -> Option<Rect> {
        assert!(i >= 0);

        self.gate_pins.get(i as usize).copied()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MosError {
    #[error("mismatched lengths (not all devices have the same channel length)")]
    MismatchedLengths,
    #[error("mismatched number of fingers (not all devices have the same number of fingers)")]
    MismatchedFingers,
    #[error("invalid number of fingers: {0}")]
    InvalidNumFingers(Uint),
    #[error("invalid params: {0}")]
    BadParams(String),
    #[error("no devices to draw")]
    NoDevices,

    #[error("error doing layout: {0}")]
    Layout(#[from] LayoutError),
}

pub type MosResult<T> = std::result::Result<T, MosError>;
