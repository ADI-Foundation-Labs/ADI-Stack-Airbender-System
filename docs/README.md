# Documentation

This directory contains documentation and descriptions of the logic and design behind this repository:

## Getting Started

- [**Overview**](./overview.md) - High-level introduction to ZKsync Airbender and its role in the ZKsync ecosystem
- [**Writing Programs**](./writing_programs.md) - How to write, build, and run RISC-V programs for Airbender
- [**End-to-End Guide**](./end_to_end.md) - Complete workflow from binary to SNARK proof generation
- [**Tutorial**](./tutorial.md) - Step-by-step tutorial for getting started with the prover

## Architecture & Design

- [**Philosophy and Logic**](./philosophy_and_logic.md) - Core design principles and architectural decisions
- [**Circuit Overview**](./circuit_overview.md) - High-level view of the constraint system architecture
- [**Machine Configuration**](./machine_configuration.md) - Different RISC-V machine configurations and their use cases
- [**Delegation Circuits**](./delegation_circuits.md) - Precompiles and specialized circuits for cryptographic operations

## Technical Details

- [**Philosophy and Logic**](./philosophy_and_logic.md) - Core technical architecture, field arithmetic, memory management, and proving pipeline
- [**AIR-style Constraints**](./air_constraints.md) - Algebraic Intermediate Representation constraints used in the codebase
- [**Instruction Gadgets**](./instruction_gadgets.md) - RISC-V instruction implementations and constraint gadgets
- [**Circuit Entry Points**](./circuit_entry_points.md) - How circuits are structured and connected
- [**Custom Assumptions**](./custom_assumptions.md) - Security assumptions and trusted setup requirements
- [**Subarguments Used**](./subarguments_used.md) - Mathematical subarguments and proof techniques

## Implementation

- [**Repository Layout**](./repo_layout.md) - Code organization and directory structure
- [**RISC-V Simulator**](./simulator_supported_instructions.md) - Supported instructions and execution environment
- [**GPU Acceleration**](./gpu.md) - GPU-accelerated proving and performance optimization
- [**Test Suites**](./tests_overview.md) - Testing framework for circuits and proof system

## Diagrams

- [Circuit layout](./charts/circuit_layout.png)
- [Proving flow](./charts/Proving.png)
- [Setup creation](./charts/setup_creation.png)