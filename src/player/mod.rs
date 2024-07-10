use bevy::{
    input::ButtonInput,
    math::{vec3, Vec3},
    prelude::{KeyCode, Res, Resource},
    time::Time,
};

#[derive(Resource, Clone, Copy, PartialEq, Debug)]
pub struct Player {
    pub pos: Vec3,
    pub jump_force: f32,
}

impl Player {
    const MOVE_UNITS: f32 = 10.0;
    const JUMP_UNITS: f32 = 2.5;
    const GRAVITY_UNITS: f32 = 1.5;

    pub fn update(&mut self, keys: Res<ButtonInput<KeyCode>>, time: Res<Time>) {
        let delta = time.delta_seconds();

        if keys.pressed(KeyCode::KeyW) {
            self.pos += vec3(delta * Self::MOVE_UNITS, 0.0, 0.0);
        } else if keys.pressed(KeyCode::KeyS) {
            self.pos += vec3(-delta * Self::MOVE_UNITS, 0.0, 0.0);
        } else if keys.pressed(KeyCode::KeyA) {
            self.pos += vec3(0.0, 0.0, -delta * Self::MOVE_UNITS);
        } else if keys.pressed(KeyCode::KeyD) {
            self.pos += vec3(0.0, 0.0, delta * Self::MOVE_UNITS);
        }

        if keys.pressed(KeyCode::Space) && self.jump_force == 0.0 && self.pos.y == 0.5 {
            self.jump_force = Self::JUMP_UNITS;
        }

        if self.jump_force > delta {
            self.pos += vec3(0.0, self.jump_force * delta, 0.0);
            self.jump_force -= delta;
        } else {
            self.jump_force = 0.0;
        }

        if self.pos.y > 0.5 {
            self.pos.y -= Self::GRAVITY_UNITS * delta;
        }
    }
}
