pub struct GuardRingParams {
    pub name: String,
    pub enclosure: Rect,
}

#[derive(Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Builder)]
pub struct RingParams {
    enclosure: Rect,
    h_width: Int,
    v_width: Int,
}

impl RingParams {
    #[inline]
    pub fn builder() -> RingParamsBuilder {
        RingParamsBuilder::default()
    }

    #[inline]
    pub fn draw(self) -> Ring {
        draw_ring(self)
    }
}
