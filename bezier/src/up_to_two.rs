use std::mem;

/// A simple stack allocated container that can have zero, one or two elements.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum UpToTwo<T> {
    None,
    One(T),
    Two(T, T),
}

impl<T> UpToTwo<T> {
    #[inline]
    pub fn len(&self) -> usize {
        match self {
            &UpToTwo::None => 0,
            &UpToTwo::One(_) => 1,
            &UpToTwo::Two(_, _) => 2,
        }
    }

    #[inline]
    pub fn get(&self, idx: usize) -> &T {
        return match (idx, self) {
            (0, &UpToTwo::One(ref val)) => val,
            (0, &UpToTwo::Two(ref val, _)) => val,
            (1, &UpToTwo::Two(_, ref val)) => val,
            _ => {
                panic!("Out of bounds: index: {}, len: {}", idx, self.len());
            }
        }
    }

    #[inline]
    pub fn first(&self) -> Option<&T> {
        match self {
            &UpToTwo::None => None,
            &UpToTwo::One(ref val) => Some(val),
            &UpToTwo::Two(ref val, _) => Some(val),
        }
    }

    #[inline]
    pub fn second(&self) -> Option<&T> {
        match self {
            &UpToTwo::Two(_, ref val) => Some(val),
            _ => None,
        }
    }

    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        let this = mem::replace(self, UpToTwo::None);

        let (first, next) = match this {
            UpToTwo::None => (None, UpToTwo::None),
            UpToTwo::One(v1) => (Some(v1), UpToTwo::None),
            UpToTwo::Two(v1, v2) => (Some(v1), UpToTwo::One(v2)),
        };

        *self = next;

        first
    }
}
