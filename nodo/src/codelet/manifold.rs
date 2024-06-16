use core::ops::Index;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Manifold {
    vertices: HashMap<VertexId, Vertex>,
}

impl Manifold {
    pub fn new() -> Self {
        Self {
            vertices: HashMap::new(),
        }
    }
}

impl Index<VertexId> for Manifold {
    type Output = Vertex;

    fn index(&self, idx: VertexId) -> &Self::Output {
        self.vertices.get(&idx).unwrap()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VertexId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkerId(u64);

#[derive(Debug, Clone)]
pub struct Vertex {
    pub name: String,
    pub typename: String,
    pub worker: WorkerId,
}
