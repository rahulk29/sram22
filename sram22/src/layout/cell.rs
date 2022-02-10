use magic_vlsi::units::Rect;

pub struct LayoutCell {
    pub bbox: Rect,
    pub ports: Vec<LayoutPort>,
}

pub struct LayoutPort {
    pub name: String,
    pub bbox: Rect,
    pub layer: String,
}
