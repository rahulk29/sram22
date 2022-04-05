use layout21::raw::LayerKey;

use crate::Pdk;

pub trait Sky130Pdk {
    fn diff(&self) -> LayerKey;
    fn poly(&self) -> LayerKey;
    fn npc(&self) -> LayerKey;
    fn nwell(&self) -> LayerKey;
    fn licon1(&self) -> LayerKey;
    fn li1(&self) -> LayerKey;
    fn met1(&self) -> LayerKey;
    fn via(&self) -> LayerKey;
    fn met2(&self) -> LayerKey;
    fn via2(&self) -> LayerKey;
    fn met3(&self) -> LayerKey;
}

impl Sky130Pdk for Pdk {
    fn diff(&self) -> LayerKey {
        self.get_layerkey("diff").unwrap()
    }
    fn poly(&self) -> LayerKey {
        self.get_layerkey("poly").unwrap()
    }
    fn npc(&self) -> LayerKey {
        self.get_layerkey("npc").unwrap()
    }
    fn nwell(&self) -> LayerKey {
        self.get_layerkey("nwell").unwrap()
    }
    fn licon1(&self) -> LayerKey {
        self.get_layerkey("licon").unwrap()
    }
    fn li1(&self) -> LayerKey {
        self.get_layerkey("li").unwrap()
    }
    fn met1(&self) -> LayerKey {
        self.get_layerkey("m1").unwrap()
    }
    fn via(&self) -> LayerKey {
        self.get_layerkey("via").unwrap()
    }
    fn met2(&self) -> LayerKey {
        self.get_layerkey("m2").unwrap()
    }
    fn via2(&self) -> LayerKey {
        self.get_layerkey("via2").unwrap()
    }
    fn met3(&self) -> LayerKey {
        self.get_layerkey("m3").unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::Sky130Pdk;

    #[test]
    fn test_sky130_pdk_layers() -> Result<(), Box<dyn std::error::Error>> {
        let pdk = super::super::pdk()?;
        let _ = pdk.diff();
        let _ = pdk.poly();
        let _ = pdk.npc();
        let _ = pdk.nwell();
        let _ = pdk.licon1();
        let _ = pdk.li1();
        let _ = pdk.met1();
        let _ = pdk.via();
        let _ = pdk.met2();
        let _ = pdk.via2();
        let _ = pdk.met3();
        Ok(())
    }
}
