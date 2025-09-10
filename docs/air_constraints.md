# AIR-style constraints in this codebase

This document explains how algebraic constraints are represented and enforced in the circuit system used by this repository. It covers the core types, degree rules, normalization, witness generation, invariants, and common construction patterns.

## Core types
Term<F>:   Algebraic Intermediate Representation for a single base-field monomial.
  - Variants:
    - Constant(F): a constant field element.
    - Expression { coeff: F, inner: [Variable; 4], degree: usize }: represents coeff * v0 * … * v{degree-1}.
  - Degrees up to 4 are allowed for Terms during intermediate algebra.

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
- set_values(value_fn): records a closure that computes and assigns witness values for variables. These functions do not add constraints.
- During witness generation, the executor runs the recorded closures to fill variable values.
- You must still add constraints/lookup relations that verify the assigned witnesses satisfy the circuit equations.

## Invariants and compiler layout

Some properties are not enforced immediately but are recorded as invariants and realized during compilation/placement:

Boolean variables: created via add_boolean_variable or helpers that return Boolean::Is.
  - Records Invariant::Boolean.
  - Compiler emits x^2 − x = 0 (i.e., x·(x−1) = 0) for each boolean in the witness subtree.

Range-checked variables: add_variable_with_range_check(width) records Invariant::RangeChecked { width }.
  - Compiler converts these into lookup constraints (8-bit and 16-bit tables supported here).

Substitutions/Linkage: some variables are marked with substitutions or linkages (e.g., public I/O linkage), and the compiler materializes the corresponding constraints at layout time.

## Equality and zero-check gadgets

equals_to(a, b) returns a Boolean using an inverse-or-zero trick:
  - (a − b) · zero_flag = 0
  - (a − b) · inv + zero_flag − 1 = 0
- is_zero(var) uses equals_to(var, 0).
- Variants exist for register tuples when parts are range-checked and sums can be used.

## Selection and masking patterns

- choose(flag, if_true, if_false): builds either quadratic or linear constraints depending on operand constness:
  - new = flag·a + (1 − flag)·b = flag·(a − b) + b, then constrain new and set its witness via value_fn.

- choose_from_orthogonal_variants(flags, variants): sums masked terms under orthogonality and materializes a result variable, with a final ≤ quadratic constraint.

- Masking helpers combine linear terms with booleans and ensure the resulting expressions remain ≤ quadratic.

## Notes

- There is no automatic “quartic to two quadratics” pass. If you compose terms to degree 3 or 4, you must manually introduce auxiliaries to keep the final constraints quadratic.
- Term * Term yields a Constraint but does not normalize immediately. Ensure the resulting Constraint passes through a path that normalizes before storage.
- set_values alone does not ensure correctness. Constraints/lookup relations must verify the witness assignments.
- Not(boolean) is a view. some APIs expect Boolean::Is and will reject Boolean::Not in certain paths.

---

## What is an AIR?

An Algebraic Intermediate Representation (AIR) encodes a computation as polynomial equalities over a finite field F_q. The computation execution trace is laid out in rows and columns. Constraints enforce:

- **Boundary conditions** (initial/final rows)
- **Transition relations** between successive rows 
- **Auxiliary relations** like booleanity, range, lookups, and permutations


