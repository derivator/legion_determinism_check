# Legion ECS determinism check
[Legion](https://github.com/amethyst/legion) is an
[Entity component system](https://en.wikipedia.org/wiki/Entity_component_system) (ECS) library for Rust.


This repository aims to answer these questions:
* (When) is Legion deterministic?
* Is Legion suitable for deterministic lockstep multiplayer?

## Methodology
To check if Legion introduces non-determinism somewhere, we create a `World` and add some entities with multiple
archetypes. Then we remove some entities, change some components around
and optionally run some other operations on the world so Legion has to do some work.

We then run a query.for_each_mut that mutates the entities in a way that depends on iteration order.
After repeating this whole procedure twice in the exact same way,
we serialize both worlds and check if the outputs match.

## Results

| Test                              | Description                                         |       Result       | Comment                                                                                                 |
|-----------------------------------|-----------------------------------------------------|:------------------:|---------------------------------------------------------------------------------------------------------|
| `serialize_with_default_canon`    | Check if serialization itself is deterministic      |         :x:        | Default Canon uses random UUIDs for entities, have to provide your own stable mapping if desired        |
| `query_for_each_order`            | Iteration order in `query.for_each_mut` stable?     | :heavy_check_mark: | This is used in the next tests to check if iteration order remains deterministic after other operations |
| `order_after_query_par_for_each`  | Order stable after `query.par_for_each_mut`         | :heavy_check_mark: |                                                                                                         |
| `order_after_system_par_for_each` | Order stable after `#[system(par_for_each)]`        | :heavy_check_mark: |                                                                                                         |
| `query_par_for_each_order`        | Iteration order in `query.par_for_each_mut` stable? |     :x: :shrug:    | This can't be deterministic, if you rely on iteration order here it's *your* fault                      |

## Conclusion

I have only tested a few scenarios, but it looks like sequential iteration order in Legion is deterministic.
If you pay attention not to rely on order in parallel iterations,
Legion should be suitable for deterministic lockstep multiplayer.
