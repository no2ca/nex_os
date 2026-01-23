use core::fmt::Debug;

use crate::SHELL_ELF;

pub trait Fs {
    type NodeType: Node;
    fn lookup(&self, name: &str) -> Option<Self::NodeType>;
}

pub trait Node {
    fn get_id(&self) -> usize;
    fn prefix(&self) -> &'static [u8];
}

pub struct MemoryFs;

impl Fs for MemoryFs {
    type NodeType = MemoryNode;
    fn lookup(&self, name: &str) -> Option<Self::NodeType> {
        match name {
            "shell" => Some(MemoryNode::new(0, SHELL_ELF)),
            _ => todo!(),
        }
    }
}

#[derive(Debug)]
pub struct MemoryNode {
    id: usize,
    prefix: &'static [u8],
}

impl Node for MemoryNode {
    fn get_id(&self) -> usize {
        self.id
    }

    fn prefix(&self) -> &'static [u8] {
        self.prefix
    }
}

impl MemoryNode {
    fn new(id: usize, prefix: &'static [u8]) -> Self {
        Self { id, prefix }
    }
}
