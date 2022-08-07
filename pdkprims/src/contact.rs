use std::collections::HashMap;
use std::fmt::Display;

use layout21::raw::geom::Dir;
use layout21::raw::{Cell, LayerKey, Rect};
use layout21::utils::Ptr;
use serde::{Deserialize, Serialize};

use crate::config::Int;
use crate::Ref;
use crate::{config::Uint, Pdk};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, derive_builder::Builder)]
pub struct ContactParams {
    pub stack: String,
    pub rows: Uint,
    pub cols: Uint,
    /// The "relaxed" direction, ie. the direction in which there is more margin (for overhangs,
    /// for instance).
    ///
    /// If the contact generator needs more space, it will try to expand in
    /// this direction first.
    pub dir: Dir,
}

#[derive(Debug, Clone, Eq, PartialEq, derive_builder::Builder)]
pub struct Contact {
    pub cell: Ptr<Cell>,
    pub rows: Uint,
    pub cols: Uint,
    pub bboxes: HashMap<LayerKey, Rect>,
}

impl Display for ContactParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}_{}x{}{}",
            &self.stack,
            self.rows,
            self.cols,
            self.dir.short_form()
        )
    }
}

impl ContactParams {
    pub fn builder() -> ContactParamsBuilder {
        ContactParamsBuilder::default()
    }
}

impl Pdk {
    pub fn get_contact(&self, params: &ContactParams) -> Ref<Contact> {
        let mut map = self.contacts.write().unwrap();
        if let Some(c) = map.get(params) {
            c.clone()
        } else {
            let c = self.draw_contact(params);
            map.insert(params.to_owned(), c.clone());
            c
        }
    }

    pub fn get_contact_sized(
        &self,
        stack: impl Into<String>,
        layer: LayerKey,
        width: Int,
    ) -> Option<Ref<Contact>> {
        let mut low = 1;
        let mut high = 100;
        let mut result = None;

        let stack = stack.into();

        while high > low {
            let mid = (high + low) / 2;
            let params = ContactParams::builder()
                .rows(1)
                .cols(mid)
                .stack(stack.clone())
                .dir(Dir::Horiz)
                .build()
                .unwrap();
            let ct = self.get_contact(&params);
            let bbox = ct.bboxes.get(&layer).unwrap();

            if bbox.p1.x - bbox.p0.x < width {
                result = Some(ct);
                low = mid + 1;
            } else {
                high = mid;
            }
        }

        result
    }
}
