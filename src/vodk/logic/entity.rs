

// | Group | Index |
pub struct EntityID(u32);
pub struct EntityGroup(u16);
pub struct EntityIndex(u16);
static ENTITY_GROUP_BITS: u32 = 16;
static ENTITY_INDEX_BITS: u32 = 16;
static ENTITY_GROUP_MASK: u32 = 0xff00 as u32;
static ENTITY_INDEX_MASK: u32 = 0x00ff as u32;

pub struct ComponentID(u16);


pub struct Entity {
    parent: EntityID,
    first_child: EntityID,
    next_sibbling: EntityID,
    main_components: [ComponentID, ..4],
}

impl Entity {
    fn new_empty() -> Entity {
        return Entity {
            parent: EntityID::null(),
            first_child: EntityID::null(),
            next_sibbling: EntityID::null(),
            main_components: [
                ComponentID(0), ComponentID(0), ComponentID(0), ComponentID(0)
            ]
        }
    }
}

impl EntityID {
    pub fn new(group: EntityGroup, index: EntityIndex) -> EntityID {
        EntityID(*index as u32 + *group as u32 << ENTITY_INDEX_BITS)
    }
    pub fn null() -> EntityID {
        return EntityID::new(EntityGroup(0), EntityIndex(0));
    }
    pub fn group(self) -> EntityGroup {
        EntityGroup(((*self & ENTITY_GROUP_MASK) >> ENTITY_INDEX_BITS) as u16)
    }
    pub fn index(self) -> EntityIndex {
        EntityIndex((*self & ENTITY_INDEX_MASK) as u16)
    }
    pub fn is_null(self) -> bool {
        return *self.index() == 0; 
    }
}


pub struct EntityManager {
    groups: ~[Group],
}

struct Group {
    entities: ~[Entity],
}

impl EntityManager {
    fn create(&mut self) -> EntityID {
        if self.groups.len() == 0 {
            self.groups.push(Group{ entities: ~[]});
        }
        self.groups[0].entities.push(Entity::new_empty());
        return EntityID::new(EntityGroup(0),EntityIndex((self.groups.len()-1) as u16));
    }
}