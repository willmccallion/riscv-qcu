use crate::QecError;
use alloc::alloc::Global;
use alloc::vec::Vec;
use core::alloc::Allocator;

#[derive(Clone, Copy, Debug)]
pub struct Edge {
    pub target: usize,
    pub weight: f64,
}

pub struct DecodingGraph<A: Allocator = Global> {
    pub fast_edges: Vec<(u32, u32), A>,
    pub num_nodes_capacity: usize,
    pub max_node_id: usize,
}

impl DecodingGraph<Global> {
    pub fn new(capacity: usize) -> Self {
        Self::new_in(capacity, Global)
    }
}

impl<A: Allocator> DecodingGraph<A> {
    pub fn new_in(capacity: usize, alloc: A) -> Self {
        Self {
            fast_edges: Vec::with_capacity_in(capacity * 4, alloc),
            num_nodes_capacity: capacity,
            max_node_id: 0,
        }
    }

    pub fn ensure_size(&mut self, n: usize) {
        if n > self.num_nodes_capacity {
            self.num_nodes_capacity = n;
        }
    }

    pub fn add_edge(&mut self, u: usize, v: usize, _weight: f64) -> Result<(), QecError> {
        let max_idx = if u > v { u } else { v };
        self.ensure_size(max_idx + 1);

        if max_idx >= self.max_node_id {
            self.max_node_id = max_idx + 1;
        }

        self.fast_edges.push((u as u32, v as u32));

        Ok(())
    }

    pub fn num_nodes(&self) -> usize {
        self.max_node_id
    }
}
