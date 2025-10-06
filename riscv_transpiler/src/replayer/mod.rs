#[derive(Clone, Copy, Debug)]
pub struct ReplayerRam<'a> {
    pub ram_log: &'a [(u32, (u32, u32))],
}

#[derive(Clone, Copy, Debug)]
pub struct ReplayerNonDeterminism<'a> {
    pub non_determinism_reads_log: &'a [u32],
}
