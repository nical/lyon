
#[derive(Copy, Show, PartialEq)]
pub struct Range {
    pub first: u16,
    pub count: u16,
}

impl Range {
    pub fn new(first: u16, count: u16) -> Range { Range { first: first, count: count } }

    pub fn contains(&self, other: &Range) -> bool {
        self.first <= other.first && self.first + self.count >= other.first + other.count
    }
    pub fn intersects(&self, other: &Range) -> bool {
        self.first <= other.first + self.count && self.first + self.count >= other.first
    }
    pub fn shrink_left(&mut self, amount: u16) {
        self.count -= amount;
        self.first += amount;
    }
    pub fn shrink_right(&mut self, amount: u16) {
        self.count -= amount;
    }
    pub fn expand_left(&mut self, amount: u16) {
        self.count += amount;
        self.first -= amount;
    }
    pub fn expand_right(&mut self, amount: u16) {
        self.count += amount;
    }
    pub fn is_left_adjacent_to(&self, other: &Range) -> bool {
        self.first + self.count == other.first
    }
    pub fn is_right_adjacent_to(&self, other: &Range) -> bool {
        other.is_left_adjacent_to(self)
    }
    pub fn is_adjacent_to(&self, other: &Range) -> bool {
        self.is_left_adjacent_to(other) || other.is_right_adjacent_to(other)
    }

    pub fn is_left_of(&self, other: &Range) -> bool {
        self.first < other.first
    }

    pub fn right_most(&self) -> u16 {
        self.first + self.count
    }
}

