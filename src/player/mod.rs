use std::f32::consts::*;

use bevy::{input::mouse::MouseMotion, math::Vec3Swizzles, prelude::*};
use bevy_rapier3d::prelude::*;

pub struct FpsControllerPlugin;

impl Plugin for FpsControllerPlugin {
    fn build(&self, app: &mut App) {
        use bevy::input::{gamepad, keyboard, mouse, touch};

        app.add_systems(
            PreUpdate,
            (
                fps_controller_input,
                fps_controller_look,
                fps_controller_move,
                fps_controller_render,
            )
                .chain()
                .after(mouse::mouse_button_input_system)
                .after(keyboard::keyboard_input_system)
                .after(gamepad::gamepad_axis_event_system)
                .after(gamepad::gamepad_button_event_system)
                .after(gamepad::gamepad_connection_system)
                .after(gamepad::gamepad_event_system)
                .after(touch::touch_screen_input_system),
        );
    }
}

#[derive(Component)]
pub struct LogicalPlayer;

#[derive(Component)]
pub struct RenderPlayer {
    pub logical_entity: Entity,
}

#[derive(Component)]
pub struct CameraConfig {
    pub height_offset: f32,
}

#[derive(Component, Default)]
pub struct FpsControllerInput {
    pub jump: bool,
    pub crouch: bool,
    pub pitch: f32,
    pub yaw: f32,
    pub movement: Vec3,
}

#[derive(Component)]
pub struct FpsController {
    pub gravity: f32,
    pub walk_speed: f32,
    pub forward_speed: f32,
    pub side_speed: f32,
    pub air_speed_cap: f32,
    pub air_acceleration: f32,
    pub max_air_speed: f32,
    pub acceleration: f32,
    pub friction: f32,
    pub traction_normal_cutoff: f32,
    pub friction_speed_cutoff: f32,
    pub jump_speed: f32,
    pub crouched_speed: f32,
    pub crouch_speed: f32,
    pub uncrouch_speed: f32,
    pub height: f32,
    pub upright_height: f32,
    pub crouch_height: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub ground_tick: u8,
    pub stop_speed: f32,
    pub sensitivity: f32,
    pub enable_input: bool,
    pub step_offset: f32,
    pub key_forward: KeyCode,
    pub key_back: KeyCode,
    pub key_left: KeyCode,
    pub key_right: KeyCode,
    pub key_jump: KeyCode,
    pub key_crouch: KeyCode,
}

impl Default for FpsController {
    fn default() -> Self {
        Self {
            gravity: 23.0,
            walk_speed: 9.0,
            forward_speed: 30.0,
            side_speed: 30.0,
            air_speed_cap: 2.0,
            air_acceleration: 20.0,
            max_air_speed: 15.0,
            crouched_speed: 5.0,
            crouch_speed: 6.0,
            uncrouch_speed: 8.0,
            height: 3.0,
            upright_height: 3.0,
            crouch_height: 2.0,
            acceleration: 10.0,
            friction: 10.0,
            traction_normal_cutoff: 0.7,
            friction_speed_cutoff: 0.1,
            pitch: 0.0,
            yaw: 0.0,
            ground_tick: 0,
            stop_speed: 1.0,
            jump_speed: 8.5,
            step_offset: 0.25,
            enable_input: true,
            key_forward: KeyCode::KeyW,
            key_back: KeyCode::KeyS,
            key_left: KeyCode::KeyA,
            key_right: KeyCode::KeyD,
            key_jump: KeyCode::Space,
            key_crouch: KeyCode::ShiftLeft,
            sensitivity: 0.001,
        }
    }
}

const ANGLE_EPSILON: f32 = 0.001953125;
const GROUNDED_DISTANCE: f32 = 0.125;
const SLIGHT_SCALE_DOWN: f32 = 0.9375;

pub fn fps_controller_input(
    key_input: Res<ButtonInput<KeyCode>>,
    mut mouse_events: EventReader<MouseMotion>,
    mut query: Query<(&FpsController, &mut FpsControllerInput)>,
) {
    for (controller, mut input) in query
        .iter_mut()
        .filter(|(controller, _)| controller.enable_input)
    {
        let mut mouse_delta = Vec2::ZERO;
        for mouse_event in mouse_events.read() {
            mouse_delta += mouse_event.delta;
        }
        mouse_delta *= controller.sensitivity;

        input.pitch = (input.pitch - mouse_delta.y)
            .clamp(-FRAC_PI_2 + ANGLE_EPSILON, FRAC_PI_2 - ANGLE_EPSILON);
        input.yaw -= mouse_delta.x;
        if input.yaw.abs() > PI {
            input.yaw = input.yaw.rem_euclid(TAU);
        }

        input.movement = Vec3::new(
            get_axis(&key_input, controller.key_right, controller.key_left),
            0.0,
            get_axis(&key_input, controller.key_forward, controller.key_back),
        );
        input.crouch = key_input.pressed(controller.key_crouch);
        input.jump = key_input.pressed(controller.key_jump) && !input.crouch;
    }
}

pub fn fps_controller_look(mut query: Query<(&mut FpsController, &FpsControllerInput)>) {
    for (mut controller, input) in query.iter_mut() {
        controller.pitch = input.pitch;
        controller.yaw = input.yaw;
    }
}

pub fn fps_controller_move(
    time: Res<Time>,
    physics_context: Res<RapierContext>,
    mut query: Query<(
        Entity,
        &FpsControllerInput,
        &mut FpsController,
        &mut Collider,
        &mut Transform,
        &mut Velocity,
    )>,
) {
    let dt = time.delta_seconds();

    for (entity, input, mut controller, mut collider, mut transform, mut velocity) in
        query.iter_mut()
    {
        let filter = QueryFilter::default().exclude_rigid_body(entity);
        let ground_cast = physics_context.cast_shape(
            transform.translation,
            transform.rotation,
            -Vec3::Y,
            &scaled_collider_laterally(&collider, SLIGHT_SCALE_DOWN),
            ShapeCastOptions::with_max_time_of_impact(GROUNDED_DISTANCE),
            filter,
        );

        let speeds = Vec3::new(controller.side_speed, 0.0, controller.forward_speed);
        let mut move_to_world = Mat3::from_axis_angle(Vec3::Y, input.yaw);
        move_to_world.z_axis *= -1.0;
        let mut wish_direction = move_to_world * (input.movement * speeds);
        let mut wish_speed = wish_direction.length();
        if wish_speed > f32::EPSILON {
            wish_direction /= wish_speed;
        }
        let max_speed = if input.crouch {
            controller.crouched_speed
        } else {
            controller.walk_speed
        };

        wish_speed = f32::min(wish_speed, max_speed);

        if let Some((hit, hit_details)) = unwrap_hit_details(ground_cast) {
            let has_traction =
                Vec3::dot(hit_details.normal1, Vec3::Y) > controller.traction_normal_cutoff;

            if controller.ground_tick >= 1 && has_traction {
                let lateral_speed = velocity.linvel.xz().length();
                if lateral_speed > controller.friction_speed_cutoff {
                    let control = f32::max(lateral_speed, controller.stop_speed);
                    let drop = control * controller.friction * dt;
                    let new_speed = f32::max((lateral_speed - drop) / lateral_speed, 0.0);
                    velocity.linvel.x *= new_speed;
                    velocity.linvel.z *= new_speed;
                } else {
                    velocity.linvel = Vec3::ZERO;
                }
                if controller.ground_tick == 1 {
                    velocity.linvel.y = -hit.time_of_impact;
                }
            }

            let mut add = acceleration(
                wish_direction,
                wish_speed,
                controller.acceleration,
                velocity.linvel,
                dt,
            );
            if !has_traction {
                add.y -= controller.gravity * dt;
            }
            velocity.linvel += add;

            if has_traction {
                let linear_velocity = velocity.linvel;
                velocity.linvel -=
                    Vec3::dot(linear_velocity, hit_details.normal1) * hit_details.normal1;

                if input.jump {
                    velocity.linvel.y = controller.jump_speed;
                }
            }

            controller.ground_tick = controller.ground_tick.saturating_add(1);
        } else {
            controller.ground_tick = 0;
            wish_speed = f32::min(wish_speed, controller.air_speed_cap);

            let mut add = acceleration(
                wish_direction,
                wish_speed,
                controller.air_acceleration,
                velocity.linvel,
                dt,
            );
            add.y = -controller.gravity * dt;
            velocity.linvel += add;

            let air_speed = velocity.linvel.xz().length();
            if air_speed > controller.max_air_speed {
                let ratio = controller.max_air_speed / air_speed;
                velocity.linvel.x *= ratio;
                velocity.linvel.z *= ratio;
            }
        }

        /* Crouching */

        let crouch_height = controller.crouch_height;
        let upright_height = controller.upright_height;

        let crouch_speed = if input.crouch {
            -controller.crouch_speed
        } else {
            controller.uncrouch_speed
        };
        controller.height += dt * crouch_speed;
        controller.height = controller.height.clamp(crouch_height, upright_height);

        if let Some(mut capsule) = collider.as_capsule_mut() {
            let radius = capsule.radius();
            let half = Vec3::Y * (controller.height * 0.5 - radius);
            capsule.set_segment(-half, half);
        } else if let Some(mut cylinder) = collider.as_cylinder_mut() {
            cylinder.set_half_height(controller.height * 0.5);
        } else {
            panic!("Controller must use a cylinder or capsule collider")
        }

        if collider.as_cylinder().is_some()
            && controller.step_offset > f32::EPSILON
            && controller.ground_tick >= 1
        {
            let future_position = transform.translation + velocity.linvel * dt;
            let future_position_lifted = future_position + Vec3::Y * controller.step_offset;
            let cast = physics_context.cast_shape(
                future_position_lifted,
                transform.rotation,
                -Vec3::Y,
                &collider,
                ShapeCastOptions::with_max_time_of_impact(
                    controller.step_offset * SLIGHT_SCALE_DOWN,
                ),
                filter,
            );
            if let Some((hit, details)) = unwrap_hit_details(cast) {
                let has_traction_on_ledge =
                    Vec3::dot(details.normal1, Vec3::Y) > controller.traction_normal_cutoff;
                if has_traction_on_ledge {
                    transform.translation.y += controller.step_offset - hit.time_of_impact;
                }
            }
        }

        if controller.ground_tick >= 1 && input.crouch && !input.jump {
            for _ in 0..2 {
                let overhang = overhang_component(
                    entity,
                    &collider,
                    transform.as_ref(),
                    physics_context.as_ref(),
                    velocity.linvel,
                    dt,
                );
                if let Some(overhang) = overhang {
                    velocity.linvel -= overhang;
                }
            }

            if overhang_component(
                entity,
                &collider,
                transform.as_ref(),
                physics_context.as_ref(),
                velocity.linvel,
                dt,
            )
            .is_some()
            {
                velocity.linvel = Vec3::ZERO;
            }
        }
    }
}

fn unwrap_hit_details(
    ground_cast: Option<(Entity, ShapeCastHit)>,
) -> Option<(ShapeCastHit, ShapeCastHitDetails)> {
    if let Some((_, hit)) = ground_cast {
        if let Some(details) = hit.details {
            return Some((hit, details));
        }
    }
    None
}

fn collider_y_offset(collider: &Collider) -> Vec3 {
    Vec3::Y
        * if let Some(cylinder) = collider.as_cylinder() {
            cylinder.half_height()
        } else if let Some(capsule) = collider.as_capsule() {
            capsule.half_height() + capsule.radius()
        } else {
            panic!("Controller must use a cylinder or capsule collider")
        }
}

fn scaled_collider_laterally(collider: &Collider, scale: f32) -> Collider {
    if let Some(cylinder) = collider.as_cylinder() {
        let new_cylinder = Collider::cylinder(cylinder.half_height(), cylinder.radius() * scale);
        new_cylinder
    } else if let Some(capsule) = collider.as_capsule() {
        let new_capsule = Collider::capsule(
            capsule.segment().a(),
            capsule.segment().b(),
            capsule.radius() * scale,
        );
        new_capsule
    } else {
        panic!("Controller must use a cylinder or capsule collider")
    }
}

fn overhang_component(
    entity: Entity,
    collider: &Collider,
    transform: &Transform,
    physics_context: &RapierContext,
    velocity: Vec3,
    dt: f32,
) -> Option<Vec3> {
    let cast_capsule = Collider::capsule(Vec3::Y * 0.25, -Vec3::Y * 0.25, 0.01);
    let filter = QueryFilter::default().exclude_rigid_body(entity);
    let collider_offset = collider_y_offset(collider);
    let future_position = transform.translation - collider_offset + velocity * dt;
    let cast = physics_context.cast_shape(
        future_position,
        transform.rotation,
        -velocity,
        &cast_capsule,
        ShapeCastOptions::with_max_time_of_impact(0.5),
        filter,
    );
    if let Some((_, hit_details)) = unwrap_hit_details(cast) {
        let cast = physics_context.cast_ray(
            future_position + Vec3::Y * 0.125,
            -Vec3::Y,
            0.375,
            false,
            filter,
        );

        if cast.is_none() {
            let normal = -hit_details.normal1;
            let alignment = Vec3::dot(velocity, normal);
            return Some(alignment * normal);
        }
    }
    None
}

fn acceleration(
    wish_direction: Vec3,
    wish_speed: f32,
    acceleration: f32,
    velocity: Vec3,
    dt: f32,
) -> Vec3 {
    let velocity_projection = Vec3::dot(velocity, wish_direction);
    let add_speed = wish_speed - velocity_projection;
    if add_speed <= 0.0 {
        return Vec3::ZERO;
    }

    let acceleration_speed = f32::min(acceleration * wish_speed * dt, add_speed);
    wish_direction * acceleration_speed
}

fn get_pressed(key_input: &Res<ButtonInput<KeyCode>>, key: KeyCode) -> f32 {
    if key_input.pressed(key) {
        1.0
    } else {
        0.0
    }
}

fn get_axis(key_input: &Res<ButtonInput<KeyCode>>, key_pos: KeyCode, key_neg: KeyCode) -> f32 {
    get_pressed(key_input, key_pos) - get_pressed(key_input, key_neg)
}

pub fn fps_controller_render(
    mut render_query: Query<(&mut Transform, &RenderPlayer), With<RenderPlayer>>,
    logical_query: Query<
        (&Transform, &Collider, &FpsController, &CameraConfig),
        (With<LogicalPlayer>, Without<RenderPlayer>),
    >,
) {
    for (mut render_transform, render_player) in render_query.iter_mut() {
        if let Ok((logical_transform, collider, controller, camera_config)) =
            logical_query.get(render_player.logical_entity)
        {
            let collider_offset = collider_y_offset(collider);
            let camera_offset = Vec3::Y * camera_config.height_offset;
            render_transform.translation =
                logical_transform.translation + collider_offset + camera_offset;
            render_transform.rotation =
                Quat::from_euler(EulerRot::YXZ, controller.yaw, controller.pitch, 0.0);
        }
    }
}
