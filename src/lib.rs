use legion::serialize::{Canon, EntityName};
use legion::*;

use legion::systems::CommandBuffer;
use legion::world::SubWorld;
use serde::{Deserialize, Serialize};
use serde_json::Value;
//

#[derive(Default)]
pub struct SeqCanon {
    pub canon: Canon,
    count: u64,
}

impl SeqCanon {
    pub fn canonize(&mut self, e: Entity) {
        self.canon.canonize(e, self.next_name()).unwrap();
        self.count += 1;
    }

    fn next_name(&self) -> EntityName {
        let b = self.count.to_be_bytes();
        [
            0, 0, 0, 0, 0, 0, 0, 0, b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]
    }
}

#[derive(Serialize, Deserialize)]
pub struct Counter(u64);

pub fn create_registry() -> Registry<String> {
    let mut registry = Registry::<String>::default();
    registry.register::<Counter>("counter".to_string());
    registry.register::<u64>("u64".to_string());
    registry.register::<f32>("f32".to_string());
    registry.register::<f64>("f64".to_string());
    registry.register::<bool>("bool".to_string());
    registry
}

/// Set up a "worn-in" world that has seen some entity/component additions/deletions
pub fn setup() -> (SeqCanon, World, Resources) {
    let mut canon = SeqCanon::default();
    let mut world = World::default();
    let mut resources = Resources::default();

    let count = 100;
    add_entities(&mut world, &mut canon, count);
    remove_some_entities(&mut world, &mut resources);
    change_some_components(&mut world, &mut resources);

    (canon, world, resources)
}

/// Add some entities with different archetypes to the world using different methods
pub fn add_entities(world: &mut World, canon: &mut SeqCanon, count: u64) {
    for i in 0..count {
        let id = world.push((Counter(i),));
        canon.canonize(id); //to make serde output deterministic
    }

    for i in 0..count {
        let id = world.push((Counter(count + i), true));
        canon.canonize(id);
    }

    for _ in 0..count {
        let id = world.push((0.0_f32, false));
        canon.canonize(id);
    }

    // Add some entities with extend instead of push
    let mut entities = Vec::with_capacity(count as usize);
    for i in 0..count {
        entities.push((Counter(i), i % 2 == 0))
    }
    let ids = world.extend(entities);
    for id in ids.iter() {
        canon.canonize(*id);
    }
}

/// Query all entities and remove every third entity using a CommandBuffer
fn remove_some_entities(world: &mut World, resources: &mut Resources) {
    let mut query = <Entity>::query();
    let mut buf = CommandBuffer::new(world);
    let mut i = 0;
    query.for_each(world, |e| {
        if i % 3 == 0 {
            buf.remove(*e);
        }
        i += 1;
    });
    buf.flush(world, resources);
}

/// Query all entities and add an f64 component on every third entity,
/// remove Counter component from every fifth entity using a CommandBuffer
fn change_some_components(world: &mut World, resources: &mut Resources) {
    let mut query = <Entity>::query();
    let mut buf = CommandBuffer::new(world);
    let mut i = 0;
    query.for_each(world, |e| {
        if i % 3 == 0 {
            buf.add_component(*e, 0.0_f64)
        }
        if i % 5 == 0 {
            buf.remove_component::<Counter>(*e);
        }
        i += 1;
    });
    buf.flush(world, resources);
}

pub fn compare_output(
    test: fn(&SeqCanon, &mut World, &mut Resources),
    registry: &Registry<String>,
) {
    let (canon, mut world, mut resources) = setup();
    test(&canon, &mut world, &mut resources);
    let result = serialize_seq(&canon, &mut world, registry);

    let (canon, mut world, mut resources) = setup();
    test(&canon, &mut world, &mut resources);
    let result2 = serialize_seq(&canon, &mut world, registry);

    assert_ne!(world.len(), 0); // No point in checking equality if we accidentally cleared the world
    assert_eq!(result, result2);
    println!("Success!");
    //println!("Success:\n{:#}", result);
}

pub fn serialize_seq(canon: &SeqCanon, world: &mut World, registry: &Registry<String>) -> Value {
    serde_json::to_value(&world.as_serializable(any(), registry, &canon.canon)).unwrap()
}

// ----------------------------------------------------------------------
// ----------------------------------------------------------------------

#[test] // serialization with a default Canon is not deterministic -> fails
fn serialize_with_default_canon() {
    let registry = &create_registry();
    let (_canon, world, _resources) = setup();
    let s =
        serde_json::to_value(&world.as_serializable(any(), registry, &Canon::default())).unwrap();
    let s2 =
        serde_json::to_value(&world.as_serializable(any(), registry, &Canon::default())).unwrap();

    assert_eq!(s, s2)
}

// ----------------------------------------------------------------------
// ----------------------------------------------------------------------

#[test] // #[system(par_for_each)] without order sensitivity
fn order_after_system_par_for_each() {
    #[system(par_for_each)]
    pub fn s_par_for_each(counter: &mut Counter) {
        counter.0 += 1
    }

    pub fn system_par_for_each_t(_canon: &SeqCanon, world: &mut World, resources: &mut Resources) {
        let mut schedule = Schedule::builder()
            .add_system(s_par_for_each_system())
            .add_system(q_for_each_order_system())
            .build();

        schedule.execute(world, resources);
    }

    let registry = create_registry();
    compare_output(system_par_for_each_t, &registry);
}

// ----------------------------------------------------------------------
// ----------------------------------------------------------------------

#[system]
/// Iteration order dependent query.for_each_mut
/// Also used to check if other systems affect iteration order
pub fn q_for_each_order(world: &mut SubWorld, query: &mut Query<(&mut Counter,)>) {
    let mut i = 0;
    query.for_each_mut(world, |c| {
        c.0 .0 = i;
        i += 1;
    });
}

#[test] // iteration order dependent query.for_each_mut
fn query_for_each_order() {
    pub fn query_for_each_order_t(_canon: &SeqCanon, world: &mut World, resources: &mut Resources) {
        let mut schedule = Schedule::builder()
            .add_system(q_for_each_order_system())
            .build();

        schedule.execute(world, resources);
    }

    let registry = create_registry();
    compare_output(query_for_each_order_t, &registry);
}

// ----------------------------------------------------------------------
// ----------------------------------------------------------------------

#[system]
pub fn q_par_for_each(world: &mut SubWorld, query: &mut Query<(&mut Counter,)>) {
    query.par_for_each_mut(world, |c| {
        c.0 .0 = 0;
    });
}

#[test] // query.par_for_each_mut without order sensitivity
fn order_after_query_par_for_each() {
    fn query_par_for_each_t(_canon: &SeqCanon, world: &mut World, resources: &mut Resources) {
        let mut schedule = Schedule::builder()
            .add_system(q_par_for_each_system())
            .add_system(q_for_each_order_system())
            .build();

        schedule.execute(world, resources);
    }

    let registry = create_registry();
    compare_output(query_par_for_each_t, &registry);
}

// ----------------------------------------------------------------------
// ----------------------------------------------------------------------

#[test] // explicitly iteration order dependent query.for_each_mut, MUST fail
fn query_par_for_each_order() {
    use std::sync::RwLock;

    #[system]
    fn q_par_for_each_order(world: &mut SubWorld, query: &mut Query<(&mut Counter,)>) {
        let i: RwLock<u64> = RwLock::new(0);
        query.par_for_each_mut(world, |c| {
            c.0 .0 = *i.read().unwrap(); // iteration order dependency
            let mut i = i.write().unwrap();
            *i += 1;
        });
    }

    fn q_par_for_each_order_t(_canon: &SeqCanon, world: &mut World, resources: &mut Resources) {
        let mut schedule = Schedule::builder()
            .add_system(q_par_for_each_order_system())
            .build();

        schedule.execute(world, resources);
    }

    let registry = create_registry();
    compare_output(q_par_for_each_order_t, &registry);
}
