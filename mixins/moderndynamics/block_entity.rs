// A placed item pipe declares a block entity, which saves its extractor
// attachments (`native_block_entity`, API 15).
fn observe_block_entity(block: &str, entity: &mut Option<(String, horizon_world::nbt::Compound)>) {
    crate::game::moderndynamics_state::on_block_entity(block, entity);
}
