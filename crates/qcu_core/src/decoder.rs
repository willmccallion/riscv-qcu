use crate::QecError;
use crate::bit_utils::BitPack;
use crate::dsu::UnionFind;
use crate::graph::DecodingGraph;
use crate::static_vec::StaticVec;
use core::alloc::Allocator;

pub trait CorrectionBuffer {
    fn push_correction(&mut self, u: usize, v: usize) -> Result<(), QecError>;
    fn clear_buffer(&mut self);
}

impl<A: Allocator> CorrectionBuffer for alloc::vec::Vec<(usize, usize), A> {
    fn push_correction(&mut self, u: usize, v: usize) -> Result<(), QecError> {
        self.try_reserve(1).map_err(|_| QecError::OutOfMemory)?;
        self.push((u, v));
        Ok(())
    }
    fn clear_buffer(&mut self) {
        self.clear();
    }
}

impl<const N: usize> CorrectionBuffer for StaticVec<(usize, usize), N> {
    fn push_correction(&mut self, u: usize, v: usize) -> Result<(), QecError> {
        self.push((u, v)).map_err(|_| QecError::BufferOverflow)
    }
    fn clear_buffer(&mut self) {
        self.clear();
    }
}

pub struct UnionFindDecoder<const N: usize>
where
    [(); N.div_ceil(64)]:,
{
    parent: StaticVec<usize, N>,
    rank: StaticVec<u8, N>,
    parity: StaticVec<u64, { N.div_ceil(64) }>,
    touched: StaticVec<usize, N>,
}

impl<const N: usize> Default for UnionFindDecoder<N>
where
    [(); N.div_ceil(64)]:,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> UnionFindDecoder<N>
where
    [(); N.div_ceil(64)]:,
{
    pub fn new() -> Self {
        Self {
            parent: StaticVec::new(),
            rank: StaticVec::new(),
            parity: StaticVec::new(),
            touched: StaticVec::new(),
        }
    }

    pub fn solve_into<GA: Allocator, CB: CorrectionBuffer>(
        &mut self,
        graph: &DecodingGraph<GA>,
        syndrome_indices: &[usize],
        out_buffer: &mut CB,
    ) -> Result<(), QecError> {
        out_buffer.clear_buffer();

        let num_nodes = graph.num_nodes().min(N);

        self.parent.clear();
        self.rank.clear();
        self.touched.clear();
        self.parity.clear();

        for i in 0..num_nodes {
            let _ = self.parent.push(i);
            let _ = self.rank.push(0);
            let _ = self.touched.push(0);
        }

        let num_u64 = num_nodes.div_ceil(64);
        for _ in 0..num_u64 {
            let _ = self.parity.push(0);
        }

        let mut dsu = UnionFind::new(
            self.parent.as_mut_slice(),
            self.rank.as_mut_slice(),
            self.parity.as_mut_slice(),
        );

        for &idx in syndrome_indices {
            if idx < num_nodes {
                dsu.toggle_parity(idx);
                unsafe {
                    *self.touched.get_unchecked_mut(idx) = 1;
                }
            }
        }

        loop {
            let mut changed = false;
            for &(u32_u, u32_v) in &graph.fast_edges {
                let u = u32_u as usize;
                let v = u32_v as usize;

                if unsafe {
                    *self.touched.get_unchecked(u) == 0 && *self.touched.get_unchecked(v) == 0
                } {
                    continue;
                }

                let root_u = dsu.find(u);
                let root_v = dsu.find(v);

                if root_u != root_v {
                    let u_active = BitPack::get(dsu.parity, root_u);
                    let v_active = BitPack::get(dsu.parity, root_v);

                    if (u_active || v_active) && dsu.union(u, v) {
                        out_buffer.push_correction(u, v)?;
                        changed = true;
                        unsafe {
                            *self.touched.get_unchecked_mut(u) = 1;
                            *self.touched.get_unchecked_mut(v) = 1;
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }

        Ok(())
    }
}
