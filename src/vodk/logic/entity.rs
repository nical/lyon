use std::num;

// | Group | Index |
type EntityIndex = u16;
type SystemIndex = u16;
type ComponentIndex = u16;
type FreeList = u16;

#[deriving(Clone)]
pub struct EntityID {
    index: EntityIndex,
}

#[deriving(Clone)]
pub struct ComponentID {
    system: SystemIndex,
    index: ComponentIndex,
}

// TODO proper constant
static ENTITY_NONE: EntityIndex = 9999 as EntityIndex;
static FREE_LIST_NONE: FreeList = 9999 as FreeList;
static SYSTEM_NONE: SystemIndex = 9999 as SystemIndex;
static COMPONENT_NONE: ComponentIndex = 9999 as ComponentIndex;

pub struct Entity {
    components: ~[ComponentID],
    free_list: u16,
}

impl EntityID {
    pub fn null() -> EntityID {
        return EntityID { index: ENTITY_NONE };
    }

    pub fn index(self) -> EntityIndex { self.index }

    pub fn is_null(self) -> bool {
        return self.index == ENTITY_NONE;
    }
}

impl ComponentID {
    pub fn null() -> ComponentID {
        ComponentID { system: SYSTEM_NONE, index: COMPONENT_NONE }
    }
}

pub struct EntityManager {
    entities: ~[Entity],
    free_list: u16,
}

impl EntityManager {
    pub fn new() -> EntityManager {
        EntityManager{ entities: ~[], free_list: FREE_LIST_NONE }
    }

    pub fn add(&mut self, e: Entity) -> EntityID {
        if self.free_list == FREE_LIST_NONE {
            self.entities.push(e);
            return EntityID{index: (self.entities.len()-1) as u16};
        } else {
            let index = self.free_list;
            let next_free_list = self.entities[index].free_list;
            self.entities[self.free_list] = e;
            self.free_list = next_free_list;
            return EntityID { index: index };
        }
    }

    pub fn add_empty(&mut self) -> EntityID {
        return self.add(
            Entity {
                components: ~[],
                free_list: FREE_LIST_NONE,
            }
        );
    }

    pub fn remove(&mut self, id: EntityID) {
        *self.borrow_mut(id) = Entity {
            components: ~[],
            free_list: self.free_list,
        };
        self.free_list = id.index;
    }

    pub fn clear(&mut self) {
        self.free_list = FREE_LIST_NONE;
    }

    pub fn borrow<'l>(&'l self, id: EntityID) -> &'l Entity {
        assert!(self.entities[id.index()].free_list == FREE_LIST_NONE);
        return &'l self.entities[id.index()];
    }

    pub fn borrow_mut<'l>(&'l mut self, id: EntityID) -> &'l mut Entity {
        assert!(self.entities[id.index()].free_list == FREE_LIST_NONE);
        return &'l mut self.entities[id.index()];
    }
}
