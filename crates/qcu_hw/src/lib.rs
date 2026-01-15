unsafe extern "C" {
    fn hw_init(data: *const u32, len: usize);
    fn hw_shutdown();
    fn hw_step();
    fn hw_set_input(start: i32, node: i32);
    fn hw_get_root() -> i32;
    fn hw_is_done() -> i32;
}

pub struct UnionFindAccel {
    _marker: std::marker::PhantomData<()>,
}

impl UnionFindAccel {
    pub fn new(parent_array: &[u32]) -> Self {
        unsafe {
            hw_init(parent_array.as_ptr(), parent_array.len());
        }
        Self {
            _marker: std::marker::PhantomData,
        }
    }

    pub fn find_root(&self, node_idx: u32) -> u32 {
        unsafe {
            hw_set_input(1, node_idx as i32);
            hw_step();

            hw_set_input(0, node_idx as i32);

            let mut cycles = 0;
            while hw_is_done() == 0 {
                hw_step();
                cycles += 1;
                if cycles > 2000 {
                    panic!(
                        "Hardware Accelerator Timeout on node {}! Cycles: {}",
                        node_idx, cycles
                    );
                }
            }
            hw_get_root() as u32
        }
    }
}

impl Drop for UnionFindAccel {
    fn drop(&mut self) {
        unsafe {
            hw_shutdown();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    fn sw_find(parent: &[u32], mut i: usize) -> usize {
        while i != parent[i] as usize {
            i = parent[i] as usize;
        }
        i
    }

    #[test]
    fn fuzz_hardware_equivalence() {
        let mut rng = rand::thread_rng();

        for _ in 0..100 {
            let size = 256;
            let mut parent: Vec<u32> = (0..size).collect();

            for _ in 0..size {
                let u = rng.gen_range(0..size) as usize;
                let v = rng.gen_range(0..size) as usize;
                let root_u = sw_find(&parent, u);
                let root_v = sw_find(&parent, v);
                if root_u != root_v {
                    parent[root_u] = root_v as u32;
                }
            }

            let accel = UnionFindAccel::new(&parent);

            for i in 0..size {
                let sw_root = sw_find(&parent, i as usize);
                let hw_root = accel.find_root(i);
                assert_eq!(sw_root as u32, hw_root, "Mismatch at node {}", i);
            }
        }
    }
}
