#![allow(incomplete_features)]
#![feature(generic_const_exprs)]
#![feature(vec_push_within_capacity)]

// In the first take over the compiler and the corresponding simulator we will first
// preprocess the bytecode into fixed-width format, and then will do very simple and execution loop
// that just dispatches a function pointer

pub mod ir;
pub mod vm;
