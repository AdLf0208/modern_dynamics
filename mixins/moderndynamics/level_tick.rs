// After the level's tick: refresh the pipes' drawn connections and run the
// extractors' operations (`native_level_tick_post`, API 23).
fn observe_level_tick_post(game: &mut crate::game::Game<'_>, dimension: &str) {
    crate::game::moderndynamics_state::on_level_tick_post(game, dimension);
}
