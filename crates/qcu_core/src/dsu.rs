use crate::bit_utils::BitPack;

pub struct UnionFind<'a> {
    pub parent: &'a mut [usize],
    pub rank: &'a mut [u8],
    pub parity: &'a mut [u64],
}

impl<'a> UnionFind<'a> {
    pub fn new(parent: &'a mut [usize], rank: &'a mut [u8], parity: &'a mut [u64]) -> Self {
        for i in 0..parent.len() {
            parent[i] = i;
            rank[i] = 0;
        }
        parity.fill(0);
        Self {
            parent,
            rank,
            parity,
        }
    }

    #[inline(always)]
    pub fn find(&mut self, mut i: usize) -> usize {
        while i != self.parent[i] {
            let p = self.parent[i];
            let gp = self.parent[p];
            self.parent[i] = gp;
            i = p;
        }
        i
    }

    /// # Safety
    /// `ptr` must be a valid pointer to the parent array.
    /// `node_idx` must be within the bounds of the array pointed to by `ptr`.
    #[inline(always)]
    pub unsafe fn find_hardware_accelerated(ptr: *mut usize, node_idx: usize) -> usize {
        let root: usize;

        #[cfg(all(target_arch = "riscv64", feature = "hw_accel"))]
        unsafe {
            core::arch::asm!(
                ".insn r 0x0B, 0, 0, {rd}, {rs1}, {rs2}",
                rd = out(root),
                rs1 = in(ptr),
                rs2 = in(node_idx),
                options(nostack)
            );
        }

        #[cfg(not(all(target_arch = "riscv64", feature = "hw_accel")))]
        unsafe {
            let mut i = node_idx;
            // Safety: Caller guarantees ptr is valid for node_idx
            while *ptr.add(i) != i {
                i = *ptr.add(i);
            }
            root = i;
        }

        root
    }

    pub fn union(&mut self, i: usize, j: usize) -> bool {
        #[cfg(feature = "hw_accel")]
        let (root_i, root_j) = unsafe {
            let base = self.parent.as_mut_ptr();
            (
                Self::find_hardware_accelerated(base, i),
                Self::find_hardware_accelerated(base, j),
            )
        };

        #[cfg(not(feature = "hw_accel"))]
        let (root_i, root_j) = (self.find(i), self.find(j));

        if root_i != root_j {
            let p_i = BitPack::get(self.parity, root_i);
            let p_j = BitPack::get(self.parity, root_j);

            if self.rank[root_i] < self.rank[root_j] {
                self.parent[root_i] = root_j;
                if p_i {
                    BitPack::toggle(self.parity, root_j);
                }
            } else {
                self.parent[root_j] = root_i;
                if p_j {
                    BitPack::toggle(self.parity, root_i);
                }
                if self.rank[root_i] == self.rank[root_j] {
                    self.rank[root_i] += 1;
                }
            }
            true
        } else {
            false
        }
    }

    pub fn set_parity(&mut self, i: usize, is_odd: bool) {
        let root = self.find(i);
        BitPack::set(self.parity, root, is_odd);
    }

    pub fn toggle_parity(&mut self, i: usize) {
        let root = self.find(i);
        BitPack::toggle(self.parity, root);
    }
}
