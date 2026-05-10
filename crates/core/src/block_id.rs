use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

pub type BlockId = u16;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Pod, Zeroable, Debug)]
#[repr(transparent)]
pub struct BlockIdWrapper(pub BlockId);
