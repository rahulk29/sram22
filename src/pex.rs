use std::fmt::Display;

use serde::{Deserialize, Serialize};
use substrate::component::{error, Component};
use substrate::error::ErrorSource;

#[derive(
    Copy, Clone, Eq, PartialEq, Default, Debug, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
pub enum PexCorner {
    #[default]
    Typical,
    HRHC,
    HRLC,
    LRHC,
    LRLC,
}

impl PexCorner {
    pub fn as_str(&self) -> &'static str {
        match *self {
            Self::Typical => "Typical",
            Self::HRHC => "HRHC",
            Self::HRLC => "HRLC",
            Self::LRHC => "LRHC",
            Self::LRLC => "LRLC",
        }
    }
}

impl Display for PexCorner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub struct Pex<T: Component> {
    params: T::Params,
}

impl<P: Clone + Serialize, T: Component<Params = P>> Component for Pex<T> {
    type Params = T::Params;
    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::format!("pex")
    }
    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        use std::fmt::Write;

        let inner = ctx.instantiate::<T>(&self.params)?.named("Xdut");
        let mut s = inner.name().to_string();
        for port in inner.ports()? {
            ctx.bus_port(port.name(), port.width(), port.direction());
            for i in 0..port.width() {
                if port.width > 1 {
                    write!(&mut s, " {}[{}]", port.name(), i).unwrap();
                } else {
                    write!(&mut s, " {}", port.name()).unwrap();
                }
            }
        }
        write!(&mut s, " {}", inner.module().local().unwrap().name()).unwrap();
        ctx.set_spice(s);
        Ok(())
    }

    fn layout(
        &self,
        _ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        Err(ErrorSource::Component(error::Error::ViewUnsupported(
            substrate::component::View::Layout,
        ))
        .into())
    }
}
