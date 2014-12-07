// Similar to tesselation.rs but outputs just triangle vertices rather than vertices + indices.
// This will probably go away.

use range::Range;

#[deriving(Clone, Show, PartialEq)]
pub struct BlockId {
    index: uint,
    gen: u16,
}

pub struct AllocatorBlock {
    range: Range,
    prev: Option<uint>,
    next: Option<uint>,
    state: BlockState,
    gen: u16,
}

#[deriving(Clone, Show, PartialEq)]
pub enum BlockState {
    Used,
    Unused,
}

pub struct AllocatorHelper {
    blocks: Vec<AllocatorBlock>,
    available_slots: Vec<uint>,
    first: uint,
    last: uint,
    next_gen: u16,
}

impl AllocatorHelper {
    pub fn new(range: Range, state: BlockState) -> AllocatorHelper {
        AllocatorHelper {
            blocks: vec!(AllocatorBlock {
                range: range,
                prev: None,
                next: None,
                state: state,
                gen: 1,
            }),
            available_slots: Vec::new(),
            first: 0,
            last: 0,
            next_gen: 2,
        }
    }

    pub fn split(
        &mut self,
        id: BlockId,
        at: u16,
        left_state: BlockState,
        right_state: BlockState
    ) -> (BlockId, BlockId) {
        assert!(self.has_id(id));
        let next = self.blocks[id.index].next;
        let first = self.blocks[id.index].range.first;
        let new_count = self.blocks[id.index].range.count - at;
        let new_index;
        let left_gen = self.get_next_gen();
        let right_gen = self.get_next_gen();

        match self.available_slots.pop() {
            Some(idx) => {
                self.blocks[idx] = AllocatorBlock {
                    range: Range { first: first + at, count: new_count },
                    prev: Some(id.index),
                    next: next,
                    state: right_state,
                    gen: right_gen,
                };
                new_index = idx;
            }
            None => {
                self.blocks.push(AllocatorBlock {
                    range: Range { first: first + at, count: new_count },
                    prev: Some(id.index),
                    next: next,
                    state: right_state,
                    gen: right_gen,
                });
                new_index = self.blocks.len() - 1;
            }
        }
        self.blocks[id.index].next = Some(new_index);
        self.blocks[id.index].range.count = at;
        self.blocks[id.index].state = left_state;
        self.blocks[id.index].gen = left_gen;
        if self.last == id.index { self.last = new_index; }
        return (
            BlockId { index: id.index, gen: left_gen },
            BlockId { index: new_index, gen: right_gen }
        );
    }

    pub fn merge_next(&mut self, id: BlockId, state: BlockState) -> BlockId {
        assert!(self.has_id(id));
        let next = self.blocks[id.index].next;
        let next = next.unwrap();
        let next_next = self.blocks[next].next;
        self.blocks[id.index].next = next_next;
        self.blocks[id.index].range.count += self.blocks[next].range.count;
        self.blocks[id.index].state = state;
        self.blocks[id.index].gen = self.get_next_gen();
        self.blocks[next].gen = 0;
        self.blocks[next].range.count = 0;
        if self.last == next { self.last = id.index; }
        self.available_slots.push(next);
        return BlockId {
            index: id.index,
            gen: self.blocks[id.index].gen
        };
    }

    pub fn find_available_block(&mut self, size: u16) -> Option<BlockId> {
        let mut it = self.first;
        loop {
            if self.blocks[it].state == BlockState::Unused
                && self.blocks[it].range.count >= size {
                return Some(BlockId {
                    index: it,
                    gen: self.blocks[it].gen,
                });
            }

            match self.blocks[it].next {
                Some(next) => { it = next; }
                None => { break; }
            }
        }
        return None;
    }

    pub fn first(&self) -> BlockId {
        BlockId { index: self.first, gen: self.blocks[self.first].gen }
    }

    pub fn last(&self) -> BlockId {
        BlockId { index: self.last, gen: self.blocks[self.last].gen }
    }

    pub fn get_state(&self, id: BlockId) -> BlockState {
        assert!(self.has_id(id));
        self.blocks[id.index].state
    }

    pub fn set_state(&mut self, id: BlockId, state:BlockState) {
        assert!(self.has_id(id));
        self.blocks[id.index].state = state;
    }

    pub fn get_range(&self, id: BlockId) -> Range {
        assert!(self.has_id(id));
        self.blocks[id.index].range
    }

    pub fn has_id(&self, id: BlockId) -> bool {
        id.index < self.blocks.len() && self.blocks[id.index].gen == id.gen
    }

    fn get_next_gen(&mut self) -> u16 {
        self.next_gen += 1;
        // Keep 0 as an always invalid generation
        if self.next_gen == 0 { self.next_gen = 1; }
        return self.next_gen;
    }

    pub fn get_next(&self, id: BlockId) -> Option<BlockId> {
        if let Some(index) = self.blocks[id.index].next {
            return Some(BlockId {
                index: index,
                gen: self.blocks[index].gen
            });
        }
        return None;
    }

    pub fn get_previous(&self, id: BlockId) -> Option<BlockId> {
        if let Some(index) = self.blocks[id.index].prev {
            return Some(BlockId {
                index: index,
                gen: self.blocks[index].gen
            });
        }
        return None;
    }
}

#[test]
fn allocator_test() {
    let mut alloc = AllocatorHelper::new(Range::new(0, 100), BlockState::Unused);
    assert_eq!(alloc.first(), alloc.last());
    let a0 = alloc.first();
    assert!(alloc.has_id(a0));
    assert_eq!(alloc.get_state(a0), BlockState::Unused);
    let (a1,b1) = alloc.split(a0, 50, BlockState::Used, BlockState::Unused);
    assert!(!alloc.has_id(a0));
    assert!(a1 != b1);
    assert_eq!(alloc.get_state(a1), BlockState::Used);
    assert_eq!(alloc.get_state(b1), BlockState::Unused);
    assert_eq!(alloc.get_range(a1), Range::new(0, 50));
    assert_eq!(alloc.get_range(b1), Range::new(50, 50));
    let a2 = alloc.merge_next(a1, BlockState::Unused);
    assert!(!alloc.has_id(a1));
    assert!(!alloc.has_id(b1));
    assert_eq!(alloc.get_range(a2), Range::new(0, 100));
}
