use super::freelist_vector::{PodFreeListVector, Index, FREE_LIST_NONE};

pub struct IdLookupTable {
    // Dense array
    data_to_index: Vec<Index>,
    // Sparse array
    index_to_data: Vec<Index>,
    // offset of the first empty element in the sparse array
    free_list: Index,
}

impl IdLookupTable {
    pub fn new() -> IdLookupTable {
        IdLookupTable {
            data_to_index: Vec::new(),
            index_to_data: Vec::new(),
            free_list: FREE_LIST_NONE,
        }
    }

    pub fn with_capacity(capacity: uint) -> IdLookupTable {
        IdLookupTable {
            data_to_index: Vec::with_capacity(capacity),
            index_to_data: Vec::with_capacity(capacity),
            free_list: FREE_LIST_NONE,
        }
    }

    pub fn add(&mut self) -> Index {
        if self.free_list == FREE_LIST_NONE {
            self.index_to_data.push(self.data_to_index.len() as Index);
            self.data_to_index.push((self.index_to_data.len()-1) as Index);
            return (self.index_to_data.len()-1) as Index;
        }
        let idx = self.free_list as uint;
        self.free_list = self.index_to_data[idx];
        self.index_to_data[idx] = self.data_to_index.len() as Index;
        self.data_to_index.push(idx as Index);
        return idx as Index;
    }

    pub fn remove(&mut self, idx: Index) {
        let o = self.index_to_data[idx as uint] as uint;
        if o == self.data_to_index.len()-1 {
            self.data_to_index.pop();
        } else {
            let moved = self.data_to_index.pop().unwrap();
            self.index_to_data[moved as uint] = o as Index;
            self.data_to_index[o] = moved;
        }
        self.index_to_data[idx as uint] = self.free_list;
        self.free_list = idx;
    }

    pub fn clear(&mut self) {
        self.free_list = FREE_LIST_NONE;
        self.data_to_index.clear();
        self.index_to_data.clear();
    }

    pub fn lookup(&self, idx: Index) -> Index { self.index_to_data[idx as uint] }

    pub fn index_for_offset(&self, idx: Index) -> Index { self.data_to_index[idx as uint] }

    pub fn len(&self) -> uint { self.data_to_index.len() }

    pub fn reserve(&mut self, size: uint) {
        self.index_to_data.reserve(size);
        self.data_to_index.reserve(size);
    }

    pub fn indices<'l>(&'l self) -> &'l[Index] {
        return self.data_to_index.as_slice();
    }

    pub fn swap_at_indices(&mut self, idx1: Index, idx2: Index) {
        let o1 = self.lookup(idx1);
        let o2 = self.lookup(idx2);
        self.swap_offsets(o1, o2);
    }

    pub fn swap_offsets(&mut self, o1: Index, o2: Index) {
        let temp = self.data_to_index[o1 as uint];
        self.data_to_index[o1 as uint] = self.data_to_index[o2 as uint];
        self.data_to_index[o2 as uint] = temp;
        self.index_to_data[self.data_to_index[o2 as uint] as uint] = o1;
        self.index_to_data[self.data_to_index[o1 as uint] as uint] = o2;
    }
}

mod tests {
    use super::IdLookupTable;
    use super::super::freelist_vector::Index;

    fn check_lookup_table(table: &mut IdLookupTable) {
        assert_eq!(table.len(), 0);

        for i in range(0, 100) {
            table.add();
            assert_eq!(table.lookup(table.index_for_offset(i as Index)), i as Index);
            assert_eq!(table.len(), i+1);
        }

        for i in range(0, table.len()-1) {
            assert_eq!(table.lookup(table.index_for_offset(i as Index)), i as Index);
        }

        table.remove(10);
        table.remove(1);
        table.remove(0);
        table.remove(5);
        table.remove(25);

        for i in range(0, table.len()-1) {
            assert_eq!(table.lookup(table.index_for_offset(i as Index)), i as Index);
        }

        for _ in range(0u32, 10) {
            table.add();
            for i in range(0, table.len()-1) {
                assert_eq!(table.lookup(table.index_for_offset(i as Index)), i as Index);
            }
        }
    }

    #[test]
    fn id_lookup_table() {
        let mut t1 = IdLookupTable::new();
        check_lookup_table(&mut t1);
        t1.clear();
        check_lookup_table(&mut t1);

        let mut t2 = IdLookupTable::with_capacity(30);
        check_lookup_table(&mut t2);
        t2.clear();
        check_lookup_table(&mut t2);
    }
}
