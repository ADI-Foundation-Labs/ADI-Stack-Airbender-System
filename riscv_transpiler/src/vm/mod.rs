use std::fmt::Debug;
use crate::ir::Instruction;
use common_constants::TimestampScalar;

mod instructions;

pub trait Counters: 'static + Clone + Copy + Debug {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C, align(16))]
pub struct Register {
    timestamp: TimestampScalar,
    value: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct State<C: Counters> {
    pub registers: [Register; 32],
    pub pc: u32,
    pub counters: C,
}

pub trait Snapshotter: 'static {
    type Counters: Counters;

    fn take_snapshot(&mut self, state: &State<Self::Counters>);
    fn append_memory_log(&mut self, read_value: u32, read_timestamp: TimestampScalar);
}

pub trait RAM {
    fn read_word(&mut self, address: u32) -> (TimestampScalar, u32);
    fn write_word(&mut self, word: u32, timestamp: TimestampScalar);
}

pub struct VM<S: Snapshotter, R: RAM> {
    pub state: State<S::Counters>,
    pub impls: [fn(&mut Self, &mut R, &mut S); 256],
}

pub trait InstructionTape {
    fn read_instruction(&self, pc: u32) -> Instruction;
}

impl<S: Snapshotter, R: RAM> VM<S, R>{
    pub fn run(&mut self, num_cycles: usize, ram: &mut R, snapshotter: &mut S, instruction_tape: &impl InstructionTape) {
        for _ in 0..num_cycles {
            unsafe {
                let pc = self.state.pc;
                let instr = instruction_tape.read_instruction(pc);
                let instr_impl = self.impls.get_unchecked(instr.name as u8 as usize);
                (instr_impl)(self, ram, snapshotter);
                if self.state.pc == pc {
                    return;
                }
            }
        }

        panic!("out of cycles");
    }
}
