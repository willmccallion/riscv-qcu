#![no_std]
#![feature(allocator_api)]
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

extern crate alloc;

pub mod allocator;
pub mod bit_utils;
pub mod decoder;
pub mod dsu;
pub mod graph;
pub mod hw;
pub mod hw_accel;
pub mod isa;
pub mod pauli_frame;
pub mod ring_buffer;
pub mod spmc;
pub mod static_vec;
pub mod vm;

#[derive(Debug)]
pub enum QecError {
    NodeOutOfBounds,
    DecodingFailed,
    OutOfMemory,
    BufferOverflow,
}
