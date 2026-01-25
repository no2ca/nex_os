use core::fmt::Debug;

use crate::SHELL_ELF;

pub trait Fs {
    type NodeType: Node;
    fn lookup(&self, name: &str) -> Option<Self::NodeType>;
}

pub trait Node {
    fn get_id(&self) -> usize;
    fn size(&self) -> usize;
    fn read(&self, buf: &mut [u8]) -> Result<(), ()>;
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

    fn size(&self) -> usize {
        self.prefix.len()
    }

    fn read(&self, buf: &mut [u8]) -> Result<(), ()> {
        if buf.len() < self.prefix.len() {
            return Err(());
        }
        buf[0..self.size()].copy_from_slice(self.prefix);
        Ok(())
    }
}

impl MemoryNode {
    fn new(id: usize, prefix: &'static [u8]) -> Self {
        Self { id, prefix }
    }
}
