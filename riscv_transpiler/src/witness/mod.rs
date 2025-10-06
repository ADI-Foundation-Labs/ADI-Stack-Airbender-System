pub mod delegation;
use crate::witness::delegation::DelegationWitness;
use risc_v_simulator::machine_mode_only_unrolled::{
    MemoryOpcodeTracingDataWithTimestamp, NonMemoryOpcodeTracingDataWithTimestamp,
};

pub trait WitnessTracer {
    fn write_non_memory_family_data<const FAMILY: usize>(
        &mut self,
        data: NonMemoryOpcodeTracingDataWithTimestamp,
    );
    fn write_memory_family_data<const FAMILY: usize>(
        &mut self,
        data: MemoryOpcodeTracingDataWithTimestamp,
    );
    fn write_delegation<
        const DELEGATION_TYPE: u16,
        const REG_ACCESSES: usize,
        const INDIRECT_READS: usize,
        const INDIRECT_WRITES: usize,
        const VARIABLE_OFFSETS: usize,
    >(
        &mut self,
        data: DelegationWitness<REG_ACCESSES, INDIRECT_READS, INDIRECT_WRITES, VARIABLE_OFFSETS>,
    );
}

// this is largely an example, but is fine for all CPU purposes

// Holder for destination buffer for one particular delegation type. It may represent only part
// of the destination circuit's capacity
pub struct DelegationDestinationHolder<
    'a,
    const REG_ACCESSES: usize,
    const INDIRECT_READS: usize,
    const INDIRECT_WRITES: usize,
    const VARIABLE_OFFSETS: usize,
> {
    pub buffer: &'a mut [DelegationWitness<
        REG_ACCESSES,
        INDIRECT_READS,
        INDIRECT_WRITES,
        VARIABLE_OFFSETS,
    >],
}

impl<
        'a,
        const REG_ACCESSES: usize,
        const INDIRECT_READS: usize,
        const INDIRECT_WRITES: usize,
        const VARIABLE_OFFSETS: usize,
    > WitnessTracer
    for DelegationDestinationHolder<
        'a,
        REG_ACCESSES,
        INDIRECT_READS,
        INDIRECT_WRITES,
        VARIABLE_OFFSETS,
    >
{
    fn write_non_memory_family_data<const FAMILY: usize>(
        &mut self,
        _data: NonMemoryOpcodeTracingDataWithTimestamp,
    ) {
    }
    fn write_memory_family_data<const FAMILY: usize>(
        &mut self,
        _data: MemoryOpcodeTracingDataWithTimestamp,
    ) {
    }

    #[inline(always)]
    fn write_delegation<
        const DELEGATION_TYPE: u16,
        const REG_ACCESSES_T: usize,
        const INDIRECT_READS_T: usize,
        const INDIRECT_WRITES_T: usize,
        const VARIABLE_OFFSETS_T: usize,
    >(
        &mut self,
        data: DelegationWitness<
            REG_ACCESSES_T,
            INDIRECT_READS_T,
            INDIRECT_WRITES_T,
            VARIABLE_OFFSETS_T,
        >,
    ) {
        debug_assert_eq!(REG_ACCESSES, REG_ACCESSES_T);
        debug_assert_eq!(INDIRECT_READS, INDIRECT_READS_T);
        debug_assert_eq!(INDIRECT_WRITES, INDIRECT_WRITES);
        debug_assert_eq!(VARIABLE_OFFSETS, VARIABLE_OFFSETS_T);
        debug_assert!(self.buffer.len() > 0);
        unsafe {
            self.buffer
                .as_mut_ptr()
                .cast::<DelegationWitness<
                    REG_ACCESSES_T,
                    INDIRECT_READS_T,
                    INDIRECT_WRITES_T,
                    VARIABLE_OFFSETS_T,
                >>()
                .write(data);
            // For some reason truncating the buffer doesn't work - livetime analysis complains
            self.buffer = core::mem::transmute(self.buffer.get_unchecked_mut(1..));
        }
    }
}

// Holder for destination buffer for one particular delegation type. It may represent only part
// of the destination circuit's capacity
pub struct NonMemDestinationHolder<'a, const FAMILY: usize> {
    pub buffer: &'a mut [NonMemoryOpcodeTracingDataWithTimestamp],
}

impl<'a, const FAMILY: usize> WitnessTracer for NonMemDestinationHolder<'a, FAMILY> {
    #[inline(always)]
    fn write_non_memory_family_data<const FAMILY_T: usize>(
        &mut self,
        data: NonMemoryOpcodeTracingDataWithTimestamp,
    ) {
        if FAMILY == FAMILY_T {
            unsafe {
                self.buffer.as_mut_ptr().write(data);
                // For some reason truncating the buffer doesn't work - livetime analysis complains
                self.buffer = core::mem::transmute(self.buffer.get_unchecked_mut(1..));
            }
        } else {
        }
    }
    fn write_memory_family_data<const FAMILY_T: usize>(
        &mut self,
        _data: MemoryOpcodeTracingDataWithTimestamp,
    ) {
    }

    fn write_delegation<
        const DELEGATION_TYPE: u16,
        const REG_ACCESSES_T: usize,
        const INDIRECT_READS_T: usize,
        const INDIRECT_WRITES_T: usize,
        const VARIABLE_OFFSETS_T: usize,
    >(
        &mut self,
        _data: DelegationWitness<
            REG_ACCESSES_T,
            INDIRECT_READS_T,
            INDIRECT_WRITES_T,
            VARIABLE_OFFSETS_T,
        >,
    ) {
    }
}

// Holder for destination buffer for one particular delegation type. It may represent only part
// of the destination circuit's capacity
pub struct MemDestinationHolder<'a, const FAMILY: usize> {
    pub buffer: &'a mut [MemoryOpcodeTracingDataWithTimestamp],
}

impl<'a, const FAMILY: usize> WitnessTracer for MemDestinationHolder<'a, FAMILY> {
    fn write_non_memory_family_data<const FAMILY_T: usize>(
        &mut self,
        _data: NonMemoryOpcodeTracingDataWithTimestamp,
    ) {
    }

    #[inline(always)]
    fn write_memory_family_data<const FAMILY_T: usize>(
        &mut self,
        data: MemoryOpcodeTracingDataWithTimestamp,
    ) {
        if FAMILY == FAMILY_T {
            unsafe {
                self.buffer.as_mut_ptr().write(data);
                // For some reason truncating the buffer doesn't work - livetime analysis complains
                self.buffer = core::mem::transmute(self.buffer.get_unchecked_mut(1..));
            }
        } else {
        }
    }

    fn write_delegation<
        const DELEGATION_TYPE: u16,
        const REG_ACCESSES_T: usize,
        const INDIRECT_READS_T: usize,
        const INDIRECT_WRITES_T: usize,
        const VARIABLE_OFFSETS_T: usize,
    >(
        &mut self,
        _data: DelegationWitness<
            REG_ACCESSES_T,
            INDIRECT_READS_T,
            INDIRECT_WRITES_T,
            VARIABLE_OFFSETS_T,
        >,
    ) {
    }
}
