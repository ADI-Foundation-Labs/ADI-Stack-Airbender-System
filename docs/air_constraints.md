# AIR-style constraints in this codebase

This document explains how algebraic constraints are represented and enforced in the circuit system used by this repository. It covers the core types, degree rules, normalization, witness generation, invariants, and common construction patterns.

## Core types
Term<F>:   Algebraic Intermediate Representation for a single base-field monomial.
  - Variants:
    - Constant(F): a constant field element.
    - Expression { coeff: F, inner: [Variable; 4], degree: usize }: represents coeff * v0 * … * v{degree-1}.
  - Degrees up to 4 are allowed for Terms during intermediate algebra.
  - Clarification:
    - In `Expression`, variables are stored in `inner` and multiplied together with `coeff`.
    - `degree` is the actual monomial degree (0–4), not always 4. Terms may be quartic internally for composition, but final constraints must normalize to ≤ quadratic (degree ≤ 2).

Constraint<F>: A sum of Term<F> values. Conceptually represents a polynomial relation that must evaluate to zero (unless rearranged by helper methods that subtract a result variable). Constraints are normalized and must be at most quadratic (degree <=> 2) before being accepted by the circuit.
 
## Degree and normalization

Term * Term can create degree up to 4 at the Term level. This is allowed for composition, but not for final constraints.
- Constraint::normalize():
  - Sorts terms (by degree, then variables),
  - Merges like monomials,
  - Drops zero terms,
  - Asserts final degree <= 2.
- Normalization is applied:
  - After most arithmetic on Constraint (e.g., add/sub/mul with a Term),
  - Before storing constraints via add_constraint/add_constraint_allow_explicit_linear,
  - Before splitting with split_max_quadratic(),
  - After transform helpers like express_variable/substitute_variable.
- If a Constraint still has degree > 2 at normalization time, it panics.

## Witness generation vs constraints 
- create an empty variable (placeholder with index but without assigned witness value).
- set_values(value_fn): records a closure that computes and assigns witness values for variables. These functions do not add constraints. Means the closure is stored and executed later during the witness‑generation phase (before constraints are checked). Use it to fill concrete values for variables that were allocated earlier (placeholders).
- During witness generation, the executor runs the recorded closures to fill variable values.
- You must still add constraints/lookup relations that verify the assigned witnesses satisfy the circuit equations.

## Invariants and compiler layout

Some properties are not enforced immediately but are recorded as invariants and realized during compilation/placement. Concretely:
- At allocation time the builder queues an invariant via `require_invariant(...)`.
  - Booleans: the variable is pushed into an internal `boolean_variables` list.
  - Range checks: a `RangeCheckQuery { variable, width }` is pushed into `rangechecked_expressions`.
- During finalize/layout:
  - Range checks: the compiler converts queued `rangechecked_expressions` into lookups against the 8/16‑bit tables (batched where possible), and appends them to lookup storage.
  - Booleans: the queued `boolean_variables` are laid out into dedicated columns, and the compiled circuit enforces `x^2 − x = 0` for each (one boolean constraint per placed row/column).
In practice, no polynomial is emitted at the call site, we tag the variable/relation now and materialize the corresponding polynomial later while building the prover's execution table.

Boolean variables: created via add_boolean_variable or helpers that return Boolean::Is.
  - Records Invariant::Boolean.
  - Compiler emits x^2 − x = 0 (i.e., x·(x−1) = 0) for each boolean in the witness subtree.

Range-checked variables: add_variable_with_range_check(width) records Invariant::RangeChecked { width }.
  - Compiler converts these into lookup constraints (8-bit and 16-bit tables supported here).

Substitutions/Linkage: some variables are marked with substitutions or linkages (e.g., public I/O linkage), and the compiler materializes (generates and inserts) the corresponding constraints at layout time.

## Equality and zero-check gadgets

- equals_to(a, b) returns a Boolean `zero_flag` (the output flag) using an inverse-or-zero trick:
  - Constraints:
    - (a − b) · zero_flag = 0
    - (a − b) · inv + zero_flag − 1 = 0
- is_zero(var) returns a Boolean and is implemented as equals_to(var, 0).
- Variants exist for register tuples when parts are range-checked and sums can be used.

## Selection and masking patterns

- choose(flag, a, b): selects between `a` and `b` using a Boolean `flag` and materializes a fresh output variable `out`.
  - Definition/constraint: `out − (flag · (a − b) + b) = 0` (equivalently `out = flag·a + (1 − flag)·b`).
  - Degree: linear if one of `a` or `b` is a constant, otherwise quadratic, always ≤ 2.
  - Witnessing: set `out`'s value via `value_fn` to `a` when `flag=1`, else `b`.

- choose_from_orthogonal_variants(flags, variants): sums masked terms under orthogonality and materializes a result variable, with a final ≤ quadratic constraint.

- Masking helpers combine linear terms with booleans and ensure the resulting expressions remain ≤ quadratic.

## Notes

- There is no automatic “quartic to two quadratics” pass. If you compose terms to degree 3 or 4, you must manually introduce auxiliaries to keep the final constraints quadratic.
- Term * Term yields a Constraint but does not normalize immediately. Ensure the resulting Constraint passes through a path that normalizes before storage.
- set_values alone does not ensure correctness. Constraints/lookup relations must verify the witness assignments.
- Not(boolean) is a view. some APIs expect Boolean::Is and will reject Boolean::Not in certain paths.

---

## What is an AIR?

An Algebraic Intermediate Representation (AIR) encodes a computation (the program's transition function over its execution trace) as polynomial equalities over a finite field F_q. The computation execution trace is laid out in rows and columns. Constraints enforce:

- **Boundary conditions** (initial/final rows)
- **Transition relations** between successive rows 
- **Auxiliary relations** like booleanity, range, lookups, and permutations


