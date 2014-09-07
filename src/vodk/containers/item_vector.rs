use super::id::FromIndex;
use super::id::ToIndex;

pub struct ItemVector<T, ID> {
    data: Vec<T>,
    removed_items: Vec<ID>,
}

impl<T, ID: FromIndex+ToIndex+Copy> ItemVector<T, ID> {
    pub fn new() -> ItemVector<T, ID> {
        ItemVector {
            data: Vec::new(),
            removed_items: Vec::new(),
        }
    }

    pub fn get(&self, id: ID) -> &T { return self.data.get(id.to_index()); }

    pub fn get_mut(&mut self, id: ID) -> &mut T { return self.data.get_mut(id.to_index()); }

    pub fn add(&mut self, item: T) -> ID {
        match self.removed_items.pop() {
            Some(id) => {
                *self.data.get_mut(id.to_index()) = item;
                return id;
            }
            None => {
                self.data.push(item);
                return FromIndex::from_index(self.data.len() - 1);
            }
        }
    }

    pub fn remove(&mut self, id: ID) {
        self.removed_items.push(id);
    }

    pub fn len(&self) -> uint { self.data.len() - self.removed_items.len() }

    pub fn clear(&mut self) {
        self.data.clear();
        self.removed_items.clear();
    }
}

