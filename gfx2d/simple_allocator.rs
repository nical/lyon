use range::Range;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct BlockId {
    index: usize,
    gen: u16,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum BlockState {
    Used,
    Unused,
}

struct AllocatorBlock {
    range: Range,
    prev: Option<usize>,
    next: Option<usize>,
    state: BlockState,
    gen: u16,
}

pub struct AllocatorHelper {
    blocks: Vec<AllocatorBlock>,
    available_slots: Vec<usize>,
    first: usize,
    last: usize,
    next_gen: u16,
}

impl AllocatorHelper {
    pub fn new(range: Range, state: BlockState) -> AllocatorHelper {
        AllocatorHelper {
            blocks: vec![AllocatorBlock {
                range: range,
                prev: None,
                next: None,
                state: state,
                gen: 1,
            }],
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
        assert!(self.contains_block_id(id));
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
        assert!(self.contains_block_id(id));
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

    pub fn clear(&mut self) -> BlockId {
        loop {
            if self.first == self.last {
                break;
            }
            let first = self.get_first();
            self.merge_next(first, BlockState::Unused);
        }
        return self.get_first();
    }

    pub fn find_available_block(&self, size: u16) -> Option<BlockId> {
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

    pub fn get_first(&self) -> BlockId {
        BlockId { index: self.first, gen: self.blocks[self.first].gen }
    }

    pub fn get_last(&self) -> BlockId {
        BlockId { index: self.last, gen: self.blocks[self.last].gen }
    }

    pub fn get_block_state(&self, id: BlockId) -> BlockState {
        assert!(self.contains_block_id(id));
        self.blocks[id.index].state
    }

    pub fn set_block_state(&mut self, id: BlockId, state:BlockState) {
        assert!(self.contains_block_id(id));
        self.blocks[id.index].state = state;
    }

    pub fn get_block_range(&self, id: BlockId) -> Range {
        assert!(self.contains_block_id(id));
        self.blocks[id.index].range
    }

    pub fn contains_block_id(&self, id: BlockId) -> bool {
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

    pub fn enclosing_used_range(&self) -> Range {
        let mut it = self.first;
        let mut first;
        loop {
            first = self.blocks[it].range.first;
            if self.blocks[it].state == BlockState::Used {
                break;
            }
            if let Some(idx) = self.blocks[it].next {
                it = idx;
            } else {
                break;
            }
        }
        let mut it = self.last;
        let mut last;
        loop {
            last = self.blocks[it].range.right_most();
            if self.blocks[it].state == BlockState::Used {
                break;
            }
            if let Some(idx) = self.blocks[it].prev {
                it = idx;
            } else {
                break;
            }
        }
        return Range { first: first, count: last - first };
    }

    pub fn blocks<'l>(&'l mut self) -> BlockIterator<'l> {
        return BlockIterator {
            allocator: self,
            current: Some(self.get_first()),
            filter: None,
        };
    }

    pub fn blocks_with_state<'l>(&'l mut self, filter: BlockState) -> BlockIterator<'l> {
        return BlockIterator {
            allocator: self,
            current: Some(self.get_first()),
            filter: Some(filter),
        };
    }
}

pub struct BlockIterator<'l> {
    allocator: &'l AllocatorHelper,
    current: Option<BlockId>,
    filter: Option<BlockState>,
}

impl<'l> Iterator for BlockIterator<'l> {

    type Item = BlockId;

    fn next(&mut self) -> Option<BlockId> {
        loop {
            let current = self.current;
            let mut done = true;
            if let Some(id) = current {
                self.current = self.allocator.get_next(id);
                if let Some(filter) = self.filter {
                    if filter != self.allocator.get_block_state(id) {
                        done = false;
                    }
                }
            }
            if done {
                return current;
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.allocator.blocks.len()))
    }
}

#[test]
fn test_allocator() {
    let mut alloc = AllocatorHelper::new(Range::new(0, 100), BlockState::Unused);
    assert_eq!(alloc.get_first(), alloc.get_last());
    let a0 = alloc.get_first();
    let ids: Vec<BlockId> = FromIterator::from_iter(alloc.blocks());
    assert_eq!(ids, vec![a0]);
    assert!(alloc.contains_block_id(a0));
    assert_eq!(alloc.get_block_state(a0), BlockState::Unused);

    let (a1, b1) = alloc.split(a0, 50, BlockState::Used, BlockState::Unused);
    assert!(!alloc.contains_block_id(a0));
    assert!(a1 != b1);
    assert_eq!(alloc.get_block_state(a1), BlockState::Used);
    assert_eq!(alloc.get_block_state(b1), BlockState::Unused);
    assert_eq!(alloc.get_block_range(a1), Range::new(0, 50));
    assert_eq!(alloc.get_block_range(b1), Range::new(50, 50));
    let ids: Vec<BlockId> = FromIterator::from_iter(alloc.blocks());
    assert_eq!(ids, vec![a1, b1]);
    let ids: Vec<BlockId> = FromIterator::from_iter(alloc.blocks_with_state(BlockState::Used));
    assert_eq!(ids, vec![a1]);
    let ids: Vec<BlockId> = FromIterator::from_iter(alloc.blocks_with_state(BlockState::Unused));
    assert_eq!(ids, vec![b1]);

    let a2 = alloc.merge_next(a1, BlockState::Unused);
    assert!(!alloc.contains_block_id(a1));
    assert!(!alloc.contains_block_id(b1));
    let ids: Vec<BlockId> = FromIterator::from_iter(alloc.blocks());
    assert_eq!(ids, vec![a2]);
    assert_eq!(alloc.get_block_range(a2), Range::new(0, 100));

    alloc.clear();
    alloc.clear();
}
