//! Disjoint Set Union (DSU) data structure with parity tracking.
//!
//! Implements a union-find data structure that maintains disjoint sets of
//! nodes while tracking parity (odd/even count) for each set. Used by the
//! decoder to group syndrome nodes into clusters and determine which clusters
//! require corrections. Supports both software and hardware-accelerated find
//! operations via conditional compilation.

use crate::bit_utils::BitPack;

/// Union-Find data structure with parity tracking for decoder clusters.
///
/// Manages a collection of disjoint sets where each set represents a cluster
/// of syndrome nodes. Tracks whether each set has odd or even parity, which
/// determines whether the cluster requires a correction path to a boundary
/// or another odd-parity cluster. Uses path compression and union-by-rank
/// optimizations for efficient operations.
pub struct UnionFind<'a> {
    /// Parent pointer array for the union-find forest.
    ///
    /// Each element stores the parent node index, with root nodes pointing
    /// to themselves. Modified during find operations to implement path
    /// compression, flattening the tree structure for future lookups.
    pub parent: &'a mut [usize],

    /// Rank array for union-by-rank heuristic.
    ///
    /// Approximates the depth of each tree to guide union operations toward
    /// balanced structures. Prevents worst-case O(n) find performance that
    /// would occur with linear chains.
    pub rank: &'a mut [u8],

    /// Parity bit vector for each set root.
    ///
    /// Packed as u64 words, with each bit indicating odd parity (true) or
    /// even parity (false) for the corresponding set. Updated during union
    /// operations to maintain correct parity when sets are merged.
    pub parity: &'a mut [u64],
}

impl<'a> UnionFind<'a> {
    /// Initializes a new union-find structure from pre-allocated slices.
    ///
    /// Sets all nodes to be their own parent (forming singleton sets), resets
    /// all ranks to zero, and clears all parity bits. The slices must have
    /// matching lengths and remain valid for the lifetime of the UnionFind
    /// instance.
    ///
    /// # Arguments
    ///
    /// * `parent` - Mutable slice for parent pointers
    /// * `rank` - Mutable slice for rank values
    /// * `parity` - Mutable slice for parity bits (u64 words)
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

    /// Finds the root of the set containing node i, with path compression.
    ///
    /// Traverses the parent chain to locate the root, simultaneously updating
    /// parent pointers to point directly to the root (path compression). This
    /// optimization ensures future finds for the same node are nearly O(1).
    /// The parity information is stored at the root, so finding the root is
    /// necessary to access or modify set parity.
    ///
    /// # Arguments
    ///
    /// * `i` - Node index to find the root for
    ///
    /// # Returns
    ///
    /// The root node index of the set containing i.
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

    /// Hardware-accelerated find operation using custom RISC-V instruction.
    ///
    /// Delegates to a custom RISC-V instruction when the hardware accelerator
    /// feature is enabled and the target architecture is riscv64. Falls back
    /// to software implementation otherwise. The hardware accelerator performs
    /// path compression in hardware, reducing latency for find operations in
    /// the critical decoding path.
    ///
    /// # Safety
    ///
    /// `ptr` must be a valid pointer to the parent array, and `node_idx` must
    /// be within the bounds of the array. The caller must ensure the memory
    /// remains valid for the duration of the operation.
    ///
    /// # Arguments
    ///
    /// * `ptr` - Pointer to the parent array base
    /// * `node_idx` - Node index to find the root for
    ///
    /// # Returns
    ///
    /// The root node index of the set containing node_idx.
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
            while *ptr.add(i) != i {
                i = *ptr.add(i);
            }
            root = i;
        }

        root
    }

    /// Merges the sets containing nodes i and j, updating parity.
    ///
    /// If the nodes are in different sets, performs a union operation using
    /// the union-by-rank heuristic to maintain balanced trees. When merging,
    /// the parity of the resulting set is the XOR of the two original parities,
    /// which is implemented by toggling the root parity if the subordinate
    /// set had odd parity. Returns true if a union occurred, false if the
    /// nodes were already in the same set.
    ///
    /// # Arguments
    ///
    /// * `i` - First node index
    /// * `j` - Second node index
    ///
    /// # Returns
    ///
    /// True if the sets were merged, false if they were already united.
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

    /// Sets the parity of the set containing node i to a specific value.
    ///
    /// Finds the root of the set and updates its parity bit. Used to initialize
    /// or reset parity state, typically when starting a new decoding cycle
    /// or when explicitly setting the expected parity for a set.
    ///
    /// # Arguments
    ///
    /// * `i` - Node index whose set's parity should be set
    /// * `is_odd` - True to set odd parity, false for even parity
    pub fn set_parity(&mut self, i: usize, is_odd: bool) {
        let root = self.find(i);
        BitPack::set(self.parity, root, is_odd);
    }

    /// Toggles the parity of the set containing node i.
    ///
    /// Finds the root and flips its parity bit. This is the preferred operation
    /// for updating parity when processing syndrome bits, as it handles both
    /// setting and clearing without branching. Each syndrome bit toggles the
    /// parity of its associated detector node's set.
    ///
    /// # Arguments
    ///
    /// * `i` - Node index whose set's parity should be toggled
    pub fn toggle_parity(&mut self, i: usize) {
        let root = self.find(i);
        BitPack::toggle(self.parity, root);
    }
}
