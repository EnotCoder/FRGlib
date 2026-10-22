//! A run of textures played one after another.
//!
//! Only present with the `cached-textures` feature -- see [`Animation`] for
//! why the frames have to already be on the GPU.

use crate::graphics::Texture;

/// A sequence of uploaded textures, played at a fixed rate.
///
/// Holds nothing but the handles and a position in time, so keeping one around
/// is cheap: every frame is already on the GPU, and playing means picking one
/// of them rather than uploading anything. That is the whole point -- the
/// alternative uploads a texture per frame of animation per object, which is
/// what made the first version of the example take a minute to start.
///
/// The rate is a plain number of frames per second, so an animation is not
/// tied to the frame rate of the game: [`advance`](Self::advance) takes the
/// time the last frame really took.
///
/// ```no_run
/// # fn main() -> Result<(), frglib::Error> {
/// # let mut window = frglib::Window::builder("demo").build()?;
/// use frglib::{Animation, Image};
///
/// let mut frames = Vec::new();
/// for name in ["1.png", "2.png", "3.png"] {
///     let image = Image::from_file(name)?;
///     frames.push(window.upload(&image)?);
/// }
/// let mut anim = Animation::new(8.0).with_frames(frames);
///
/// // ...inside the frame loop:
/// # let dt: f32 = 1.0 / 60.0;
/// anim.advance(dt);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Animation {
    frames: Vec<Texture>,
    /// Frames per second. Zero or less freezes the animation on its first
    /// frame rather than dividing by nothing.
    fps: f32,
    /// Seconds since the animation started, already wrapped or clamped.
    at: f32,
    /// Whether reaching the end starts over. A walk cycle loops; an explosion
    /// plays once and stops on its last frame.
    looped: bool,
}

impl Animation {
    /// An empty animation at `fps` frames per second.
    ///
    /// Empty means [`frame`](Self::frame) reports `None` and drawing it puts
    /// nothing on screen, so a caller that builds the frame list later does
    /// not have to invent a placeholder.
    ///
    /// A rate of zero or less freezes the animation on its first frame.
    pub fn new(fps: f32) -> Self {
        Animation {
            frames: Vec::new(),
            fps,
            at: 0.0,
            looped: true,
        }
    }

    /// Sets the frames, in the order they are played.
    ///
    /// The playback position is left alone, so adding frames to an animation
    /// already running does not restart it.
    pub fn with_frames(mut self, frames: impl IntoIterator<Item = Texture>) -> Self {
        self.frames.extend(frames);
        self
    }

    /// Appends one frame, for lists that are built rather than listed.
    pub fn push(&mut self, frame: Texture) {
        self.frames.push(frame);
    }

    /// Whether the animation starts over at the end instead of stopping there.
    ///
    /// Looped by default, which is what a sprite cycle wants. Turning it off
    /// makes the animation hold its last frame -- the behaviour an explosion
    /// or a one-shot intro needs.
    pub fn looped(mut self, looped: bool) -> Self {
        self.looped = looped;
        self
    }

    /// Whether the animation starts over at the end.
    ///
    /// The read side of [`looped`](Self::looped): the builder answers what to
    /// build, this answers what is running.
    pub fn is_looped(&self) -> bool {
        self.looped
    }

    /// Turns looping on or off without moving the playback position.
    ///
    /// Switching a running animation to one-shot leaves it where it is, so a
    /// cycle that gets unlooped part-way through still finishes the pass it is
    /// on rather than jumping to its end.
    pub fn set_looped(&mut self, looped: bool) {
        self.looped = looped;

        // Going one-shot while sitting past the new end would show nothing
        // until the next wrap that will never come.
        if !looped && !self.frames.is_empty() {
            self.at = self.at.min(self.duration());
        }
    }

    /// Moves the animation on by `dt` seconds.
    ///
    /// Pass the time the last frame really took, not a constant: that is what
    /// keeps the rate the same on a machine that is struggling.
    ///
    /// A negative step rewinds, but not past the start. Looping wraps around;
    /// a one-shot animation stops on its last frame and stays there.
    pub fn advance(&mut self, dt: f32) {
        if self.frames.is_empty() {
            return;
        }

        self.at = (self.at + dt).max(0.0);

        let duration = self.duration();
        if duration <= 0.0 {
            self.at = 0.0;
        } else if self.looped {
            self.at %= duration;
        } else {
            self.at = self.at.min(duration);
        }
    }

    /// Puts the animation back on its first frame.
    pub fn rewind(&mut self) {
        self.at = 0.0;
    }

    /// How many frames the animation has.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Whether there are no frames at all.
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Frames per second.
    pub fn fps(&self) -> f32 {
        self.fps
    }

    /// Changes the rate, keeping the position in seconds.
    ///
    /// The position is a time, not a frame number, so speeding up does not
    /// rewind: the animation keeps having played as much as it has played,
    /// and simply shows the later frames that time now lands on.
    pub fn set_fps(&mut self, fps: f32) {
        self.fps = fps;
        // A rate that went non-positive has no duration to sit inside any more.
        if self.duration() <= 0.0 {
            self.at = 0.0;
        }
    }

    /// How long one pass takes, in seconds. Zero when the rate is unusable.
    pub fn duration(&self) -> f32 {
        if self.fps <= 0.0 {
            return 0.0;
        }
        self.frames.len() as f32 / self.fps
    }

    /// Which frame is showing, counting from zero.
    ///
    /// Reports zero for an empty animation; [`frame`](Self::frame) is the one
    /// that tells the two apart.
    pub fn frame_index(&self) -> usize {
        let len = self.frames.len();
        if len == 0 {
            return 0;
        }

        let duration = self.duration();
        if duration <= 0.0 {
            return 0;
        }

        // `at` already fits inside the duration, so this cannot run past the
        // end; the `min` covers the last frame holding for a whole step.
        //
        // Multiplied rather than divided: `(at / duration) * len` routes
        // through `1/3` in f32 for a three-frame pass and lands on 0.99999994
        // instead of 1.0, which truncates a frame short.
        let at = self.at.min(duration);
        ((at * self.fps) as usize).min(len - 1)
    }

    /// The texture showing now, or `None` when there are no frames.
    pub fn frame(&self) -> Option<Texture> {
        self.frames.get(self.frame_index()).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Three frames at two per second: one and a half seconds per pass.
    fn anim() -> Animation {
        Animation::new(2.0).with_frames([Texture::at(0), Texture::at(1), Texture::at(2)])
    }

    #[test]
    fn it_starts_on_the_first_frame() {
        let anim = anim();

        assert_eq!(anim.frame_index(), 0);
        assert_eq!(anim.frame(), Some(Texture::at(0)));
        assert_eq!(anim.len(), 3);
        assert!(!anim.is_empty());
    }

    #[test]
    fn it_moves_on_by_time_not_by_frame() {
        let mut anim = anim();

        // Half a second is one frame at two per second.
        anim.advance(0.5);
        assert_eq!(anim.frame_index(), 1);

        anim.advance(0.5);
        assert_eq!(anim.frame_index(), 2);
    }

    /// A frame's worth of time is still the same frame: the next one starts
    /// after it, not on it.
    #[test]
    fn the_end_of_a_step_belongs_to_the_next_frame() {
        let mut anim = anim();

        anim.advance(0.5);
        anim.advance(0.5);
        anim.advance(0.5);
        assert_eq!(anim.frame_index(), 0);
    }

    /// The whole point of a looping animation: running past the end wraps.
    #[test]
    fn it_wraps_around_the_end() {
        let mut anim = anim();

        anim.advance(1.5);
        assert_eq!(anim.frame_index(), 0);
        assert_eq!(anim.frame(), Some(Texture::at(0)));
    }

    /// A big step wraps just as well, so a stalled frame does not throw the
    /// animation out of phase.
    #[test]
    fn a_step_longer_than_the_pass_wraps_too() {
        let mut anim = anim();

        anim.advance(7.0);
        assert_eq!(anim.frame(), Some(Texture::at(2)));
    }

    /// An unlooped animation stops on its last frame and stays there.
    #[test]
    fn an_unlooped_animation_stops_at_the_end() {
        let mut anim = anim().looped(false);

        anim.advance(10.0);
        assert_eq!(anim.frame_index(), 2);
        assert_eq!(anim.frame(), Some(Texture::at(2)));

        anim.advance(10.0);
        assert_eq!(anim.frame_index(), 2);
    }

    #[test]
    fn rewind_puts_it_back_on_the_first_frame() {
        let mut anim = anim();

        anim.advance(1.0);
        anim.rewind();
        assert_eq!(anim.frame_index(), 0);
        assert_eq!(anim.frame(), Some(Texture::at(0)));
    }

    /// A negative step rewinds, but the animation does not end up before its
    /// own start.
    #[test]
    fn a_negative_step_rewinds_but_not_past_the_start() {
        let mut anim = anim();

        anim.advance(-5.0);
        assert_eq!(anim.frame_index(), 0);
    }

    /// An empty animation shows nothing, rather than failing or showing a
    /// frame that was never given.
    #[test]
    fn an_empty_animation_has_no_frame() {
        let mut anim = Animation::new(2.0);

        assert!(anim.is_empty());
        assert_eq!(anim.frame(), None);
        assert_eq!(anim.frame_index(), 0);

        // Advancing it is not an error either.
        anim.advance(1.0);
        assert_eq!(anim.frame(), None);
    }

    /// A rate of zero freezes instead of dividing by nothing.
    #[test]
    fn a_rate_of_zero_freezes_the_animation() {
        let mut anim = Animation::new(0.0).with_frames([Texture::at(0), Texture::at(1)]);

        assert_eq!(anim.duration(), 0.0);

        anim.advance(10.0);
        assert_eq!(anim.frame_index(), 0);
    }

    /// The rate is per second, so changing it changes how far a given time
    /// reaches -- without rewinding the animation.
    #[test]
    fn raising_the_rate_shows_later_frames_for_the_same_time() {
        let mut anim = anim();

        anim.advance(0.5);
        assert_eq!(anim.frame_index(), 1);

        // The same half second is now a whole frame at four per second.
        anim.set_fps(4.0);
        assert_eq!(anim.frame_index(), 2);
    }

    /// Slowing down to an unusable rate parks the animation rather than
    /// leaving it sitting at a time nothing maps to.
    #[test]
    fn a_rate_that_goes_unusable_parks_the_animation() {
        let mut anim = anim();

        anim.advance(0.5);
        anim.set_fps(0.0);

        assert_eq!(anim.frame_index(), 0);
        assert_eq!(anim.duration(), 0.0);
    }

    /// One pass takes as long as the frames and the rate say it does.
    #[test]
    fn the_duration_is_the_frames_over_the_rate() {
        assert_eq!(anim().duration(), 1.5);

        let slow = Animation::new(0.5).with_frames([Texture::at(0), Texture::at(1)]);
        assert_eq!(slow.duration(), 4.0);
    }

    /// Frames are played in the order they were given.
    ///
    /// The step is one whole frame: at two per second that is half a second.
    #[test]
    fn the_frames_are_played_in_order() {
        let mut anim = anim();

        let mut seen = Vec::new();
        for _ in 0..6 {
            seen.push(anim.frame());
            anim.advance(0.5);
        }

        assert_eq!(
            seen,
            vec![
                Some(Texture::at(0)),
                Some(Texture::at(1)),
                Some(Texture::at(2)),
                Some(Texture::at(0)),
                Some(Texture::at(1)),
                Some(Texture::at(2)),
            ]
        );
    }

    /// Half a frame of a step leaves the frame where it was: the next one
    /// starts after a whole frame, not halfway through it.
    #[test]
    fn a_half_frame_step_holds_the_same_frame() {
        let mut anim = anim();

        let mut seen = Vec::new();
        for _ in 0..4 {
            seen.push(anim.frame_index());
            anim.advance(0.25);
        }

        assert_eq!(seen, vec![0, 0, 1, 1]);
    }

    /// Frames can be appended to an animation that is already running, and
    /// that does not restart it.
    #[test]
    fn pushing_a_frame_keeps_the_position() {
        let mut anim = anim();

        anim.advance(0.5);
        anim.push(Texture::at(3));

        assert_eq!(anim.len(), 4);
        assert_eq!(anim.frame_index(), 1);
    }

    #[test]
    fn looping_is_on_unless_asked_otherwise() {
        assert!(Animation::new(2.0).is_looped());
        assert!(!Animation::new(2.0).looped(false).is_looped());
    }

    /// Switching a running animation to one-shot leaves it where it is, so it
    /// finishes the pass it is on instead of jumping to the end.
    #[test]
    fn unlooping_a_running_animation_keeps_where_it_is() {
        let mut anim = anim();

        anim.advance(0.5);
        anim.set_looped(false);

        assert!(!anim.is_looped());
        assert_eq!(anim.frame_index(), 1);
    }

    /// Once unlooped, it stops at the end rather than wrapping.
    #[test]
    fn an_animation_unlooped_mid_play_still_reaches_the_end() {
        let mut anim = anim();

        anim.advance(0.5);
        anim.set_looped(false);
        anim.advance(10.0);

        assert_eq!(anim.frame_index(), 2);
    }

    /// Turning looping back on resumes wrapping from the same position: the
    /// step past the end comes out at the start instead of being clamped.
    #[test]
    fn looping_again_resumes_wrapping() {
        let mut anim = anim().looped(false);

        anim.advance(5.0);
        assert_eq!(anim.frame_index(), 2);

        anim.set_looped(true);
        anim.advance(0.1);
        assert_eq!(anim.frame_index(), 0);
    }
}
