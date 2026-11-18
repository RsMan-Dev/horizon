//! The boxes of the carts near a cart, taken once when its tick begins ([`NearCarts`]), so its moves
//! do not work out every cart's box again.
use super::*;

/// How far past a cart's box the boxes are taken when its tick begins (a longer move asks the list).
pub(super) const NEAR_CARTS_REACH: f64 = 8.0;
impl CartIndex {
    /// The boxes of the other carts that meet `area`, in list order, for the cart in slot `i`;
    /// None: the cart asks the list itself.
    pub(super) fn near_snapshot(
        &mut self,
        carts: &[Minecart],
        i: usize,
        area: Aabb,
    ) -> Option<NearCarts> {
        #[cfg(test)]
        if tests_support::NEAR_CARTS_OFF.get() {
            return None;
        }
        if !self.pass {
            return None;
        }
        let mut slots = Vec::new();
        if !self.carts_near(carts, i, area, &mut slots) {
            return None;
        }
        let id = carts[i].id;
        let boxes = slots
            .iter()
            .map(|&j| &carts[j as usize])
            .filter(|o| o.id != id)
            .map(Minecart::bounds)
            .filter(|b| b.intersects(area))
            .collect();
        Some(NearCarts { area, boxes })
    }
}
/// What the tests switch: the boxes taken at the start of a tick off, and how often one answered.
#[cfg(test)]
pub(super) mod tests_support {
    use std::cell::Cell;
    thread_local! {
        pub(in crate::game) static NEAR_CARTS_OFF: Cell<bool> = const { Cell::new(false) };
        pub(in crate::game) static NEAR_CARTS_ANSWERS: Cell<u32> = const { Cell::new(0) };
    }
}
