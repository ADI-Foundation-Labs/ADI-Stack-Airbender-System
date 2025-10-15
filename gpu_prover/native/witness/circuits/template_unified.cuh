#pragma once
#include "../trace_unified.cuh"
#include "../witness_generation.cuh"

using namespace ::airbender::witness::generation;
using namespace ::airbender::witness::trace::unified;

namespace airbender::witness::circuits::NAME {

#include CIRCUIT_INCLUDE(NAME)

KERNEL(NAME, UnifiedTrace)

} // namespace airbender::witness::circuits::NAME