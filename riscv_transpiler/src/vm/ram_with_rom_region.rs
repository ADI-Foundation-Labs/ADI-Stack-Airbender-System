use common_constants::TimestampScalar;

use crate::vm::{Register, RAM};

pub struct RamWithRomRegion<const ROM_BOUND_SECOND_WORD_BITS: usize> {
    backing: Vec<Register>,
}

impl<const ROM_BOUND_SECOND_WORD_BITS: usize> RamWithRomRegion<ROM_BOUND_SECOND_WORD_BITS> {
    pub fn from_rom_content(content: &[u32], total_size_bytes: usize) -> Self {
        assert!(total_size_bytes.is_power_of_two());
        let rom_bytes = 1 << (16 + ROM_BOUND_SECOND_WORD_BITS);
        assert!(total_size_bytes > rom_bytes);
        let num_rom_words = rom_bytes / core::mem::size_of::<u32>();

        assert!(content.len() <= num_rom_words);
        let ram_words = total_size_bytes / core::mem::size_of::<u32>();

        let mut backing = vec![
            Register {
                value: 0,
                timestamp: 0
            };
            ram_words
        ];
        for (dst, src) in backing.iter_mut().zip(content.iter()) {
            dst.value = *src;
        }

        Self { backing }
    }
}

// NOTE: we will not branch and special-case here to model ROM reads as reads from address 0 of 0 value,
// and witness post-processing can track it. Instead we will only track last access for snapshotting purposes

impl<const ROM_BOUND_SECOND_WORD_BITS: usize> RAM for RamWithRomRegion<ROM_BOUND_SECOND_WORD_BITS> {
    #[inline(always)]
    fn peek_word(&self, address: u32) -> u32 {
        debug_assert_eq!(address % 4, 0);
        unsafe {
            let word_idx = (address / 4) as usize;
            debug_assert!(word_idx < self.backing.len());
            let slot = self.backing.get_unchecked(word_idx);
            let value = slot.value;

            value
        }
    }

    #[inline(always)]
    fn mask_read_value_for_witness(&self, _address: u32, _value: &mut u32) {
        // we do not do anything here
    }

    #[inline(always)]
    fn read_word(&mut self, address: u32, timestamp: TimestampScalar) -> (TimestampScalar, u32) {
        debug_assert_eq!(address % 4, 0);
        unsafe {
            let word_idx = (address / 4) as usize;
            debug_assert!(word_idx < self.backing.len());
            let value;
            let read_timestamp;
            if word_idx < 1 << (16 + ROM_BOUND_SECOND_WORD_BITS) / core::mem::size_of::<u32>() {
                // value is from real slot, but we mask the access
                value = self.backing.get_unchecked(word_idx).value;
                // we should use one of 0 slot
                let slot = self.backing.get_unchecked_mut(0);
                read_timestamp = slot.timestamp;
                debug_assert!(read_timestamp < timestamp | 1);
                slot.timestamp = timestamp | 1;
            } else {
                let slot = self.backing.get_unchecked_mut(word_idx);
                value = slot.value;
                read_timestamp = slot.timestamp;
                debug_assert!(read_timestamp < timestamp | 1);
                slot.timestamp = timestamp | 1;
            }

            // NOTE: value here will allow us to replay based on log only,
            // but timestamp will allow us to use it later on for witness gen
            // when such reads would be masked into reading from 0 address

            (read_timestamp, value)
        }
    }

    #[inline(always)]
    fn write_word(
        &mut self,
        address: u32,
        word: u32,
        timestamp: TimestampScalar,
    ) -> (TimestampScalar, u32) {
        debug_assert_eq!(address % 4, 0);
        unsafe {
            let word_idx = (address / 4) as usize;
            debug_assert!(word_idx < self.backing.len());
            if word_idx < 1 << (16 + ROM_BOUND_SECOND_WORD_BITS) / core::mem::size_of::<u32>() {
                panic!();
            }
            let slot = self.backing.get_unchecked_mut(word_idx);
            let old_value = slot.value;
            let read_timestamp = slot.timestamp;
            debug_assert!(read_timestamp < timestamp | 2);
            slot.value = word;
            slot.timestamp = timestamp | 2;

            (read_timestamp, old_value)
        }
    }
}
