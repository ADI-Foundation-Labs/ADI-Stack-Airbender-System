use super::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct DelegationsCounters {
    pub non_determinism_reads: usize,
    pub blake_calls: usize,
    pub bigint_calls: usize,
    pub keccak_calls: usize,
}

impl Counters for DelegationsCounters {}

#[derive(Clone, Copy, Debug)]
pub struct SimpleSnapshot {
    pub state: State<DelegationsCounters>,
    pub last_zero_address_read_timestamp: TimestampScalar,
    pub non_determinism_reads_start: usize,
    pub memory_reads_start: usize,
    pub memory_reads_end: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct PartialSnapshot {
    pub last_zero_address_read_timestamp: TimestampScalar,
    pub non_determinism_reads_offset: usize,
    pub memory_reads_offset: usize,
}

pub struct SimpleSnapshotter<const ROM_BOUND_SECOND_WORD_BITS: usize> {
    pub current_partial_snapshot: PartialSnapshot,
    pub snapshots: Vec<SimpleSnapshot>,
    pub last_zero_address_read_timestamp: TimestampScalar,
    pub reads_buffer: Vec<(u32, (u32, u32))>,
    pub non_determinism_reads_buffer: Vec<u32>,
}

impl<const ROM_BOUND_SECOND_WORD_BITS: usize> SimpleSnapshotter<ROM_BOUND_SECOND_WORD_BITS> {
    pub fn new_with_cycle_limit(limit: usize, period: usize) -> Self {
        Self {
            current_partial_snapshot: PartialSnapshot {
                last_zero_address_read_timestamp: 0,
                non_determinism_reads_offset: 0,
                memory_reads_offset: 0,
            },
            snapshots: Vec::with_capacity(limit.div_ceil(period)),
            last_zero_address_read_timestamp: 0,
            reads_buffer: Vec::with_capacity(limit),
            non_determinism_reads_buffer: Vec::with_capacity(limit),
        }
    }
}

impl<const ROM_BOUND_SECOND_WORD_BITS: usize> Snapshotter
    for SimpleSnapshotter<ROM_BOUND_SECOND_WORD_BITS>
{
    type Counters = DelegationsCounters;

    #[inline(always)]
    fn take_snapshot(&mut self, state: &State<Self::Counters>) {
        let new_snapshot = SimpleSnapshot {
            state: *state,
            non_determinism_reads_start: self.current_partial_snapshot.non_determinism_reads_offset,
            last_zero_address_read_timestamp: self
                .current_partial_snapshot
                .last_zero_address_read_timestamp,
            memory_reads_start: self.current_partial_snapshot.memory_reads_offset,
            memory_reads_end: self.reads_buffer.len(),
        };
        self.current_partial_snapshot
            .last_zero_address_read_timestamp = self.last_zero_address_read_timestamp;
        self.current_partial_snapshot.non_determinism_reads_offset =
            self.non_determinism_reads_buffer.len();
        self.current_partial_snapshot.memory_reads_offset = self.reads_buffer.len();
        self.snapshots.push(new_snapshot);
    }

    #[inline(always)]
    fn append_non_determinism_read(&mut self, value: u32) {
        unsafe {
            self.non_determinism_reads_buffer
                .push_within_capacity(value)
                .unwrap_unchecked();
        }
    }

    #[inline(always)]
    fn append_memory_read(
        &mut self,
        address: u32,
        read_value: u32,
        read_timestamp: TimestampScalar,
        write_timestamp: TimestampScalar,
    ) {
        if address < (1 << (16 + ROM_BOUND_SECOND_WORD_BITS)) {
            self.last_zero_address_read_timestamp = write_timestamp;
        }
        unsafe {
            self.reads_buffer
                .push_within_capacity((
                    read_value,
                    (read_timestamp as u32, (read_timestamp >> 32) as u32),
                ))
                .unwrap_unchecked();
        }
    }
}
