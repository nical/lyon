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

// TODO preper constant
static ENTITY_NONE: EntityIndex = 9999 as EntityIndex;
static FREE_LIST_NONE: FreeList = 9999 as FreeList;
static SYSTEM_NONE: SystemIndex = 9999 as SystemIndex;
static COMPONENT_NONE: ComponentIndex = 9999 as ComponentIndex;

#[deriving(Clone)]
pub struct Transform {
    x: f32, y:f32, z: f32,
    yaw: f32, pitch: f32, roll: f32,
}

impl Transform {
    fn identity() -> Transform {
        Transform {
            x: 0.0, y: 0.0, z: 0.0,
            pitch: 0.0, yaw: 0.0, roll: 0.0,
        }
    }
}

#[deriving(Clone)]
pub struct ComponentID {
    system: SystemIndex,
    index: ComponentIndex,
}

pub struct Entity {
    parent: EntityID,
    first_child: EntityID,
    next_sibbling: EntityID,
    main_components: [ComponentID, ..4],
    transform: Transform,
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
                parent: EntityID::null(),
                first_child: EntityID::null(),
                next_sibbling: EntityID::null(),
                main_components: [
                    ComponentID::null(),
                    ComponentID::null(),
                    ComponentID::null(),
                    ComponentID::null(),
                ],
                transform: Transform::identity(),
                free_list: FREE_LIST_NONE,
            }
        );
    }

    pub fn remove(&mut self, id: EntityID) {
        *self.borrow_mut(id) = Entity {
            parent: EntityID::null(),
            first_child: EntityID::null(),
            next_sibbling: EntityID::null(),
            main_components: [
                ComponentID::null(),
                ComponentID::null(),
                ComponentID::null(),
                ComponentID::null(),
            ],
            transform: Transform::identity(),
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


#[test]
fn test_entity_manager_basic() {
    let mut em = EntityManager::new();
    let mut id : ~[EntityID] = ~[];
    for i in range(0, 10) {
        id.push(em.add_empty());
    }
    for i in range(0, 10) {
        let mut e = em.borrow_mut(id[i]);
        e.main_components[0] = ComponentID { system: 42, index: i as ComponentIndex };
        assert!(id[i].index != ENTITY_NONE);
    }
    for i in range(0, 10) {
        let e = em.borrow(id[i]);
        assert_eq!(e.main_components[0].index, i as ComponentIndex);
    }

    em.remove(id[3]);
    em.remove(id[0]);
    em.remove(id[4]);

    for i in range(0, 10) {
        if i == 0 || i == 3 || i == 4 { continue; }
        let e = em.borrow(id[i]);
        assert_eq!(e.main_components[0].index, i as ComponentIndex);
    }

    id[4] = em.add_empty();
    id[0] = em.add_empty();
    id[3] = em.add_empty();

    for i in range(0, 10) {
        let e = em.borrow(id[i]);
    }
}