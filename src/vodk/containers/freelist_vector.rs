
pub type Index = u32;
pub static FREE_LIST_NONE: Index = 2147483647 as Index;

pub struct PodFreeListVector<T> {
    data: Vec<FreeListVectorSlot<T>>,
    free_list: Index,
}

struct FreeListVectorSlot<T> {
    payload: T,
    free_list: Index,
}

impl<T: Copy> PodFreeListVector<T> {
    pub fn new() -> PodFreeListVector<T> {
        PodFreeListVector {
            data: Vec::new(),
            free_list: FREE_LIST_NONE
        }
    }

    pub fn with_capacity(capacity: uint) -> PodFreeListVector<T> {
        PodFreeListVector {
            data: Vec::with_capacity(capacity),
            free_list: FREE_LIST_NONE
        }
    }

    pub fn add(&mut self, val: T) -> Index {
        if self.free_list == FREE_LIST_NONE {
            self.data.push(FreeListVectorSlot{ payload: val, free_list: FREE_LIST_NONE });
            return (self.data.len()-1) as Index;
        } else {
            let index = self.free_list;
            let next_free_list = self.data[index as uint].free_list;
            self.data[self.free_list as uint].payload = val;
            self.free_list = next_free_list;
            return index;
        }
    }

    pub fn remove(&mut self, idx: Index) {
        self.data[idx as uint].free_list = self.free_list;
        self.free_list = idx;
    }

    pub fn clear(&mut self) {
        self.free_list = FREE_LIST_NONE;
    }

    pub fn borrow<'l>(&'l self, id: Index) -> &'l T {
        assert!(self.data[id as uint].free_list == FREE_LIST_NONE);
        return &self.data[id as uint].payload;
    }

    pub fn borrow_mut<'l>(&'l mut self, idx: Index) -> &'l mut T {
        assert!(self.data[idx as uint].free_list == FREE_LIST_NONE);
        return &mut self.data[idx as uint].payload;
    }
}

