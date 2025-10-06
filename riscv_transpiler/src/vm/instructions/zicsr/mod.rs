use super::*;

// there is no interpretation of methods here, it's just read/write and that's all
pub trait NonDeterminismCSRSource<R: RAM + ?Sized> {
    fn read(&mut self) -> u32;

    // we in general can allow CSR source to peek into memory (readonly)
    // to perform adhoc computations to prepare result. This will allow to save on
    // passing large structures
    fn write_with_memory_access(&mut self, ram: &R, value: u32);
}

impl<R: RAM> NonDeterminismCSRSource<R> for () {
    fn read(&mut self) -> u32 {
        0u32
    }
    fn write_with_memory_access(&mut self, _ram: &R, _value: u32) {}
}

#[inline(always)]
pub(crate) fn nd_read<S: Snapshotter, R: RAM, ND: NonDeterminismCSRSource<R>>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    snapshotter: &mut S,
    instr: Instruction,
    nd: &mut ND,
) {
    let _rs1_value = read_register::<S, 0>(state, instr.rs1);
    let _rs2_value = read_register::<S, 1>(state, instr.rs2); // formal
    let rd = nd.read();
    snapshotter.append_non_determinism_read(rd);
    state.counters.bump_non_determinism();
    write_register::<S, 2>(state, instr.rd, rd);
    default_increase_pc::<S>(state);
}

#[inline(always)]
pub(crate) fn nd_write<S: Snapshotter, R: RAM, ND: NonDeterminismCSRSource<R>>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    snapshotter: &mut S,
    instr: Instruction,
    nd: &mut ND,
) {
    let rs1_value = read_register::<S, 0>(state, instr.rs1);
    let _rs2_value = read_register::<S, 1>(state, instr.rs2); // formal
    nd.write_with_memory_access(&*ram, rs1_value);
    write_register::<S, 2>(state, instr.rd, 0);
    default_increase_pc::<S>(state);
}
