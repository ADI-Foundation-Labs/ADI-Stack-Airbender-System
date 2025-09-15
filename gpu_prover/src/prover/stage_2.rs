use super::arg_utils::LookupChallenges;
use super::callbacks::Callbacks;
use super::context::{HostAllocation, ProverContext};
use super::setup::SetupPrecomputations;
use super::stage_1::StageOneOutput;
pub(crate) use super::stage_2_kernels::*;
use super::trace_holder::{flatten_tree_caps, TraceHolder};
use super::{BF, E4};
use crate::allocator::tracker::AllocationPlacement;
use crate::device_structures::{DeviceMatrix, DeviceMatrixChunk, DeviceMatrixMut};
use crate::ops_simple::set_by_ref;
use blake2s_u32::BLAKE2S_DIGEST_SIZE_U32_WORDS;
use cs::definitions::NUM_LOOKUP_ARGUMENT_LINEARIZATION_CHALLENGES;
use cs::one_row_compiler::CompiledCircuitArtifact;
use era_cudart::memory::memory_copy_async;
use era_cudart::result::CudaResult;
use era_cudart::slice::DeviceSlice;
use field::{Field, FieldExtension};
use prover::definitions::Transcript;
use prover::prover_stages::cached_data::ProverCachedData;
use prover::transcript::Seed;
use std::slice;

pub(crate) struct StageTwoOutput {
    pub(crate) trace_holder: TraceHolder<BF>,
    pub(crate) lookup_challenges: Option<HostAllocation<LookupChallenges>>,
    pub(crate) last_row: Option<HostAllocation<[BF]>>,
    pub(crate) offset_for_grand_product_poly: usize,
    pub(crate) offset_for_sum_over_delegation_poly: Option<usize>,
}

impl StageTwoOutput {
    pub fn allocate_trace_evaluations(
        circuit: &CompiledCircuitArtifact<BF>,
        log_lde_factor: u32,
        log_tree_cap_size: u32,
        recompute_cosets: bool,
        recompute_trees: bool,
        context: &ProverContext,
    ) -> CudaResult<Self> {
        let trace_len = circuit.trace_len;
        assert!(trace_len.is_power_of_two());
        let log_domain_size = trace_len.trailing_zeros();
        let layout = circuit.stage_2_layout;
        let num_stage_2_cols = layout.total_width;
        let trace_holder = TraceHolder::allocate_only_evaluation(
            log_domain_size,
            log_lde_factor,
            0,
            log_tree_cap_size,
            num_stage_2_cols,
            true,
            true,
            recompute_cosets,
            recompute_trees,
            context,
        )?;
        Ok(Self {
            trace_holder,
            lookup_challenges: None,
            last_row: None,
            offset_for_grand_product_poly: 0,
            offset_for_sum_over_delegation_poly: None,
        })
    }

    pub fn generate(
        &mut self,
        seed: &mut HostAllocation<Seed>,
        circuit: &CompiledCircuitArtifact<BF>,
        cached_data: &ProverCachedData,
        setup: &mut SetupPrecomputations,
        stage_1_output: &mut StageOneOutput,
        callbacks: &mut Callbacks,
        context: &ProverContext,
    ) -> CudaResult<()> {
        let trace_len = circuit.trace_len;
        assert!(trace_len.is_power_of_two());
        let log_domain_size = trace_len.trailing_zeros();
        let layout = circuit.stage_2_layout;
        let num_stage_2_cols = layout.total_width;
        let mut lookup_challenges = unsafe { context.alloc_host_uninit::<LookupChallenges>() };
        let stream = context.get_exec_stream();
        let seed_accessor = seed.get_mut_accessor();
        let lookup_challenges_accessor = lookup_challenges.get_mut_accessor();
        let lookup_challenges_fn = move || unsafe {
            let mut transcript_challenges = [0u32;
                ((NUM_LOOKUP_ARGUMENT_LINEARIZATION_CHALLENGES + 1) * 4)
                    .next_multiple_of(BLAKE2S_DIGEST_SIZE_U32_WORDS)];
            Transcript::draw_randomness(seed_accessor.get_mut(), &mut transcript_challenges);
            let mut it = transcript_challenges.as_chunks::<4>().0.iter();
            let mut get_challenge =
                || E4::from_coeffs_in_base(&it.next().unwrap().map(BF::from_nonreduced_u32));
            let challenges = lookup_challenges_accessor.get_mut();
            challenges.linearization_challenges = std::array::from_fn(|_| get_challenge());
            challenges.gamma = get_challenge();
        };
        callbacks.schedule(lookup_challenges_fn, stream)?;
        let num_stage_2_bf_cols = layout.num_base_field_polys();
        let num_stage_2_e4_cols = layout.num_ext4_field_polys();
        assert_eq!(
            num_stage_2_cols,
            4 * (((num_stage_2_bf_cols + 3) / 4) + num_stage_2_e4_cols)
        );
        let setup_evaluations = setup.trace_holder.get_evaluations(context)?;
        let setup_cols = DeviceMatrix::new(&setup_evaluations, trace_len);
        let generic_lookup_mappings = stage_1_output.generic_lookup_mapping.take().unwrap();
        let d_generic_lookups_args_to_table_entries_map =
            DeviceMatrix::new(&generic_lookup_mappings, trace_len);
        let trace_holder = &mut self.trace_holder;
        let evaluations = trace_holder.get_uninit_evaluations_mut();
        let mut d_stage_2_cols = DeviceMatrixMut::new(evaluations, trace_len);
        let num_e4_scratch_elems = get_stage_2_e4_scratch(trace_len, circuit);
        let mut d_alloc_e4_scratch =
            context.alloc(num_e4_scratch_elems, AllocationPlacement::BestFit)?;
        let (cub_scratch_bytes, batch_reduce_intermediate_elems) =
            get_stage_2_cub_and_batch_reduce_intermediate_scratch(
                trace_len,
                num_stage_2_bf_cols,
                cached_data.handle_delegation_requests,
                cached_data.process_delegations,
                context.get_device_properties(),
            )?;
        let mut d_alloc_scratch_for_cub_ops =
            context.alloc(cub_scratch_bytes, AllocationPlacement::BestFit)?;
        let mut maybe_batch_reduce_intermediates_alloc = if batch_reduce_intermediate_elems > 0 {
            let alloc = context.alloc(
                batch_reduce_intermediate_elems,
                AllocationPlacement::BestFit,
            )?;
            Some(alloc)
        } else {
            None
        };
        let mut maybe_batch_reduce_intermediates: Option<&mut DeviceSlice<BF>> =
            if let Some(ref mut d_alloc) = maybe_batch_reduce_intermediates_alloc {
                Some(d_alloc)
            } else {
                None
            };
        let col_sums_scratch_elems = get_stage_2_col_sums_scratch(num_stage_2_bf_cols);
        let mut d_alloc_scratch_for_col_sums =
            context.alloc(col_sums_scratch_elems, AllocationPlacement::BestFit)?;
        let mut d_lookup_challenges =
            context.alloc::<LookupChallenges>(1, AllocationPlacement::BestFit)?;
        memory_copy_async(
            &mut d_lookup_challenges,
            slice::from_ref(unsafe { lookup_challenges_accessor.get() }),
            stream,
        )?;
        self.lookup_challenges = Some(lookup_challenges);
        let witness_evaluations = stage_1_output.witness_holder.get_evaluations(context)?;
        let d_witness_cols = DeviceMatrix::new(&witness_evaluations, trace_len);
        let memory_evaluations = stage_1_output.memory_holder.get_evaluations(context)?;
        let d_memory_cols = DeviceMatrix::new(&memory_evaluations, trace_len);
        compute_stage_2_args_on_main_domain(
            &setup_cols,
            &d_witness_cols,
            &d_memory_cols,
            &d_generic_lookups_args_to_table_entries_map,
            &mut d_stage_2_cols,
            &mut d_alloc_e4_scratch,
            &mut d_alloc_scratch_for_cub_ops,
            &mut maybe_batch_reduce_intermediates,
            &mut d_alloc_scratch_for_col_sums,
            &d_lookup_challenges[0],
            cached_data,
            circuit,
            circuit.total_tables_size,
            log_domain_size,
            stream,
            context.get_device_properties(),
        )?;
        generic_lookup_mappings.free();
        d_alloc_e4_scratch.free();
        d_alloc_scratch_for_cub_ops.free();
        if let Some(allocation) = maybe_batch_reduce_intermediates_alloc {
            allocation.free();
        }
        d_alloc_scratch_for_col_sums.free();
        d_lookup_challenges.free();
        trace_holder.allocate_to_full(context)?;
        trace_holder.extend_and_commit(0, context)?;
        let mut d_last_row = context.alloc(num_stage_2_cols, AllocationPlacement::BestFit)?;
        let evaluations = trace_holder.get_evaluations(context)?;
        let last_row_src = DeviceMatrixChunk::new(evaluations, trace_len, trace_len - 1, 1);
        let mut las_row_dst = DeviceMatrixMut::new(&mut d_last_row, 1);
        set_by_ref(&last_row_src, &mut las_row_dst, stream)?;
        let mut last_row = unsafe { context.alloc_host_uninit_slice(num_stage_2_cols) };
        let last_row_accessor = last_row.get_mut_accessor();
        memory_copy_async(unsafe { last_row_accessor.get_mut() }, &d_last_row, stream)?;
        self.last_row = Some(last_row);
        let offset_for_grand_product_poly = layout.intermediate_poly_for_grand_product.start();
        self.offset_for_grand_product_poly = offset_for_grand_product_poly;
        let offset_for_sum_over_delegation_poly =
            if cached_data.handle_delegation_requests || cached_data.process_delegations {
                Some(cached_data.delegation_processing_aux_poly.start())
            } else {
                None
            };
        self.offset_for_sum_over_delegation_poly = offset_for_sum_over_delegation_poly;
        let has_delegation_processing_aux_poly = circuit
            .stage_2_layout
            .delegation_processing_aux_poly
            .is_some();
        let tree_caps_accessors = trace_holder.get_tree_caps_accessors();
        let update_seed_fn = move || unsafe {
            let mut transcript_input = vec![];
            let last_row = last_row_accessor.get();
            transcript_input.extend(flatten_tree_caps(&tree_caps_accessors));
            transcript_input.extend(
                Self::get_grand_product_accumulator(offset_for_grand_product_poly, last_row)
                    .into_coeffs_in_base()
                    .iter()
                    .map(BF::to_reduced_u32),
            );
            if has_delegation_processing_aux_poly {
                transcript_input.extend(
                    Self::get_sum_over_delegation_poly(
                        offset_for_sum_over_delegation_poly,
                        last_row,
                    )
                    .unwrap_or_default()
                    .into_coeffs_in_base()
                    .iter()
                    .map(BF::to_reduced_u32),
                );
            }
            Transcript::commit_with_seed(seed_accessor.get_mut(), &transcript_input);
        };
        callbacks.schedule(update_seed_fn, stream)?;
        Ok(())
    }

    pub fn get_grand_product_accumulator(offset: usize, last_row: &[BF]) -> E4 {
        E4::from_coeffs_in_base(&last_row[offset..offset + 4])
    }

    pub fn get_sum_over_delegation_poly(offset: Option<usize>, last_row: &[BF]) -> Option<E4> {
        offset.map(|o| {
            let coeffs = &last_row[o..o + 4];
            let mut value = E4::from_coeffs_in_base(coeffs);
            value.negate();
            value
        })
    }
}
