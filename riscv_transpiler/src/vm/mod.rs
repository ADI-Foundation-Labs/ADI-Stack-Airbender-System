use crate::ir::DelegationType;
use crate::ir::Instruction;
use crate::ir::InstructionName;
use common_constants::{TimestampScalar, INITIAL_TIMESTAMP, TIMESTAMP_STEP};
use std::fmt::Debug;

mod instructions;
mod ram_with_rom_region;
mod replay_snapshotter;
mod simple_tape;

pub(crate) mod delegations;

pub use self::ram_with_rom_region::RamWithRomRegion;
pub use self::replay_snapshotter::*;
pub use self::simple_tape::SimpleTape;

pub trait Counters: 'static + Clone + Copy + Debug {
    fn bump_bigint(&mut self);
    fn bump_blake2_round_function(&mut self);
    fn bump_keccak_special_5(&mut self);
    fn bump_non_determinism(&mut self);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C, align(16))]
pub struct Register {
    pub timestamp: TimestampScalar,
    pub value: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct State<C: Counters> {
    pub registers: [Register; 32],
    pub timestamp: TimestampScalar,
    pub pc: u32,
    pub counters: C,
}

impl<C: Counters> State<C> {
    pub fn initial_with_counters(counters: C) -> Self {
        Self {
            registers: [Register {
                value: 0,
                timestamp: 0,
            }; 32],
            counters,
            timestamp: INITIAL_TIMESTAMP,
            pc: 0,
        }
    }
}

pub trait Snapshotter: 'static {
    type Counters: Counters;

    fn take_snapshot(&mut self, state: &State<Self::Counters>);
    fn append_non_determinism_read(&mut self, value: u32);
    fn append_memory_read(
        &mut self,
        address: u32,
        read_value: u32,
        read_timestamp: TimestampScalar,
        write_timestamp: TimestampScalar,
    );
}

pub trait RAM {
    fn peek_word(&self, address: u32) -> u32;
    fn read_word(&mut self, address: u32, timestamp: TimestampScalar) -> (TimestampScalar, u32);
    fn mask_read_value_for_witness(&self, address: u32, value: &mut u32);
    fn write_word(
        &mut self,
        address: u32,
        word: u32,
        timestamp: TimestampScalar,
    ) -> (TimestampScalar, u32);
}

pub trait InstructionTape {
    fn read_instruction(&self, pc: u32) -> Instruction;
}

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

pub struct VM<S: Snapshotter, R: RAM> {
    pub state: State<S::Counters>,
    _marker: core::marker::PhantomData<R>,
}

impl<S: Snapshotter, R: RAM> VM<S, R> {
    pub fn run_basic_unrolled<ND: NonDeterminismCSRSource<R>>(
        state: &mut State<S::Counters>,
        num_snapshots: usize,
        ram: &mut R,
        snapshotter: &mut S,
        instruction_tape: &impl InstructionTape,
        snapshot_period: usize,
        nd: &mut ND,
    ) {
        use crate::vm::instructions::*;

        for _ in 0..num_snapshots {
            for _ in 0..snapshot_period {
                unsafe {
                    let pc = state.pc;
                    let instr = instruction_tape.read_instruction(pc);
                    match instr.name {
                        InstructionName::Illegal => illegal(state, ram, snapshotter, instr),
                        InstructionName::Lui => {
                            lui_auipc::lui::<_, _>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Auipc => {
                            lui_auipc::auipc::<_, _>(state, ram, snapshotter, instr)
                        }

                        InstructionName::Jal => {
                            jal_jalr::jal::<_, _>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Jalr => {
                            jal_jalr::jalr::<_, _>(state, ram, snapshotter, instr)
                        }

                        InstructionName::Slt => {
                            slt::slt::<_, _, false>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Slti => {
                            slt::slt::<_, _, true>(state, ram, snapshotter, instr)
                        }

                        InstructionName::Sltu => {
                            slt::sltu::<_, _, false>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Sltiu => {
                            slt::sltu::<_, _, true>(state, ram, snapshotter, instr)
                        }

                        InstructionName::Branch => {
                            branch::branch::<_, _>(state, ram, snapshotter, instr)
                        }

                        InstructionName::Sw => memory::sw::<_, _>(state, ram, snapshotter, instr),
                        InstructionName::Lw => memory::lw::<_, _>(state, ram, snapshotter, instr),

                        InstructionName::Sh => memory::sh::<_, _>(state, ram, snapshotter, instr),
                        InstructionName::Lhu => {
                            memory::lh::<_, _, false>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Lh => {
                            memory::lh::<_, _, true>(state, ram, snapshotter, instr)
                        }

                        InstructionName::Sb => memory::sb::<_, _>(state, ram, snapshotter, instr),
                        InstructionName::Lbu => {
                            memory::lb::<_, _, false>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Lb => {
                            memory::lb::<_, _, true>(state, ram, snapshotter, instr)
                        }

                        InstructionName::Add => {
                            add_sub::add_op::<_, _, false>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Addi => {
                            add_sub::add_op::<_, _, true>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Sub => {
                            add_sub::sub_op::<_, _>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Xor => {
                            binary::xor::<_, _, false>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Xori => {
                            binary::xor::<_, _, true>(state, ram, snapshotter, instr)
                        }
                        InstructionName::And => {
                            binary::and::<_, _, false>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Andi => {
                            binary::and::<_, _, true>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Or => {
                            binary::and::<_, _, false>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Ori => {
                            binary::and::<_, _, true>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Sll => {
                            shifts::sll::<_, _, false>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Slli => {
                            shifts::sll::<_, _, true>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Srl => {
                            shifts::srl::<_, _, false>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Srli => {
                            shifts::srl::<_, _, true>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Sra => {
                            shifts::sra::<_, _, false>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Srai => {
                            shifts::sra::<_, _, true>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Mul => {
                            mul_div::mul::<_, _>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Mulhu => {
                            mul_div::mulhu::<_, _>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Divu => {
                            mul_div::divu::<_, _>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Remu => {
                            mul_div::remu::<_, _>(state, ram, snapshotter, instr)
                        }
                        InstructionName::ZicsrNonDeterminismRead => {
                            zicsr::nd_read::<_, _, ND>(state, ram, snapshotter, instr, nd)
                        }
                        InstructionName::ZicsrNonDeterminismWrite => {
                            zicsr::nd_write::<_, _, ND>(state, ram, snapshotter, instr, nd)
                        }
                        InstructionName::ZicsrDelegation => {
                            zicsr::call_delegation::<_, _>(state, ram, snapshotter, instr)
                        }
                        _ => core::hint::unreachable_unchecked(),
                    }
                    if state.pc == pc {
                        snapshotter.take_snapshot(&*state);
                        return;
                    }
                    state.timestamp += TIMESTAMP_STEP;
                }
            }

            snapshotter.take_snapshot(&*state);
        }

        panic!("out of cycles");
    }
}

pub fn run_default(
    num_snapshots: usize,
    ram: &mut RamWithRomRegion<5>,
    snapshotter: &mut SimpleSnapshotter<5>,
    instruction_tape: &mut SimpleTape,
    snapshot_period: usize,
) {
    let mut state = State::initial_with_counters(DelegationsCounters::default());
    VM::<SimpleSnapshotter<5>, RamWithRomRegion<5>>::run_basic_unrolled(
        &mut state,
        num_snapshots,
        ram,
        snapshotter,
        instruction_tape,
        snapshot_period,
        &mut (),
    )
}

#[cfg(test)]
mod test {
    use crate::ir::decode;
    use crate::ir::FullUnsignedMachineDecoderConfig;

    use super::*;
    use std::path::Path;

    pub fn read_binary(path: &Path) -> (Vec<u8>, Vec<u32>) {
        use std::io::Read;
        let mut file = std::fs::File::open(path).expect("must open provided file");
        let mut buffer = vec![];
        file.read_to_end(&mut buffer).expect("must read the file");
        assert_eq!(buffer.len() % core::mem::size_of::<u32>(), 0);
        let mut binary = Vec::with_capacity(buffer.len() / core::mem::size_of::<u32>());
        for el in buffer.as_chunks::<4>().0 {
            binary.push(u32::from_le_bytes(*el));
        }

        (buffer, binary)
    }

    #[test]
    fn test_simple_fibonacci() {
        let (_, binary) = read_binary(&Path::new("examples/fibonacci/app.bin"));
        let (_, text) = read_binary(&Path::new("examples/fibonacci/app.text"));
        let instructions: Vec<Instruction> = text
            .into_iter()
            .map(|el| decode::<FullUnsignedMachineDecoderConfig>(el))
            .collect();
        let tape = SimpleTape::new(&instructions);
        let mut ram = RamWithRomRegion::<5>::from_rom_content(&binary, 1 << 30);
        let period = 1 << 20;
        let num_snapshots = 1000;
        let cycles_bound = period * num_snapshots;
        let mut snapshotter = SimpleSnapshotter::new_with_cycle_limit(cycles_bound, period);

        let mut state = State::initial_with_counters(DelegationsCounters::default());
        let now = std::time::Instant::now();
        VM::<SimpleSnapshotter<5>, RamWithRomRegion<5>>::run_basic_unrolled(
            &mut state,
            num_snapshots,
            &mut ram,
            &mut snapshotter,
            &tape,
            period,
            &mut (),
        );
        let elapsed = now.elapsed();

        let total_snapshots = snapshotter.snapshots.len();
        let cycles_upper_bound = total_snapshots * period;

        println!(
            "Performance is {} MHz ({} total snapshots with period of {} cycles)",
            (cycles_upper_bound as f64) / (elapsed.as_micros() as f64),
            total_snapshots,
            period
        );

        dbg!(&state.registers[10..18]);
    }
}
