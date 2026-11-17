//! The boxes of the carts near a cart, taken once when its tick begins ([`NearCarts`]), so its moves
//! do not work out every cart's box again.
use super::*;

/// The boxes of the other carts that meet `area`, taken in list order when a cart's tick began: the
/// carts do not move while one of them ticks, so the boxes its moves meet inside `area` are in here.
#[derive(Clone, Debug)]
pub(super) struct NearCarts {
    area: Aabb,
    pub(super) boxes: Vec<Aabb>,
}
/// How far past a cart's box the boxes are taken when its tick begins (a longer move asks the list).
pub(super) const NEAR_CARTS_REACH: f64 = 8.0;
impl NearCarts {
    /// The boxes that meet `near`, in list order; None when `near` is not inside the area taken.
    pub(super) fn meeting(&self, near: Aabb) -> Option<Vec<Aabb>> {
        let inside = [
            horizon_core::geometry::Axis::X,
            horizon_core::geometry::Axis::Y,
            horizon_core::geometry::Axis::Z,
        ]
        .into_iter()
        .all(|a| self.area.min(a) <= near.min(a) && near.max(a) <= self.area.max(a));
        #[cfg(test)]
        if inside {
            tests_support::NEAR_CARTS_ANSWERS.set(tests_support::NEAR_CARTS_ANSWERS.get() + 1);
        }
        inside.then(|| {
            self.boxes
                .iter()
                .copied()
                .filter(|b| b.intersects(near))
                .collect()
        })
    }
}

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
impl Game<'_> {
    /// `AbstractMinecart.tick` for one cart; returns false once it is gone. It takes the boxes of the
    /// carts near it once, for the moves of its tick ([`Self::tick_cart_body`]).
    pub(super) fn tick_cart(
        &mut self,
        c: &mut Minecart,
        intent: [f64; 2],
        slot: usize,
        index: &mut CartIndex,
    ) -> bool {
        let area = c.bounds().inflate(Vec3::new(
            NEAR_CARTS_REACH,
            NEAR_CARTS_REACH,
            NEAR_CARTS_REACH,
        ));
        c.near_carts = index
            .near_snapshot(&self.carts, slot, area)
            .map(std::sync::Arc::new);
        let alive = self.tick_cart_body(c, intent, slot, index);
        c.near_carts = None;
        alive
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
