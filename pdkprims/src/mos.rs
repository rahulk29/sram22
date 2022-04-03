use std::fmt::Display;

use layout21::raw::LayoutError;
use serde::{Deserialize, Serialize};

use crate::{
    config::{Int, Uint},
    geometry::CoarseDirection,
};

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
    pub intent: Intent,
    /// The channel length of the transistor. The units must match
    /// those of the `[crate::Pdk]` you will use to draw the device.
    pub length: Int,
    /// The width of a single finger of the transistor. The units must match
    /// those of the `[crate::Pdk]` you will use to draw the device.
    pub width: Int,
    /// The number of fingers to draw
    pub fingers: Uint,
}

impl MosDevice {
    pub fn builder() -> MosDeviceBuilder {
        MosDeviceBuilder::default()
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
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MosParams {
    /// A list of devices to draw.
    pub devices: Vec<MosDevice>,
    /// Direction in which to draw MOSFET gates.
    ///
    /// Note that some processes do not allow transistors
    /// to be rotated 90 degrees.
    pub direction: CoarseDirection,
    /// If true, place devices in a deep n-well.
    ///
    /// May not be supported by all processes.
    pub dnw: bool,

    /// Specifies how to place gate contacts
    pub contact_strategy: GateContactStrategy,

    /// Omit placing metal contacts on these sources/drains.
    ///
    /// Entered as a list of indices. Index 0 corresponds to the
    /// bottom-most source/drain region. The maximum allowed index
    /// is the number of fingers of the transistor being drawn.
    ///
    /// Usually, this list should never contain 0 or `nf`. Otherwise,
    /// those sources/drains will be floating.
    pub skip_sd_metal: Vec<usize>,
}

impl MosParams {
    pub fn new() -> Self {
        Default::default()
    }

    #[inline]
    pub fn direction(&mut self, direction: CoarseDirection) -> &mut Self {
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
    pub fn skip_sd_metal(&mut self, idx: usize) -> &mut Self {
        self.skip_sd_metal.push(idx);
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
