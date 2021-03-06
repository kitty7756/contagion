use rand::distributions::*;

use crate::core::geo::circle::*;
use crate::core::geo::intersect::segment_circle::*;
use crate::core::geo::polygon::*;
use crate::core::geo::segment2::*;

use crate::simulation::ai::pathfinding::find_path;
use crate::simulation::state::MoveMode;

use super::state::*;

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub enum Sound {
    Gunshot,
    Reload,
    PersonInfected,
    ZombieDeath,
}

pub struct UpdateArgs {
    pub dt: Scalar
}

pub fn update(args: &UpdateArgs, state: &mut State) -> Vec<Sound> {

    let mut sounds = vec!();

    const DOUBLE_ENTITY_RADIUS_SQUARED: f64 = 4.0 * ENTITY_RADIUS * ENTITY_RADIUS;

    // Check for collisions
    for i in 0..state.entities.len() {
        let p1 = state.entities[i].position;
        let circle = Circle { center: p1, radius: ENTITY_RADIUS };

        if state.entities[i].behaviour == Behaviour::Dead {
            continue;
        }

        // Collisions with other entities
        for j in (i + 1)..state.entities.len() {

            // Do not collide with dead entities
            if state.entities[j].behaviour == Behaviour::Dead {
                continue;
            }

            let p2 = state.entities[j].position;

            let delta = p2 - p1;

            let delta_length_squared = delta.length_squared();

            if delta_length_squared < DOUBLE_ENTITY_RADIUS_SQUARED {
                handle_collision(args, &mut state.entities, i, j, &delta, delta_length_squared, &mut sounds);
            }
        }

        // Collisions with buildings
        for j in 0..state.buildings.len() {
            // Check if position is inside the building
            let mut overlap = state.buildings[j].contains_point(p1);
            let inside = overlap;

            // Don't bother doing this if we already know there's an overlap
            if !inside {
                // Check if one of the building's sides intersects the entity
                for k in 0..state.buildings[j].num_sides() {
                    let segment = Segment2 {
                        p1: state.buildings[j].get(k),
                        p2: state.buildings[j].get((k + 1) % state.buildings[j].num_sides())
                    };

                    if segment_circle_has_intersection(&segment, &circle) {
                        overlap = true;
                        break;
                    }
                }
            }

            if overlap {
                handle_building_collision(args, &mut state.entities[i], &state.buildings[j], inside);
                break;
            }
        }
    }

    // Apply individual behaviours
    for i in 0..state.entities.len() {
        match &state.entities[i].behaviour {
            Behaviour::Cop { .. } =>
                update_cop(&args, state, i, &mut sounds),
            Behaviour::Dead =>
            // Do nothing
                (),
            Behaviour::Human =>
            // Run from zombies!
                simulate_human(args, &mut state.entities, &state.buildings, i),
            b @ Behaviour::Zombie { .. } => {
                // Chase humans and cops!
//                simulate_zombie(args, state, i)
                let behaviour = update_zombie(&args, state, i, b.clone());
                state.entities[i].behaviour = behaviour;
            }
        }
    }

    // Apply acceleration
    for e in &mut state.entities {
        let displacement = args.dt * e.velocity;
        e.position += displacement;
        e.velocity -= ENTITY_DRAG * displacement;
    }

    // Remove motionless bullets
    state.projectiles.retain(
        |p| p.kind != ProjectileKind::Bullet ||
            p.velocity.length_squared() > BULLET_SPEED_MIN
    );

    // Update projectiles
    for p in &mut state.projectiles {

        let displacement = args.dt * p.velocity;
        p.velocity -= PROJECTILE_DRAG * displacement;

        let mut segment = Segment2 { p1: p.position, p2: p.position + displacement };

        p.position = segment.p2;

        if p.kind != ProjectileKind::Bullet {
            continue;
        }

        let mut first_intersect_time_and_index = None;
        for i in 0..state.entities.len() {
            let entity = &state.entities[i];

            if entity.behaviour == Behaviour::Dead {
                // Dead entities don't collide with bullets
                continue;
            }

            let circle = Circle { center: entity.position, radius: ENTITY_RADIUS };

            let this_min_intersection = segment_circle_min_positive_intersect_time(&segment, &circle);

            match (first_intersect_time_and_index, this_min_intersection) {
                (None, Some(this)) => {
                    first_intersect_time_and_index = Some((this, i))
                }
                (Some((min, _)), Some(this)) if this < min => {
                    first_intersect_time_and_index = Some((this, i))
                }
                _ => ()
            }
        }

        match first_intersect_time_and_index {
            Some((time, _)) => {
                segment.p2 = segment.p1 + time * (segment.p2 - segment.p1);
            }
            None => ()
        }

        if !can_see(&state.buildings, segment.p1, segment.p2) {
            first_intersect_time_and_index = None;
            p.velocity = Vector2::zero();
        }

        match first_intersect_time_and_index {
            None => (),
            Some((_, i)) => {
                state.entities[i].behaviour = Behaviour::Dead;
                p.velocity = Vector2::zero();
                sounds.push(Sound::ZombieDeath);
            }
        }
    }

    sounds
}

fn handle_collision(
    args: &UpdateArgs,
    entities: &mut Vec<Entity>,
    i: usize,
    j: usize,
    delta: &Vector2,
    delta_length_squared: f64,
    sounds: &mut Vec<Sound>) {

    // Spread the infection from zombies to others
    match (&entities[i].behaviour, &entities[j].behaviour) {
        (Behaviour::Human, Behaviour::Zombie { .. }) | (Behaviour::Cop { .. }, Behaviour::Zombie { .. }) => {
            entities[i].behaviour = Behaviour::Zombie { state: ZombieState::Roaming };
            sounds.push(Sound::PersonInfected);
        }
        (Behaviour::Zombie { .. }, Behaviour::Human) | (Behaviour::Zombie { .. }, Behaviour::Cop { .. }) => {
            entities[j].behaviour = Behaviour::Zombie { state: ZombieState::Roaming };
            sounds.push(Sound::PersonInfected);
        }
        _ => ()
    }

    // Force entities apart that are overlapping
    let velocity_change = *delta * (args.dt / delta_length_squared);
    entities[i].velocity -= velocity_change;
    entities[j].velocity += velocity_change;
}

fn handle_building_collision(
    args: &UpdateArgs,
    entity: &mut Entity,
    building: &Polygon,
    inside: bool) {

    let mut distance_squared = INFINITY;
    let mut normal = Vector2::zero();
    let normals = building.normals();

    // Find the closest edge
    for i in 0..building.num_sides() {
        let seg_i = Segment2 {
            p1: building.get(i),
            p2: building.get((i + 1) % building.num_sides())
        };
        let dist_i = seg_i.dist_squared(entity.position);

        if distance_squared > dist_i {
            distance_squared = dist_i;
            normal = normals[i];
        }
    }

    const SPRING_CONSTANT: f64 = 32.0;

    let distance = distance_squared.sqrt();

    if inside {
        // If the entity is inside move them to the nearest edge
        entity.position += (distance + ENTITY_RADIUS) * normal;
    } else {
        // If the entity is overlapping, force them away from the edge
        let overlap = ENTITY_RADIUS - distance;
        entity.velocity += args.dt * SPRING_CONSTANT * overlap * normal
    }
}

fn can_see(
    buildings: &Vec<Polygon>,
    from: Vector2,
    to: Vector2) -> bool {

    for building in buildings {
        let num_intersects = building.num_intersects(from, to);
        if num_intersects > 0 {
            return false;
        }
    };
    return true;
}

fn update_cop(
    args: &UpdateArgs,
    sim_state: &mut State,
    index: usize,
    sounds: &mut Vec<Sound>){

    let entities = &mut sim_state.entities;
    let buildings = &sim_state.buildings;
    let building_outlines = &sim_state.building_outlines;

    enum StateChange {
        // Exit the state you're in
        Exit,
        // Continue in the current state unchanged
        Continue,
        // Update the state you're in
        Update(CopState),
        // Enter a new state
        Enter(CopState),
    }

    // Require unsafe block to modify the entity while reading from other entities
    unsafe {
    let entity = &mut entities[index] as *mut Entity;
    match &mut (*entity).behaviour {
        Behaviour::Cop { rounds_in_magazine, state_stack } => {
            let state_change = match state_stack.last() {
                Some(CopState::AttackingZombie { target_index, path: _ }) => {

                    if let Behaviour::Dead = entities[*target_index].behaviour {
                        // Target is dead, stop attacking
                        StateChange::Exit
                    }
                    else if *rounds_in_magazine <= 0 {
                        // Out of ammo, need to reload before we can attack
                        StateChange::Enter(
                            CopState::Reloading {
                                reload_time_remaining: COP_RELOAD_COOLDOWN
                            }
                        )
                    }
                    else if can_see(
                        &sim_state.buildings,
                        (*entity).position,
                        entities[*target_index].position) {
                        // Can see the target, take aim
                        StateChange::Enter(
                            CopState::Aiming {
                                aim_time_remaining: COP_RELOAD_COOLDOWN,
                                target_index: *target_index
                            }
                        )
                    }
                    else {
                        match find_path(entities[index].position, entities[*target_index].position, buildings, building_outlines) {
                            None => {
                                // No path to zombie possible, end chase
                                StateChange::Exit
                            },
                            Some(path) => {
                                match path.edges.first() {
                                    None =>
                                    // No path to zombie possible, end chase
                                        StateChange::Exit,
                                    Some(edge) => {
                                        let delta = edge.end.pos - entities[index].position;
                                        (*entity).accelerate_along_vector(delta, args.dt, COP_MOVEMENT_FORCE);
                                        StateChange::Update(CopState::AttackingZombie {
                                            target_index: *target_index,
                                            path: Some(path)
                                        })
                                    }
                                }
                            }
                        }
                    }
                }
                Some(CopState::Aiming { aim_time_remaining, target_index }) => {

                    // Stop aiming if the target is already dead
                    if entities[*target_index].behaviour == Behaviour::Dead {
                        StateChange::Exit
                    }

                    // Stop aiming if we can no longer see the target
                    else if !can_see(buildings,
                                entities[index].position,
                                entities[*target_index].position) {
                        StateChange::Exit
                    }
                    else {

                        let my_pos = entities[index].position;
                        let target_pos = entities[*target_index].position;
                        let delta = target_pos - my_pos;
                        entities[index].look_along_vector(delta, args.dt);

                        // aim_time_remaining -= args.dt;
                        if *aim_time_remaining > args.dt {
                            // Taking aim, update the aim time
                            StateChange::Update(
                                CopState::Aiming { aim_time_remaining: *aim_time_remaining - args.dt, target_index: *target_index }
                            )
                        } else {
                            let angular_deviation =
                                Normal::new(0.0, COP_ANGULAR_ACCURACY_STD_DEV).sample(&mut sim_state.rng);

                            // Finished aiming, take the shot
                            let delta_normal = delta.rotate_by(angular_deviation);

                            // Spawn outside of the entity - don't want to shoot the entity itself
                            let spawn_pos = entities[index].position +
                                BULLET_SPAWN_DISTANCE_MULTIPLIER * ENTITY_RADIUS * delta_normal;

                            // Fire at the target
                            sim_state.projectiles.push(
                                Projectile {
                                    position: spawn_pos,
                                    velocity: BULLET_SPEED * delta_normal,
                                    kind: ProjectileKind::Bullet
                                });

                            sim_state.projectiles.push(
                                Projectile {
                                    position: spawn_pos,
                                    // Casing ejects from the right of the weapon
                                    velocity: CASING_SPEED * delta_normal.right(),
                                    kind: ProjectileKind::Casing
                                });

                            sounds.push(Sound::Gunshot);
                            StateChange::Exit
                        }
                    }

                }
                Some(CopState::Moving { waypoint, mode, path: _ }) => {
                    match mode {
                        MoveMode::Moving => {
                            match find_path(entities[index].position, *waypoint, buildings, building_outlines) {
                                None => {
                                    StateChange::Exit
                                },
                                Some(path) => {
                                    match path.to_vec().get(1) {
                                        None => StateChange::Exit,
                                        Some(&node) => {
                                            let delta = node - entities[index].position;

                                            if *waypoint == node && delta.length_squared() < COP_MIN_DISTANCE_FROM_WAYPOINT_SQUARED {
                                                StateChange::Exit
                                            } else {
                                                entities[index].accelerate_along_vector(delta, args.dt, COP_MOVEMENT_FORCE);
                                                StateChange::Update(
                                                    CopState::Moving { waypoint: *waypoint, mode: MoveMode::Moving, path: Some(path) }
                                                )
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        MoveMode::Sprinting => {
                            // TODO:
                            StateChange::Exit
                        }
                    }
                }
                Some(CopState::Reloading { reload_time_remaining }) => {
                    let half_reload_time = 0.5 * COP_RELOAD_COOLDOWN;
                    let new_reload_time_remaining = reload_time_remaining - args.dt;

                    // Play the reload sound when half-done reloading
                    if *reload_time_remaining > half_reload_time &&
                        half_reload_time > new_reload_time_remaining {
                            sounds.push(Sound::Reload);
                    }

                    if *reload_time_remaining > 0.0 {
                        StateChange::Update(CopState::Reloading {
                            reload_time_remaining: new_reload_time_remaining
                        })
                    } else {
                        // Finished reloading: replenish rounds and return to the previous state
                        *rounds_in_magazine = COP_MAGAZINE_CAPACITY;
                        StateChange::Exit
                    }
                }
                None => {
                    // Reload if you don't have ammo
                    if *rounds_in_magazine <= 0 {
                        StateChange::Enter(CopState::Reloading { reload_time_remaining: COP_RELOAD_COOLDOWN })
                    }
                    // Look for target if you do have ammo
                    else {
                        let my_pos = entities[index].position;

                        let mut min_index = 0;
                        let mut min_distance_sqr = INFINITY;

                        for i in 0..entities.len() {
                            match entities[i].behaviour {

                                // Target zombies
                                Behaviour::Zombie { .. } => {
                                    let delta = entities[i].position - my_pos;
                                    let distance_sqr = delta.length_squared();
                                    if distance_sqr < min_distance_sqr {

                                        // make sure we can actually see the target
                                        if !can_see(buildings,
                                                    entities[index].position,
                                                    entities[i].position) {
                                            continue;
                                        }

                                        min_index = i;
                                        min_distance_sqr = distance_sqr;
                                    }
                                }

                                // Skip everything else
                                _ => ()
                            }
                        }

                        if min_distance_sqr < INFINITY {
                            let aim_time_distribution = Exp::new(COP_AIM_TIME_MEAN);
                            StateChange::Enter(CopState::Aiming {
                                aim_time_remaining: aim_time_distribution.sample(&mut sim_state.rng),
                                target_index: min_index,
                            })
                        } else {
                            // Remain in idle state
                            StateChange::Continue
                        }
                    }
                }
            };

            match state_change {
                StateChange::Exit => drop(state_stack.pop()),
                StateChange::Continue => (),
                StateChange::Update(new) => match state_stack.last_mut() {
                    None => state_stack.push(new),
                    Some(old) => *old = new,
                },
                StateChange::Enter(new) => state_stack.push(new)
            }
        }
        _ => panic!("Entity at index should be a cop!")
    }
    }
}

fn update_zombie(
    args: &UpdateArgs,
    sim_state: &mut State,
    index: usize,
    behaviour: Behaviour) -> Behaviour {

    let entities = &mut sim_state.entities;
    let buildings = &sim_state.buildings;

    let my_pos = entities[index].position;

    match behaviour {
        Behaviour::Zombie { state } => {
            match state {
                ZombieState::Chasing { target_index } => {
                    let target_pos = entities[target_index].position;
                    let delta = target_pos - my_pos;

                    entities[index].accelerate_along_vector(delta, args.dt, ZOMBIE_MOVEMENT_FORCE);

                    match entities[target_index].behaviour {
                        // If alive, check line of sight
                        Behaviour::Cop { .. } | Behaviour::Human => {
                            if delta.length_squared() < ZOMBIE_SIGHT_RADIUS_SQUARE && can_see(buildings,my_pos,target_pos) {
                                // Continue chasing
                                Behaviour::Zombie { state: ZombieState::Chasing { target_index } }
                            } else {
                                // Go to last known position
                                Behaviour::Zombie {
                                    state: ZombieState::Moving { waypoint: target_pos }
                                }
                            }
                        }
                        // Otherwise return to roaming
                        _ => Behaviour::Zombie { state: ZombieState::Roaming },
                    }
                }
                ZombieState::Moving { waypoint } => {
                    match closest_human(my_pos, entities, buildings) {
                        // Continue moving
                        None => {
                            let delta = waypoint - my_pos;
                            entities[index].accelerate_along_vector(delta, args.dt, ZOMBIE_MOVEMENT_FORCE);

                            if delta.length_squared() < COP_MIN_DISTANCE_FROM_WAYPOINT_SQUARED {
                                Behaviour::Zombie { state: ZombieState::Roaming }
                            } else {
                                Behaviour::Zombie { state: ZombieState::Moving { waypoint } }
                            }
                        },
                        // Start chasing nearest human
                        Some(i) => {
                            let delta = entities[i].position - my_pos;
                            entities[index].accelerate_along_vector(delta, args.dt, ZOMBIE_MOVEMENT_FORCE);
                            Behaviour::Zombie { state: ZombieState::Chasing { target_index: i } }
                        }
                    }
                }
                ZombieState::Roaming => {
                    // Attempt to acquire a target
                    match closest_human(my_pos, entities, buildings) {
                        None => Behaviour::Zombie { state: ZombieState::Roaming },
                        Some(i) => {
                            let delta = entities[i].position - my_pos;
                            entities[index].accelerate_along_vector(delta, args.dt, ZOMBIE_MOVEMENT_FORCE);
                            Behaviour::Zombie { state: ZombieState::Chasing { target_index: i} }
                        }
                    }
                }
            }
        }
        _ => panic!("Entity at index should be a zombie!")
    }
}

// Get the index of the closest human in line of sight and sight radius
fn closest_human(my_pos: Vector2, entities: &Vec<Entity>, buildings: &Vec<Polygon>) -> Option<usize> {
    let mut min_distance_sqr = INFINITY;
    let mut closest_index: Option<usize> = None;

    for i in 0..entities.len() {
        match entities[i].behaviour {
            Behaviour::Cop { .. } | Behaviour::Human => {
                let delta_squared = (my_pos - entities[i].position).length_squared();
                if delta_squared < ZOMBIE_SIGHT_RADIUS_SQUARE &&
                    can_see(buildings, my_pos, entities[i].position) &&
                    delta_squared < min_distance_sqr {

                    min_distance_sqr = delta_squared;
                    closest_index = Some(i);
                }
            }
            _ => ()
        }
    }

    closest_index
}

fn simulate_human(args: &UpdateArgs, entities: &mut Vec<Entity>, buildings: &Vec<Polygon>, index: usize) {
    let my_pos = entities[index].position;

    let mut min_delta = Vector2::zero();
    let mut min_distance_sqr = INFINITY;

    for i in 0..entities.len() {
        match entities[i].behaviour {

            // Run from zombies
            Behaviour::Zombie { .. } => {
                let delta = entities[i].position - my_pos;
                let distance_sqr = delta.length_squared();
                if distance_sqr < HUMAN_SIGHT_RADIUS_SQUARE &&
                    can_see(buildings, my_pos, entities[i].position) &&
                    distance_sqr < min_distance_sqr {

                    min_delta = delta;
                    min_distance_sqr = distance_sqr;
                }
            }

            // Skip everything else
            _ => ()
        }
    }

    if min_distance_sqr < INFINITY {
        // Accelerate away from the nearest zombie
        entities[index].accelerate_along_vector(-min_delta, args.dt, CIVILIAN_MOVEMENT_FORCE);
    }
}
