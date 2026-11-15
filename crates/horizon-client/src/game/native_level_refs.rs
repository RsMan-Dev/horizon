//! A level's blocks, light, biome, collision and time for native server mods, through a `LevelRef`
//! (docs/native-mods.md, "Entity and level handles", #7387). They read any
//! level the server has loaded, the one in place or another, and return
//! `None` (or `false`) on a client, for a handle of an earlier open, or for a
//! block in a chunk that isn't loaded.
use super::Game;
use horizon_client::mod_api::LevelRef;
use horizon_client::world::World;
use horizon_core::position::BlockPos;

#[cfg(test)]
mod native_level_reads_tests;

impl Game<'_> {
    /// The world a handle names now, or None for a stale or foreign one.
    pub(super) fn native_level_world(&self, level: LevelRef) -> Option<&World> {
        if self.remote.is_some() || self.native_lifecycle.epoch() != Some(level.epoch()) {
            return None;
        }
        if level.dimension() == self.dim.id() {
            return Some(&self.world);
        }
        let dim = self
            .levels
            .parked_dims()
            .into_iter()
            .find(|d| d.id() == level.dimension())?;
        self.levels.parked_world(dim)
    }
    /// The state at `pos`, if its chunk is loaded and `y` is inside the world.
    fn native_level_state(
        &self,
        level: LevelRef,
        pos: [i32; 3],
    ) -> Option<horizon_world::registry::BlockStateId> {
        let world = self.native_level_world(level)?;
        let [x, y, z] = pos;
        let inside = (world.bottom_y()..world.top_y()).contains(&y);
        (inside && world.chunks.contains_key(&(x >> 4, z >> 4))).then(|| world.block(x, y, z))
    }
    /// The block state at `pos` (`Level.getBlockState`) as `/setblock` takes
    /// it: `minecraft:oak_stairs[facing=east,half=bottom,shape=straight,waterlogged=false]`.
    /// None for a chunk that isn't loaded or a `y` outside the world. API
    /// 381.
    pub(super) fn native_level_block_state(
        &self,
        level: LevelRef,
        pos: [i32; 3],
    ) -> Option<String> {
        self.native_level_state(level, pos)
            .map(|state| self.state_string(state))
    }
    /// The block at `pos` without its properties: `minecraft:oak_stairs`.
    /// None as for `block_state`. API 381.
    pub(super) fn native_level_block(&self, level: LevelRef, pos: [i32; 3]) -> Option<String> {
        self.native_level_state(level, pos)
            .map(|state| self.res.facts().get(state).name.to_string())
    }
    /// Property `key` of the block state at `pos` (`BlockState.getValue`):
    /// `east` for `facing`. None for a block without it, or as for
    /// `block_state`. API 381.
    pub(super) fn native_level_block_property(
        &self,
        level: LevelRef,
        pos: [i32; 3],
        key: &str,
    ) -> Option<String> {
        let state = self.native_level_state(level, pos)?;
        self.pack
            .assets
            .registry
            .state(state)?
            .properties
            .get(key)
            .cloned()
    }
    /// Whether `pos` is inside the world's height and its chunk is loaded
    /// (`Level.isLoaded`, which is false outside `isInValidBounds`): a block
    /// read or write there finds the world. API 381.
    pub(super) fn native_level_is_loaded(&self, level: LevelRef, pos: [i32; 3]) -> bool {
        self.native_level_world(level).is_some_and(|w| {
            (w.bottom_y()..w.top_y()).contains(&pos[1])
                && w.chunks.contains_key(&(pos[0] >> 4, pos[2] >> 4))
        })
    }
    /// The lowest block `y` and the height of the level
    /// (`LevelHeightAccessor.getMinY`, `getHeight`). API 381.
    pub(super) fn native_level_height(&self, level: LevelRef) -> Option<(i32, i32)> {
        self.native_level_world(level)
            .map(|w| (w.shape.min_y, w.shape.height))
    }
    /// The ticks the server has run (`Level.getGameTime`), shared by every
    /// level as in vanilla. API 381.
    pub(super) fn native_level_game_time(&self, level: LevelRef) -> Option<i64> {
        self.native_level_world(level)?;
        Some(self.world.game_time)
    }
    /// The time of day in ticks (`Level.getOverworldClockTime`), shared by
    /// every level; it keeps counting past 24000. API 381.
    pub(super) fn native_level_day_time(&self, level: LevelRef) -> Option<i64> {
        self.native_level_world(level)?;
        Some(self.world.day_time)
    }
    /// The light at `pos` as `(block, sky)`, each 0 to 15, as
    /// `native_light` reads it. None for a chunk that isn't loaded.
    pub(super) fn native_level_light(&self, level: LevelRef, pos: [i32; 3]) -> Option<(u8, u8)> {
        let world = self.native_level_world(level)?;
        world.loaded(pos[0], pos[2]).then(|| {
            let light = world.light(pos[0], pos[1], pos[2]);
            (light & 15, light >> 4)
        })
    }
    /// The biome at `pos` as `native_biome` picks it. None for a chunk that
    /// isn't loaded.
    pub(super) fn native_level_biome(&self, level: LevelRef, pos: [i32; 3]) -> Option<String> {
        let world = self.native_level_world(level)?;
        let [x, y, z] = self
            .streaming
            .biome_zoom
            .quart_at_block(BlockPos::new(pos[0], pos[1], pos[2]));
        self.biome_name_in(world, [x << 2, y << 2, z << 2])
    }
    /// The collision boxes of the block at `pos`, block-local, as
    /// `native_block_collision` gives them. None for a chunk that isn't
    /// loaded or a `y` outside the world; an empty list for a block with no
    /// collision.
    pub(super) fn native_level_block_collision(
        &self,
        level: LevelRef,
        pos: [i32; 3],
    ) -> Option<Vec<[f64; 6]>> {
        self.native_level_state(level, pos)?;
        let world = self.native_level_world(level)?;
        Some(
            self.block_collision_shape_in(world, pos)
                .boxes()
                .iter()
                .map(|b| {
                    let (min, max) = (b.min_position(), b.max_position());
                    [min.x, min.y, min.z, max.x, max.y, max.z]
                })
                .collect(),
        )
    }
}
