// Right-clicking a pipe with an extractor places it on the clicked side
// (`native_right_click_block`, API 21).
fn observe_right_click_block(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    face: [i32; 3],
    canceled: &mut bool,
) {
    crate::game::moderndynamics_state::on_right_click_block(game, pos, face, canceled);
}
