use crate::allocator::BumpAllocator;
use crate::bit_utils::BitPack;
use core::slice;

pub struct PauliFrame {
    pub x_register: *mut u64,
    pub z_register: *mut u64,
    num_u64: usize,
}

impl PauliFrame {
    pub fn new(alloc: &BumpAllocator, num_qubits: usize) -> Self {
        let num_u64 = num_qubits.div_ceil(64);
        let x_reg = alloc.alloc_slice::<u64>(num_u64).unwrap().as_mut_ptr();
        let z_reg = alloc.alloc_slice::<u64>(num_u64).unwrap().as_mut_ptr();

        Self {
            x_register: x_reg,
            z_register: z_reg,
            num_u64,
        }
    }

    pub fn reset(&mut self) {
        unsafe {
            let x_slice = slice::from_raw_parts_mut(self.x_register, self.num_u64);
            let z_slice = slice::from_raw_parts_mut(self.z_register, self.num_u64);
            x_slice.fill(0);
            z_slice.fill(0);
        }
    }

    pub fn apply_hadamard(&mut self, q: usize) {
        unsafe {
            let x_slice = slice::from_raw_parts_mut(self.x_register, self.num_u64);
            let z_slice = slice::from_raw_parts_mut(self.z_register, self.num_u64);

            let has_x = BitPack::get(x_slice, q);
            let has_z = BitPack::get(z_slice, q);

            BitPack::set(x_slice, q, has_z);
            BitPack::set(z_slice, q, has_x);
        }
    }

    pub fn apply_cnot(&mut self, c: usize, t: usize) {
        unsafe {
            let x_slice = slice::from_raw_parts_mut(self.x_register, self.num_u64);
            let z_slice = slice::from_raw_parts_mut(self.z_register, self.num_u64);

            if BitPack::get(x_slice, c) {
                BitPack::toggle(x_slice, t);
            }
            if BitPack::get(z_slice, t) {
                BitPack::toggle(z_slice, c);
            }
        }
    }

    pub fn has_x_error(&self, q: usize) -> bool {
        unsafe {
            let x_slice = slice::from_raw_parts(self.x_register, self.num_u64);
            BitPack::get(x_slice, q)
        }
    }
}
