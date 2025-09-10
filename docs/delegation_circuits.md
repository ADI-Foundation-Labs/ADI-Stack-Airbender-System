### Delegation circuits
### What is a delegation circuit?

Delegation circuits are specialized gadgets executed outside the main risc-v semantics but still inside the same proving system. You can also call it 'precompile circuit'. The program issues a delegation request, the circuit builder materializes a small subcircuit to process that request, together with formal register and memory accesses that integrate into the unified memory/register argument.

**Creation**: `get_delegation_circuit()` builds and compiles a specific delegation circuit (e.g., Blake2 or BigInt), then returns a DelegationProcessorDescription containing its delegation_type (CSR ID), trace_len, num_requests_per_circuit, the circuit’s table_driver (lookup tables), and the compiled circuit artifact. 

**Access specification**: The circuit defines which registers are read/written and which memory words are accessed indirectly via base registers.

**Integration**: All register/memory touches are recorded just like main risc v circuits accesses and enforced in Stage 2/3 alongside the rest of the system.

Currently in our system we have 3 delegation circuits implemented: 

- Blake2 round with extended control — `cs/src/delegation/blake2_round_with_extended_control/mod.rs`
-   Used in: prover recursion commitments and Merkle tree hashing
- Blake2 single round — `cs/src/delegation/blake2_single_round/mod.rs`
-   Used in: experiments/tests. Not enabled by default in setups
- BigInt (u256) ops with control — `cs/src/delegation/bigint_with_control/mod.rs`
-   Used in: ZKsync OS as a BN254 math primitive (256-bit field ops; ADD/SUB/MUL/EQ, carry, memcopy)

### Delegation call 
One delegation request is a small primitive, not a full workflow. A complete high‑level operation (e.g., a full hash or multi‑step u256 flow) typically spans multiple delegation requests.
Blake2: Full hashing over multiple rounds and/or blocks requires multiple requests, typically in a loop over rounds/blocks.
BigInt (u256): A request performs one selected operation (ADD/SUB/MUL_LOW/MUL_HIGH/EQ/MEMCOPY) on a single 256‑bit pair. Larger transformations require multiple requests.

### Blake2 single round
A fast cryptographic hash function built from add/xor/rotate G rounds over 32-bit words. High performance on CPUs and GPUs, so Merkle commitments and recursion stay fast. Hash function is circuit friendly. Operations decompose into simple XOR/bitwise lookups and additions, making it efficient as a delegation circuit. Has compact 256-bit outputs suitable for commitments.
Integration is simple. Deterministic, no trusted setup.
Defined in `cs/src/delegation/blake2_single_round/mod.rs`.

 **Memory ABI**
  - 16 state words [R/W]
  - 16 message words [R]
  - 1 word `round_bitmask` (read), boolean-split into ≤10 bits

**Tables used**: `Xor`, `Xor3`, `Xor4`, `Xor7`, `Xor9`.

If `round_bitmask[0]` is set (first round), overwrite state indices `[8,9,10,11,13,15]` with IV words. Message words are permuted via `SIGMAS`. Output state is written back.

### BigInt (u256) ops with control

Defined in `cs/src/delegation/bigint_with_control/mod.rs`.

 **Registers**
  - **x10**: pointer to U256 `a` (8×32-bit words). Creates 8 indirect [R/W] accesses with alignment 2^5.
  - **x11**: pointer to U256 `b` (8×32-bit words). Creates 8 indirect [R] accesses with alignment 2^5.
  - **x12**: `control_mask` (read), boolean-split into 8 bits: `ADD`, `SUB`, `SUB_AND_NEGATE`, `MUL_LOW`, `MUL_HIGH`, `EQ`, `CARRY`, `MEMCOPY`. Exactly one operation bit must be 1 at a time, `CARRY` is a separate flag that may be set together with `ADD` or `SUB`.

**Tables used**: `U16SplitAsBytes`, `RangeCheck9x9`, `RangeCheck10x10`, `RangeCheck11`, `RangeCheck12`, `RangeCheck13`.

Results are written back to `a` (via writes at `x10`). If the delegation is not executed, ABI mandates zero writes.

### Blake2 round with extended control

Defined in `cs/src/delegation/blake2_round_with_extended_control/mod.rs`.

**Registers**
  - **x10**: pointer to state + extended state. Creates 24 indirect [R/W] accesses with alignment 2^7.
  - **x11**: pointer to message words. Creates 16 indirect [R] accesses with alignment 2^2.
  - **x12**: `round_bitmask` (read), boolean-split into ≤10 bits.
  - **x13**: `control_mask` (read), boolean-split into 3 bits (includes `LAST_ROUND`, `INPUT_IS_RIGHT_NODE`).

**Tables used**: `Xor`, `Xor3`, `Xor4`, `Xor7`, `Xor9` for compact bitwise arithmetic.

#### How it differs from Blake2 single round
Extended control: `x10` state+extended (24 words, [R/W], align 2^7), `x11` message (16 words, [R], align 2^2), `x12` round_bitmask (≤10 bits), `x13` control_mask (3 bits incl. LAST_ROUND, INPUT_IS_RIGHT_NODE).
Single round: memory-only ABI — 16 state [R/W], 16 message [R], 1 round_bitmask. No extended state, no control mask, no register-indirected accesses.


### Why “indirect access”

“Indirect” means memory is accessed via an address held in a register (a pointer) rather than by direct register file read/write. The circuit constructs formal memory queries from `(base pointer register, offset, alignment)` and feeds them into the global memory argument.

- **Direct access**: read/write the architectural register file.
- **Indirect access**: read/write RAM at an address computed from a base register (x10/x11) plus offsets. In code: `create_register_and_indirect_memory_accesses(...)` returns a register access plus a list of indirect reads/writes.


### Multiplicity and padding
Multiplicity 0 rows (padding): All columns that participate in the unified memory/register and lookup arguments are zeroed on rows that exist only for padding. This is enforced during Stage 3. Circuits must, therefore, be satisfiable when presented with all‑zero inputs on those rows.
Blake2 (single round and extended control): These circuits read all inputs (state, message, round/control masks) through the unified memory/register interface. On multiplicity 0 rows, those sources are zero, and all selectors are zero. The Blake constraints are built to be zero‑preserving under zero inputs and zero selectors, so outputs remain zero without needing to multiply every relation by an `execute` flag. BigInt with control: It derives booleans such as equality flags that could otherwise evaluate to 1 on all‑zero inputs. To avoid asserting non‑zero signals on padding rows, it masks such derived flags with the `execute` predicate obtained from `cs.process_delegation_request()` (see `cs/src/delegation/bigint_with_control/mod.rs`).

This uniform handling ensures that delegation circuits agree with padding/zeroing constraints: on rows with multiplicity 0, inputs are zero, derived selectors are zero, and outputs are zero, keeping the circuit satisfiable. 




