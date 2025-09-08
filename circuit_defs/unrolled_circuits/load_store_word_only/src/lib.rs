#![feature(allocator_api)]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

use std::alloc::Allocator;

use common_constants::circuit_families::LOAD_STORE_WORD_ONLY_CIRCUIT_FAMILY_IDX;
use prover::cs::cs::circuit::Circuit;
use prover::cs::cs::oracle::ExecutorFamilyDecoderData;
use prover::cs::machine::ops::unrolled::DecoderTableEntry;
use prover::cs::machine::ops::unrolled::{
    compile_unrolled_circuit_state_transition, MemoryFamilyDecoder,
};
use prover::cs::*;
use prover::field::Mersenne31Field;
use prover::tracers::unrolled::tracer::MemTracingFamilyChunk;
use prover::*;

pub const FAMILY_IDX: u8 = LOAD_STORE_WORD_ONLY_CIRCUIT_FAMILY_IDX;
pub const TRACE_LEN_LOG2: u32 = 24;
pub const DOMAIN_SIZE: usize = 1 << TRACE_LEN_LOG2;
pub const NUM_CYCLES: usize = DOMAIN_SIZE - 1;
pub const LDE_FACTOR: usize = 2;
pub const LDE_SOURCE_COSETS: &[usize] = &[0, 1];
pub const TREE_CAP_SIZE: usize = 32;
pub const MAX_ROM_SIZE: usize = 1 << 21; // bytes
pub const ROM_ADDRESS_SPACE_SECOND_WORD_BITS: usize = (MAX_ROM_SIZE.trailing_zeros() - 16) as usize;

fn serialize_to_file<T: serde::Serialize>(el: &T, filename: &str) {
    let mut dst = std::fs::File::create(filename).unwrap();
    serde_json::to_writer_pretty(&mut dst, el).unwrap();
}

pub fn get_circuit(
    bytecode: &[u32],
    delegation_csrs: &[u32],
) -> one_row_compiler::CompiledCircuitArtifact<field::Mersenne31Field> {
    get_circuit_for_rom_bound::<ROM_ADDRESS_SPACE_SECOND_WORD_BITS>(bytecode, delegation_csrs)
}

pub fn get_circuit_for_rom_bound<const ROM_ADDRESS_SPACE_SECOND_WORD_BITS: usize>(
    bytecode: &[u32],
    delegation_csrs: &[u32],
) -> one_row_compiler::CompiledCircuitArtifact<field::Mersenne31Field> {
    let num_bytecode_words = (1 << (16 + ROM_ADDRESS_SPACE_SECOND_WORD_BITS)) / 4;
    assert!(bytecode.len() <= num_bytecode_words);
    assert!(delegation_csrs.is_empty());
    use prover::cs::machine::ops::unrolled::load_store_word_only::*;

    compile_unrolled_circuit_state_transition(
        &|cs| {
            word_only_load_store_table_addition_fn(cs);

            let extra_tables = create_word_only_load_store_special_tables::<
                _,
                ROM_ADDRESS_SPACE_SECOND_WORD_BITS,
            >(bytecode);
            for (table_type, table) in extra_tables {
                cs.add_table_with_content(table_type, table);
            }
        },
        &|cs| {
            word_only_load_store_circuit_with_preprocessed_bytecode::<
                _,
                _,
                ROM_ADDRESS_SPACE_SECOND_WORD_BITS,
            >(cs)
        },
        num_bytecode_words,
        TRACE_LEN_LOG2 as usize,
    )
}

pub fn dump_ssa_form(
    bytecode: &[u32],
    delegation_csrs: &[u32],
) -> Vec<Vec<prover::cs::cs::witness_placer::graph_description::RawExpression<Mersenne31Field>>> {
    dump_ssa_form_for_rom_bound::<ROM_ADDRESS_SPACE_SECOND_WORD_BITS>(bytecode, delegation_csrs)
}

pub fn dump_ssa_form_for_rom_bound<const ROM_ADDRESS_SPACE_SECOND_WORD_BITS: usize>(
    bytecode: &[u32],
    delegation_csrs: &[u32],
) -> Vec<Vec<prover::cs::cs::witness_placer::graph_description::RawExpression<Mersenne31Field>>> {
    let num_bytecode_words = (1 << (16 + ROM_ADDRESS_SPACE_SECOND_WORD_BITS)) / 4;
    assert!(bytecode.len() <= num_bytecode_words);
    assert!(delegation_csrs.is_empty());
    use crate::machine::ops::unrolled::dump_ssa_witness_eval_form_for_unrolled_circuit;
    use crate::machine::ops::unrolled::load_store_word_only::create_word_only_load_store_special_tables;
    use prover::cs::machine::ops::unrolled::load_store_word_only::*;

    dump_ssa_witness_eval_form_for_unrolled_circuit::<Mersenne31Field>(
        &|cs| {
            word_only_load_store_table_addition_fn(cs);

            let extra_tables = create_word_only_load_store_special_tables::<
                _,
                ROM_ADDRESS_SPACE_SECOND_WORD_BITS,
            >(bytecode);
            for (table_type, table) in extra_tables {
                cs.add_table_with_content(table_type, table);
            }
        },
        &|cs| {
            word_only_load_store_circuit_with_preprocessed_bytecode::<
                _,
                _,
                ROM_ADDRESS_SPACE_SECOND_WORD_BITS,
            >(cs)
        },
    )
}

pub fn get_table_driver(
    bytecode: &[u32],
    delegation_csrs: &[u32],
) -> prover::cs::tables::TableDriver<Mersenne31Field> {
    get_table_driver_for_rom_bound::<ROM_ADDRESS_SPACE_SECOND_WORD_BITS>(bytecode, delegation_csrs)
}

pub fn get_table_driver_for_rom_bound<const ROM_ADDRESS_SPACE_SECOND_WORD_BITS: usize>(
    bytecode: &[u32],
    delegation_csrs: &[u32],
) -> prover::cs::tables::TableDriver<Mersenne31Field> {
    use crate::tables::TableDriver;
    use prover::cs::machine::ops::unrolled::load_store_word_only::*;

    let num_bytecode_words = (1 << (16 + ROM_ADDRESS_SPACE_SECOND_WORD_BITS)) / 4;
    assert!(bytecode.len() <= num_bytecode_words);
    assert!(delegation_csrs.is_empty());

    let mut table_driver = TableDriver::<Mersenne31Field>::new();
    word_only_load_store_table_driver_fn(&mut table_driver);

    let extra_tables = create_word_only_load_store_special_tables::<
        _,
        ROM_ADDRESS_SPACE_SECOND_WORD_BITS,
    >(bytecode);
    for (table_type, table) in extra_tables {
        table_driver.add_table_with_content(table_type, table);
    }

    table_driver
}

pub fn get_tracer_factory<A: Allocator>(
    allocator: A,
) -> (
    u8,
    Box<dyn Fn() -> MemTracingFamilyChunk + Send + Sync + 'static, A>,
) {
    use prover::tracers::unrolled::tracer::MemTracingFamilyChunk;
    let factory = Box::new_in(
        || MemTracingFamilyChunk::new_for_num_cycles(DOMAIN_SIZE - 1),
        allocator,
    );

    (FAMILY_IDX, factory as _)
}

pub fn get_decoder_table<const ROM_ADDRESS_SPACE_SECOND_WORD_BITS: usize>(
    bytecode: &[u32],
    delegation_csrs: &[u16],
) -> (
    Vec<Option<DecoderTableEntry<Mersenne31Field>>>,
    Vec<ExecutorFamilyDecoderData>,
) {
    let num_bytecode_words = (1 << (16 + ROM_ADDRESS_SPACE_SECOND_WORD_BITS)) / 4;
    assert!(bytecode.len() <= num_bytecode_words);
    assert!(delegation_csrs.is_empty());

    use crate::machine::ops::unrolled::process_binary_into_separate_tables_ext;
    let mut t = process_binary_into_separate_tables_ext::<Mersenne31Field, true>(
        bytecode,
        &[Box::new(MemoryFamilyDecoder)],
        num_bytecode_words,
        delegation_csrs,
    );

    t.remove(&common_constants::circuit_families::LOAD_STORE_CIRCUIT_FAMILY_IDX)
        .expect("decoder data")
}

#[cfg(feature = "witness_eval_fn")]
mod sealed {
    use crate::field::Mersenne31Field;
    use prover::cs::cs::witness_placer::scalar_witness_type_set::ScalarWitnessTypeSet;
    use prover::cs::cs::witness_placer::WitnessTypeSet;
    use prover::cs::cs::witness_placer::{
        WitnessComputationCore, WitnessComputationalField, WitnessComputationalI32,
        WitnessComputationalInteger, WitnessComputationalU16, WitnessComputationalU32,
        WitnessComputationalU8, WitnessMask,
    };
    use prover::unrolled::MemoryCircuitOracle;
    use prover::witness_proxy::WitnessProxy;
    use prover::SimpleWitnessProxy;

    include!("../generated/witness_generation_fn.rs");

    pub fn witness_eval_fn<'a, 'b>(proxy: &'_ mut SimpleWitnessProxy<'a, MemoryCircuitOracle<'b>>) {
        let fn_ptr = evaluate_witness_fn::<
            ScalarWitnessTypeSet<Mersenne31Field, true>,
            SimpleWitnessProxy<'a, MemoryCircuitOracle<'b>>,
        >;
        (fn_ptr)(proxy);
    }
}

#[cfg(feature = "witness_eval_fn")]
pub fn witness_eval_fn_for_gpu_tracer<'a, 'b>(
    proxy: &'_ mut SimpleWitnessProxy<'a, prover::unrolled::MemoryCircuitOracle<'b>>,
) {
    self::sealed::witness_eval_fn(proxy)
}

/// This function will generate layout and quotient files for verifier
pub fn generate_artifacts() {
    use std::io::Write;

    // particular bytecode doesn't matter here - it only goes to special lookup tables in setup
    let compiled_machine =
        get_circuit_for_rom_bound::<ROM_ADDRESS_SPACE_SECOND_WORD_BITS>(&[], &[]);
    serialize_to_file(&compiled_machine, "generated/layout.json");

    let (layout, quotient) = verifier_generator::generate_for_description(compiled_machine);

    let mut dst = std::fs::File::create("generated/circuit_layout.rs").unwrap();
    dst.write_all(&layout.as_bytes()).unwrap();

    let mut dst = std::fs::File::create("generated/quotient.rs").unwrap();
    dst.write_all(&quotient.as_bytes()).unwrap();
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn generate() {
        generate_artifacts();
    }
}
