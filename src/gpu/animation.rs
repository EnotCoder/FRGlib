//! Playback of glTF animation clips.
//!
//! A [`GltfAnimationPlayer`] owns a copy of the model's node hierarchy, skins
//! and clips. It evaluates the selected clip at the current time and exposes
//! the resulting node transforms and joint matrices. Pass the latter to
//! [`Renderer3D::draw_gltf_animated`](super::Renderer3D::draw_gltf_animated) to
//! deform the model on the GPU.
//!
//! Clips loop automatically. `CUBICSPLINE` channels use the middle value of
//! each triple, so tangents are ignored and the result is a linear
//! approximation of the spline.

use glam::{Mat4, Quat, Vec3};

use super::gltf::{
    joint_matrices, node_world_matrices, GltfAnimation, GltfAnimationChannel,
    GltfAnimationProperty, GltfInterpolation, GltfModel, GltfNode, GltfSkin,
};

/// Evaluates one glTF animation clip at a time.
#[derive(Debug, Clone)]
pub struct GltfAnimationPlayer {
    nodes: Vec<GltfNode>,
    skins: Vec<GltfSkin>,
    animations: Vec<GltfAnimation>,
    clip: Option<usize>,
    time: f32,
    speed: f32,

    translations: Vec<Vec3>,
    rotations: Vec<Quat>,
    scales: Vec<Vec3>,
    locals: Vec<Mat4>,
    world: Vec<Mat4>,
    joints: Vec<Vec<Mat4>>,
}

impl GltfAnimationPlayer {
    /// Creates a player using the model's first clip, if it has one.
    ///
    /// Models without animations are still useful: the player then exposes the
    /// bind pose, so animated and static draws can share one code path.
    pub fn new(model: &GltfModel) -> Self {
        let clip = if model.animations.is_empty() {
            None
        } else {
            Some(0)
        };
        Self::build(model, clip)
    }

    /// Creates a player for a specific clip.
    ///
    /// Returns `None` when `clip` is not a valid index into
    /// [`GltfModel::animations`].
    pub fn with_clip(model: &GltfModel, clip: usize) -> Option<Self> {
        if clip >= model.animations.len() {
            return None;
        }
        Some(Self::build(model, Some(clip)))
    }

    fn build(model: &GltfModel, clip: Option<usize>) -> Self {
        let count = model.nodes.len();
        let mut player = Self {
            nodes: model.nodes.clone(),
            skins: model.skins.clone(),
            animations: model.animations.clone(),
            clip,
            time: 0.0,
            speed: 1.0,
            translations: vec![Vec3::ZERO; count],
            rotations: vec![Quat::IDENTITY; count],
            scales: vec![Vec3::ONE; count],
            locals: vec![Mat4::IDENTITY; count],
            world: vec![Mat4::IDENTITY; count],
            joints: model.skins.iter().map(|_| Vec::new()).collect(),
        };
        player.evaluate();
        player
    }

    /// Number of clips stored in the model.
    pub fn clip_count(&self) -> usize {
        self.animations.len()
    }

    /// Index of the selected clip, or `None` for the bind pose.
    pub fn clip(&self) -> Option<usize> {
        self.clip
    }

    /// Selects another clip and rewinds to its start.
    ///
    /// Returns `false` and keeps the current clip when the index is invalid.
    pub fn set_clip(&mut self, clip: usize) -> bool {
        if clip >= self.animations.len() {
            return false;
        }
        self.clip = Some(clip);
        self.set_time(0.0);
        true
    }

    /// Duration of the selected clip in seconds, or zero for the bind pose.
    pub fn duration(&self) -> f32 {
        self.clip.map_or(0.0, |clip| self.animations[clip].duration)
    }

    /// Current time inside the clip, in seconds.
    pub fn time(&self) -> f32 {
        self.time
    }

    /// Playback rate: `1.0` is normal speed, negative values play backwards.
    pub fn speed(&self) -> f32 {
        self.speed
    }

    /// Changes the playback rate without resetting the current time.
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
    }

    /// Jumps to a time inside the clip. The time wraps around the clip length.
    pub fn set_time(&mut self, time: f32) {
        let duration = self.duration();
        self.time = if duration > 0.0 {
            time.rem_euclid(duration)
        } else {
            0.0
        };
        self.evaluate();
    }

    /// Advances the clip by `dt` seconds and recomputes the joint matrices.
    ///
    /// Call this once per frame before the draw calls. Passing a clamped
    /// `FrameClock::dt()` keeps a long stall from skipping a large part of a
    /// short clip.
    pub fn update(&mut self, dt: f32) {
        if self.clip.is_none() {
            return;
        }
        self.set_time(self.time + dt * self.speed);
    }

    /// World transform of a node in the current pose.
    ///
    /// Skinned meshes ignore their node transform in glTF, so a draw should use
    /// identity for them and only use the joint matrices.
    pub fn node_transform(&self, node: usize) -> Option<Mat4> {
        self.world.get(node).copied()
    }

    /// Joint matrices for a skin in the current pose.
    ///
    /// The slice length matches [`GltfSkin::joints`](super::gltf::GltfSkin::joints),
    /// which is the order used by `JOINTS_0`.
    pub fn joint_matrices(&self, skin: usize) -> Option<&[Mat4]> {
        self.joints.get(skin).map(Vec::as_slice)
    }

    fn evaluate(&mut self) {
        for (index, node) in self.nodes.iter().enumerate() {
            self.translations[index] = node.translation;
            self.rotations[index] = node.rotation;
            self.scales[index] = node.scale;
        }

        let time = self.time;
        if let Some(clip) = self.clip {
            for channel in &self.animations[clip].channels {
                if channel.node >= self.nodes.len() {
                    continue;
                }
                let value = sample(channel, time);
                match channel.property {
                    GltfAnimationProperty::Translation => {
                        self.translations[channel.node] = Vec3::new(value[0], value[1], value[2]);
                    }
                    GltfAnimationProperty::Rotation => {
                        self.rotations[channel.node] = Quat::from_array(value).normalize();
                    }
                    GltfAnimationProperty::Scale => {
                        self.scales[channel.node] = Vec3::new(value[0], value[1], value[2]);
                    }
                }
            }
        }

        for (index, local) in self.locals.iter_mut().enumerate() {
            *local = Mat4::from_translation(self.translations[index])
                * Mat4::from_quat(self.rotations[index])
                * Mat4::from_scale(self.scales[index]);
        }
        self.world = node_world_matrices(&self.nodes, &self.locals);
        self.joints = self
            .skins
            .iter()
            .map(|skin| joint_matrices(skin, &self.world))
            .collect();
    }
}

/// Samples one channel's `(time, value)` pairs at `time`.
fn sample(channel: &GltfAnimationChannel, time: f32) -> [f32; 4] {
    let samples = &channel.samples;
    let first = samples[0].1;
    if time <= samples[0].0 {
        return first;
    }

    // First keyframe whose time is after `time`; the one before it is the
    // start of the current segment.
    let next = samples.partition_point(|(key_time, _)| *key_time <= time);
    if next >= samples.len() {
        return samples[samples.len() - 1].1;
    }
    if channel.interpolation == GltfInterpolation::Step {
        return samples[next - 1].1;
    }

    let (start_time, start) = samples[next - 1];
    let (end_time, end) = samples[next];
    let alpha = ((time - start_time) / (end_time - start_time).max(f32::EPSILON)).clamp(0.0, 1.0);

    if channel.property == GltfAnimationProperty::Rotation {
        let start = Quat::from_array(start).normalize();
        let end = Quat::from_array(end).normalize();
        return start.slerp(end, alpha).to_array();
    }

    let mut value = start;
    for (component, (from, to)) in value.iter_mut().zip(start.iter().zip(end.iter())) {
        *component = from + (to - from) * alpha;
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel(
        samples: Vec<(f32, [f32; 4])>,
        property: GltfAnimationProperty,
    ) -> GltfAnimationChannel {
        GltfAnimationChannel {
            node: 0,
            property,
            interpolation: GltfInterpolation::Linear,
            samples,
        }
    }

    #[test]
    fn a_translation_channel_interpolates_between_keys() {
        let result = sample(
            &channel(
                vec![(0.0, [0.0, 0.0, 0.0, 0.0]), (2.0, [4.0, 6.0, 8.0, 0.0])],
                GltfAnimationProperty::Translation,
            ),
            0.5,
        );
        assert_eq!(result, [1.0, 1.5, 2.0, 0.0]);
    }

    #[test]
    fn a_rotation_channel_slerps_quaternions() {
        let result = sample(
            &channel(
                vec![(0.0, [0.0, 0.0, 0.0, 1.0]), (1.0, [0.0, 1.0, 0.0, 0.0])],
                GltfAnimationProperty::Rotation,
            ),
            0.5,
        );
        let rotation = Quat::from_array(result);
        assert!(rotation.is_normalized());
        assert!(rotation.x.abs() < 1e-5);
        assert!(rotation.z.abs() < 1e-5);
        assert!((rotation.y - rotation.w).abs() < 1e-5);
    }

    #[test]
    fn a_step_channel_holds_the_previous_value() {
        let mut step = channel(
            vec![(0.0, [0.0; 4]), (1.0, [1.0; 4])],
            GltfAnimationProperty::Scale,
        );
        step.interpolation = GltfInterpolation::Step;
        assert_eq!(sample(&step, 0.75), [0.0; 4]);
        assert_eq!(sample(&step, 1.25), [1.0; 4]);
    }

    #[test]
    fn sampling_clamps_outside_the_key_range() {
        let channel = channel(
            vec![(1.0, [2.0; 4]), (3.0, [6.0; 4])],
            GltfAnimationProperty::Translation,
        );
        assert_eq!(sample(&channel, -1.0), [2.0; 4]);
        assert_eq!(sample(&channel, 9.0), [6.0; 4]);
    }

    #[test]
    fn playback_wraps_at_the_clip_duration() {
        let model = GltfModel {
            primitives: Vec::new(),
            instances: Vec::new(),
            nodes: vec![GltfNode::default()],
            skins: Vec::new(),
            animations: vec![GltfAnimation {
                channels: vec![channel(
                    vec![(0.0, [0.0; 4]), (2.0, [2.0; 4])],
                    GltfAnimationProperty::Translation,
                )],
                duration: 2.0,
            }],
            materials: Vec::new(),
            images: Vec::new(),
        };
        let mut player = GltfAnimationPlayer::new(&model);
        assert_eq!(player.duration(), 2.0);
        player.update(2.5);
        assert!((player.time() - 0.5).abs() < 1e-6);
        assert!((player.node_transform(0).unwrap().w_axis.x - 0.5).abs() < 1e-6);
    }
}
