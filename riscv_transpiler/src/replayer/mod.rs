use crate::ir::DelegationType;
use crate::ir::Instruction;
use crate::ir::InstructionName;
use crate::vm::InstructionTape;
use crate::vm::NonDeterminismCSRSource;
use crate::vm::Snapshotter;
use crate::vm::State;
use crate::vm::RAM;
use crate::witness::WitnessTracer;
use common_constants::circuit_families::*;
use common_constants::TimestampScalar;
use common_constants::TIMESTAMP_STEP;
use risc_v_simulator::machine_mode_only_unrolled::TimestampData;

mod delegations;
mod instructions;

#[derive(Clone, Copy, Debug)]
pub struct ReplayerRam<'a, const ROM_BOUND_SECOND_WORD_BITS: usize> {
    pub ram_log: &'a [(u32, (u32, u32))],
}

#[derive(Clone, Copy, Debug)]
pub struct ReplayerNonDeterminism<'a> {
    pub non_determinism_reads_log: &'a [u32],
}

impl<'a, const ROM_BOUND_SECOND_WORD_BITS: usize> RAM
    for ReplayerRam<'a, ROM_BOUND_SECOND_WORD_BITS>
{
    fn peek_word(&self, _address: u32) -> u32 {
        unimplemented!("must not be used for simplicity");
    }

    #[inline(always)]
    fn read_word(&mut self, address: u32, _timestamp: TimestampScalar) -> (TimestampScalar, u32) {
        debug_assert_eq!(address % 4, 0);
        debug_assert!(self.ram_log.len() > 0);
        unsafe {
            let (value, (low, high)) = *self.ram_log.get_unchecked(0);
            self.ram_log = core::mem::transmute(self.ram_log.get_unchecked(1..));

            (
                (low as TimestampScalar) | ((high as TimestampScalar) << 32),
                value,
            )
        }
    }

    #[inline(always)]
    fn mask_read_value_for_witness(&self, address: u32, value: &mut u32) {
        // we do not do anything here
        debug_assert_eq!(address % 4, 0);
        let word_idx = (address / 4) as usize;
        if word_idx < 1 << (16 + ROM_BOUND_SECOND_WORD_BITS) / core::mem::size_of::<u32>() {
            *value = 0u32;
        }
    }

    #[inline(always)]
    fn write_word(
        &mut self,
        address: u32,
        _word: u32,
        _timestamp: TimestampScalar,
    ) -> (TimestampScalar, u32) {
        debug_assert_eq!(address % 4, 0);
        debug_assert!(self.ram_log.len() > 0);
        unsafe {
            let (value, (low, high)) = *self.ram_log.get_unchecked(0);
            self.ram_log = core::mem::transmute(self.ram_log.get_unchecked(1..));

            (
                (low as TimestampScalar) | ((high as TimestampScalar) << 32),
                value,
            )
        }
    }
}

pub struct ReplayerVM<S: Snapshotter, R: RAM> {
    pub state: State<S::Counters>,
    _marker: core::marker::PhantomData<R>,
}

impl<S: Snapshotter, R: RAM> ReplayerVM<S, R> {
    pub fn run_basic_unrolled<ND: NonDeterminismCSRSource<R>>(
        state: &mut State<S::Counters>,
        num_snapshots: usize,
        ram: &mut R,
        instruction_tape: &impl InstructionTape,
        snapshot_period: usize,
        nd: &mut ND,
        tracer: &mut impl WitnessTracer,
    ) {
        use crate::replayer::instructions::*;

        for _ in 0..num_snapshots {
            for _ in 0..snapshot_period {
                unsafe {
                    let pc = state.pc;
                    let instr = instruction_tape.read_instruction(pc);
                    match instr.name {
                        InstructionName::Illegal => illegal::<S, R>(state, ram, instr, tracer),
                        InstructionName::Lui => lui_auipc::lui::<S, R>(state, ram, instr, tracer),
                        InstructionName::Auipc => {
                            lui_auipc::auipc::<S, R>(state, ram, instr, tracer)
                        }

                        InstructionName::Jal => jal_jalr::jal::<S, R>(state, ram, instr, tracer),
                        InstructionName::Jalr => jal_jalr::jalr::<S, R>(state, ram, instr, tracer),

                        InstructionName::Slt => slt::slt::<S, R, false>(state, ram, instr, tracer),
                        InstructionName::Slti => slt::slt::<S, R, true>(state, ram, instr, tracer),

                        InstructionName::Sltu => {
                            slt::sltu::<S, R, false>(state, ram, instr, tracer)
                        }
                        InstructionName::Sltiu => {
                            slt::sltu::<S, R, true>(state, ram, instr, tracer)
                        }

                        InstructionName::Branch => {
                            branch::branch::<S, R>(state, ram, instr, tracer)
                        }

                        InstructionName::Sw => memory::sw::<S, R>(state, ram, instr, tracer),
                        InstructionName::Lw => memory::lw::<S, R>(state, ram, instr, tracer),

                        InstructionName::Sh => memory::sh::<S, R>(state, ram, instr, tracer),
                        InstructionName::Lhu => {
                            memory::lh::<S, R, false>(state, ram, instr, tracer)
                        }
                        InstructionName::Lh => memory::lh::<S, R, true>(state, ram, instr, tracer),

                        InstructionName::Sb => memory::sb::<S, R>(state, ram, instr, tracer),
                        InstructionName::Lbu => {
                            memory::lb::<S, R, false>(state, ram, instr, tracer)
                        }
                        InstructionName::Lb => memory::lb::<S, R, true>(state, ram, instr, tracer),

                        InstructionName::Add => {
                            add_sub::add_op::<S, R, false>(state, ram, instr, tracer)
                        }
                        InstructionName::Addi => {
                            add_sub::add_op::<S, R, true>(state, ram, instr, tracer)
                        }
                        InstructionName::Sub => add_sub::sub_op::<S, R>(state, ram, instr, tracer),
                        InstructionName::Xor => {
                            binary::xor::<S, R, false>(state, ram, instr, tracer)
                        }
                        InstructionName::Xori => {
                            binary::xor::<S, R, true>(state, ram, instr, tracer)
                        }
                        InstructionName::And => {
                            binary::and::<S, R, false>(state, ram, instr, tracer)
                        }
                        InstructionName::Andi => {
                            binary::and::<S, R, true>(state, ram, instr, tracer)
                        }
                        InstructionName::Or => {
                            binary::and::<S, R, false>(state, ram, instr, tracer)
                        }
                        InstructionName::Ori => {
                            binary::and::<S, R, true>(state, ram, instr, tracer)
                        }
                        InstructionName::Sll => {
                            shifts::sll::<S, R, false>(state, ram, instr, tracer)
                        }
                        InstructionName::Slli => {
                            shifts::sll::<S, R, true>(state, ram, instr, tracer)
                        }
                        InstructionName::Srl => {
                            shifts::srl::<S, R, false>(state, ram, instr, tracer)
                        }
                        InstructionName::Srli => {
                            shifts::srl::<S, R, true>(state, ram, instr, tracer)
                        }
                        InstructionName::Sra => {
                            shifts::sra::<S, R, false>(state, ram, instr, tracer)
                        }
                        InstructionName::Srai => {
                            shifts::sra::<S, R, true>(state, ram, instr, tracer)
                        }
                        InstructionName::Mul => mul_div::mul::<S, R>(state, ram, instr, tracer),
                        InstructionName::Mulhu => mul_div::mulhu::<S, R>(state, ram, instr, tracer),
                        InstructionName::Divu => mul_div::divu::<S, R>(state, ram, instr, tracer),
                        InstructionName::Remu => mul_div::remu::<S, R>(state, ram, instr, tracer),
                        InstructionName::ZicsrNonDeterminismRead => {
                            zicsr::nd_read::<S, R, ND>(state, ram, instr, tracer, nd)
                        }
                        InstructionName::ZicsrNonDeterminismWrite => {
                            zicsr::nd_write::<S, R>(state, ram, instr, tracer)
                        }
                        InstructionName::ZicsrDelegation => {
                            zicsr::call_delegation::<S, R>(state, ram, instr, tracer)
                        }
                        _ => core::hint::unreachable_unchecked(),
                    }
                    if state.pc == pc {
                        return;
                    }
                    state.timestamp += TIMESTAMP_STEP;
                }
            }
        }
    }
}
