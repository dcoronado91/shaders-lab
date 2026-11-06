//! Camera, light and time presets for the headless evidence modes. Each
//! body has a shot with its own name, plus close-ups for specific effects.

use crate::scene::{Body, SceneState};

#[derive(Clone, Copy, Debug)]
pub struct Shot {
    pub name: &'static str,
    pub body: Body,
    pub yaw: f32,
    pub pitch: f32,
    /// Camera distance; `None` keeps the body's default.
    pub distance: Option<f32>,
    /// Light azimuth; `None` keeps the body's default.
    pub light: Option<f32>,
    /// Light rotation while recording, radians per second.
    pub light_speed: f32,
    pub yaw_speed: f32,
    pub time: f32,
    /// Simulated seconds per recorded second (time-lapse factor).
    pub time_speed: f32,
    pub seconds: f32,
    pub coverage: Option<f32>,
    pub focus_moon: bool,
}

const fn body_shot(name: &'static str, body: Body) -> Shot {
    Shot {
        name,
        body,
        yaw: 0.0,
        pitch: 0.18,
        distance: None,
        light: None,
        light_speed: 0.0,
        yaw_speed: 0.0,
        time: 0.0,
        time_speed: 1.0,
        seconds: 3.0,
        coverage: None,
        focus_moon: false,
    }
}

pub const SHOTS: [Shot; 15] = [
    Shot { time_speed: 2.5, ..body_shot("estrella", Body::Star) },
    body_shot("rocoso", Body::Rocky),
    Shot { pitch: 0.3, ..body_shot("gaseoso", Body::Gas) },
    Shot { time_speed: 2.0, ..body_shot("lava", Body::Lava) },
    Shot { time_speed: 2.0, ..body_shot("hielo", Body::Ice) },
    Shot { time_speed: 3.0, ..body_shot("toxico", Body::Toxic) },
    Shot { pitch: 0.42, yaw_speed: 0.15, time_speed: 4.0, ..body_shot("todos", Body::All) },
    Shot {
        yaw: 0.4,
        distance: Some(2.6),
        light: Some(-0.4),
        time_speed: 12.0,
        coverage: Some(0.5),
        ..body_shot("nubes", Body::Rocky)
    },
    Shot {
        yaw: 0.6,
        pitch: 0.1,
        distance: Some(1.75),
        light: Some(-1.1),
        light_speed: 0.12,
        time_speed: 8.0,
        coverage: Some(0.45),
        ..body_shot("sombras_nubes", Body::Rocky)
    },
    Shot {
        yaw: -0.3,
        distance: Some(3.0),
        light: Some(-0.6),
        light_speed: -0.55,
        seconds: 4.0,
        ..body_shot("dia_noche", Body::Rocky)
    },
    Shot {
        yaw: 0.0,
        distance: Some(2.6),
        light: Some(0.35),
        light_speed: -0.25,
        coverage: Some(0.25),
        ..body_shot("especular", Body::Rocky)
    },
    Shot {
        yaw: 0.8,
        pitch: 0.25,
        distance: Some(1.8),
        light: Some(-1.25),
        light_speed: 0.3,
        coverage: Some(0.15),
        ..body_shot("bump_rocoso", Body::Rocky)
    },
    Shot {
        yaw: 0.6,
        distance: Some(0.95),
        light: Some(-0.5),
        light_speed: -0.45,
        seconds: 4.0,
        focus_moon: true,
        ..body_shot("bump_luna", Body::Rocky)
    },
    Shot {
        yaw: 0.0,
        pitch: 0.2,
        distance: Some(2.4),
        light: Some(-0.5),
        time_speed: 10.0,
        ..body_shot("vortice", Body::Gas)
    },
    Shot {
        yaw: 0.9,
        pitch: 0.55,
        distance: Some(5.6),
        light: Some(-2.2),
        ..body_shot("anillos", Body::Gas)
    },
];

impl Shot {
    pub fn find(name: &str) -> Result<Shot, String> {
        SHOTS.iter().copied().find(|s| s.name == name).ok_or_else(|| {
            let names: Vec<_> = SHOTS.iter().map(|s| s.name).collect();
            format!("unknown shot '{name}', expected one of: {}", names.join(", "))
        })
    }

    /// Scene state `t` seconds into the recording.
    pub fn state_at(&self, t: f32) -> SceneState {
        let mut state = SceneState::new(self.body);
        state.time = self.time + t * self.time_speed;
        state.camera.yaw = self.yaw + t * self.yaw_speed;
        state.camera.pitch = self.pitch;
        if let Some(d) = self.distance {
            state.camera.distance = d;
        }
        state.light_angle = self.light.unwrap_or(state.light_angle) + t * self.light_speed;
        if let Some(c) = self.coverage {
            state.params.cloud_coverage = c;
        }
        state.focus_moon = self.focus_moon;
        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_body_has_a_shot_with_its_name() {
        for body in Body::ALL {
            assert_eq!(Shot::find(body.name()).unwrap().body, body);
        }
        assert!(Shot::find("nope").is_err());
    }
}
