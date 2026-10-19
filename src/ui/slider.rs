//! A horizontal slider with a knob that is dragged along a track.

use crate::event::MouseButton;
use crate::graphics::{Color, Object};
use crate::math::Rect;
use crate::window::{Draw, Error};
use crate::Event;

/// How a slider looks right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliderState {
    /// Not under the cursor, not being dragged.
    Idle,
    /// Under the cursor.
    Hover,
    /// Being dragged.
    Dragging,
}

/// A track with a knob that slides along it.
///
/// The slider carries a number and nothing else: it has no caption of its own,
/// because a label belongs *beside* a slider rather than on it, and a caption
/// centred on the track would be hidden by the knob half the time. Draw the
/// text yourself, wherever it belongs, and let [`draw`](Self::draw) do the track.
///
/// ```no_run
/// # fn main() -> Result<(), frglib::Error> {
/// # let events: Vec<frglib::Event> = Vec::new();
/// use frglib::{Rect, Slider};
///
/// let mut volume = Slider::new(Rect::new(40.0, 120.0, 240.0, 28.0))
///     .with_range(0.0, 1.0, 0.05);
///
/// // ...inside the frame loop:
/// volume.update(&events);
/// println!("{:?}", volume.value());
/// # Ok(())
/// # }
/// ```
///
/// Coordinates are screen ones, for the same reason a [`Button`](crate::Button)
/// uses them: a [`Draw`] carrying a camera would drag the interface around with
/// the world, so [`draw`](Self::draw) takes the camera off for the length of the
/// call and puts it back afterwards.
pub struct Slider {
    rect: Rect,
    min: f32,
    max: f32,
    /// Granularity. Zero means the value is continuous.
    step: f32,
    value: f32,
    state: SliderState,
    track: Color,
    fill: Color,
    knob: Color,
    knob_held: Color,
}

impl Slider {
    /// A slider covering `rect`, ranging from 0 to 1 with no stepping.
    ///
    /// It starts at the bottom of its range, the way a volume slider does.
    pub fn new(rect: Rect) -> Self {
        Slider {
            rect,
            min: 0.0,
            max: 1.0,
            step: 0.0,
            value: 0.0,
            state: SliderState::Idle,
            track: Color::rgb(38, 42, 54),
            fill: Color::rgb(62, 104, 176),
            knob: Color::rgb(226, 232, 244),
            knob_held: Color::rgb(150, 106, 58),
        }
    }

    /// Sets the value range and the granularity of one step.
    ///
    /// A `step` of zero leaves the value continuous. A non-zero one snaps to
    /// multiples of `step` counted from `min`, so `with_range(0.0, 1.0, 0.05)`
    /// gives twenty positions and never lands anywhere in between.
    ///
    /// The current value is pulled back into the new range, because a slider
    /// that kept its old number would have its knob somewhere the value cannot
    /// reach.
    pub fn with_range(mut self, min: f32, max: f32, step: f32) -> Self {
        self.min = min;
        self.max = max;
        self.step = step;
        self.set_value(self.value);
        self
    }

    /// Sets the four colours: track, filled part, knob, and the knob while it
    /// is held.
    pub fn with_colors(mut self, track: Color, fill: Color, knob: Color, knob_held: Color) -> Self {
        self.track = track;
        self.fill = fill;
        self.knob = knob;
        self.knob_held = knob_held;
        self
    }

    /// Feeds this frame's events in, and reports whether the value moved.
    ///
    /// Two things differ from a [`Button`](crate::Button), both on purpose.
    ///
    /// The value follows the press instead of the release, because a slider is
    /// a position rather than a trigger: moving on release makes the knob feel
    /// stuck for as long as the button is held, which is the wrong signal for
    /// "put it here".
    ///
    /// And a drag keeps following the cursor once it has left the track, because
    /// that is what dragging means. A button stops at its edge so that letting
    /// go elsewhere cancels the click; a slider that stopped too would make the
    /// last step to an end unreachable in one go.
    ///
    /// Only the left button counts. Losing the window focus ends the drag, so a
    /// knob does not keep following a pointer nobody can see.
    pub fn update(&mut self, events: &[Event]) -> bool {
        let mut moved = false;

        for event in events {
            match event {
                Event::MouseMotion { x, y } => {
                    if self.state == SliderState::Dragging {
                        moved |= self.drag_to(*x);
                    } else {
                        // Not held: the cursor only lights the slider up.
                        self.state = if self.rect.contains(*x, *y) {
                            SliderState::Hover
                        } else {
                            SliderState::Idle
                        };
                    }
                }

                Event::MouseButtonDown { button, x, y, .. }
                    if *button == MouseButton::Left && self.rect.contains(*x, *y) =>
                {
                    self.state = SliderState::Dragging;
                    moved |= self.drag_to(*x);
                }

                Event::MouseButtonUp { button, x, y, .. } if *button == MouseButton::Left => {
                    // A release anywhere ends the drag; whether it landed on
                    // the track only decides what the slider looks like next.
                    if self.state == SliderState::Dragging {
                        self.state = if self.rect.contains(*x, *y) {
                            SliderState::Hover
                        } else {
                            SliderState::Idle
                        };
                    }
                }

                // Alt-tab keeps the last mouse position but stops sending
                // anything, so without this a knob would follow a frozen pointer
                // the moment the pointer moved again.
                Event::FocusLost => self.state = SliderState::Idle,

                _ => {}
            }
        }

        moved
    }

    /// The state as of the last [`update`](Self::update).
    pub fn state(&self) -> SliderState {
        self.state
    }

    /// The area the slider covers, in screen pixels.
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// Where the knob sits now, in the slider's own units.
    pub fn value(&self) -> f32 {
        self.value
    }

    /// Moves the knob, clamped to the range and snapped to the step.
    ///
    /// Does not report whether anything moved: this is the caller telling the
    /// slider where to be, not the slider reacting to the cursor.
    pub fn set_value(&mut self, value: f32) {
        self.value = self.clamp_and_snap(value);
    }

    /// The value as a plain 0 to 1, which is what most callers actually want.
    ///
    /// An empty range reports 0 rather than dividing by zero.
    pub fn fraction(&self) -> f32 {
        let span = self.max - self.min;
        if span <= 0.0 {
            return 0.0;
        }
        ((self.value - self.min) / span).clamp(0.0, 1.0)
    }

    /// Side of the square knob, which is the height of the track.
    ///
    /// The knob is a square because this crate draws rectangles and nothing
    /// else. The knob is as tall as the track so that it fills it edge to edge,
    /// and that is also what shortens the travel: a knob centred on the very
    /// edge would hang half outside the track.
    pub fn knob_side(&self) -> f32 {
        self.rect.h.max(1.0)
    }

    /// Draws the track, the filled part and the knob.
    ///
    /// No caption: see the type docs for why the label is the caller's job.
    pub fn draw(&self, d: &mut Draw<'_>) -> Result<(), Error> {
        let side = self.knob_side();
        let cx = self.knob_x();
        let cy = self.rect.y + self.rect.h / 2.0;
        let knob = if self.state == SliderState::Dragging {
            self.knob_held
        } else {
            self.knob
        };

        // The filled part runs from the left edge to the middle of the knob, so
        // the knob always sits on a full-looking bar rather than floating at
        // the end of a stub.
        let filled = cx - self.rect.x;

        d.screen_space(|d| {
            d.fill_object(
                &Object::from_xywh(
                    self.rect.x + self.rect.w / 2.0,
                    cy,
                    self.rect.w,
                    self.rect.h,
                ),
                self.track,
            )?;

            if filled > 0.0 {
                d.fill_object(
                    &Object::from_xywh(self.rect.x + filled / 2.0, cy, filled, self.rect.h),
                    self.fill,
                )?;
            }

            d.fill_object(&Object::from_xywh(cx, cy, side, side), knob)
        })
    }

    /// Horizontal centre of the knob, in screen pixels.
    fn knob_x(&self) -> f32 {
        self.rect.x + self.knob_side() / 2.0 + self.fraction() * self.travel()
    }

    /// How far the knob's centre can go from the left inner edge to the right
    /// one. Never negative: a track narrower than its own knob simply does not
    /// move.
    fn travel(&self) -> f32 {
        (self.rect.w - self.knob_side()).max(0.0)
    }

    /// The value a click at `x` asks for.
    ///
    /// The cursor is measured from the knob's own half width, so that the ends
    /// of the range are reachable: pressing on the far half of the knob's area
    /// still lands on the end of the track rather than one knob-width short.
    fn value_at(&self, x: f32) -> f32 {
        let (span, travel) = (self.max - self.min, self.travel());
        if span <= 0.0 || travel <= 0.0 {
            return self.min;
        }

        let t = ((x - self.rect.x - self.knob_side() / 2.0) / travel).clamp(0.0, 1.0);
        self.clamp_and_snap(self.min + t * span)
    }

    /// Pulls a number into the range and onto the step grid.
    fn clamp_and_snap(&self, value: f32) -> f32 {
        let value = value.clamp(self.min, self.max);

        if self.step <= 0.0 {
            return value;
        }

        let steps = ((value - self.min) / self.step).round();
        (self.min + steps * self.step).clamp(self.min, self.max)
    }

    /// Moves the knob to `x`. Reports whether the value actually changed.
    fn drag_to(&mut self, x: f32) -> bool {
        let value = self.value_at(x);
        if value == self.value {
            return false;
        }
        self.value = value;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The slider under test, in the middle of a notional screen.
    fn slider() -> Slider {
        Slider::new(Rect::new(100.0, 100.0, 200.0, 40.0))
    }

    /// A slider that counts in hundredths, so expected values are exact.
    fn stepped() -> Slider {
        slider().with_range(0.0, 1.0, 0.05)
    }

    /// Somewhere well outside the track.
    ///
    /// Константы «внутри трека» здесь нет в отличие от тестов кнопки: у
    /// слайдера важна не сама точка, а где именно она, поэтому координаты в
    /// каждом тесте стоят свои.
    const OUT: (f32, f32) = (600.0, 600.0);

    fn motion(x: f32, y: f32) -> Event {
        Event::MouseMotion { x, y }
    }

    fn down(x: f32, y: f32) -> Event {
        Event::MouseButtonDown {
            button: MouseButton::Left,
            x,
            y,
            clicks: 1,
        }
    }

    fn up(x: f32, y: f32) -> Event {
        Event::MouseButtonUp {
            button: MouseButton::Left,
            x,
            y,
            clicks: 1,
        }
    }

    /// A fresh slider sits at the bottom of its range.
    #[test]
    fn starts_at_the_bottom() {
        let s = slider().with_range(0.0, 100.0, 1.0);
        assert_eq!(s.value(), 0.0);
    }

    /// Pressing on the middle of the track puts the knob in the middle.
    #[test]
    fn pressing_the_middle_lands_in_the_middle() {
        let mut s = stepped();

        s.update(&[down(200.0, 120.0)]);
        assert_eq!(s.value(), 0.5);
    }

    /// The whole point of a slider: the value moves on the press, not on the
    /// release.
    #[test]
    fn the_value_moves_on_the_press() {
        let mut s = stepped();

        assert!(s.update(&[down(270.0, 120.0)]));
        assert_eq!(s.state(), SliderState::Dragging);
    }

    /// Pressing on the right-hand end asks for the end of the range, with no drag
    /// needed.
    ///
    /// 280 is the last pixel that reaches it: the knob's centre travels 160
    /// pixels, and 280 is where that run ends.
    #[test]
    fn pressing_the_end_reaches_the_end() {
        let mut s = stepped();

        s.update(&[down(280.0, 120.0)]);
        assert_eq!(s.value(), 1.0);
    }

    /// And the other end too: the knob comes all the way back.
    #[test]
    fn pressing_the_start_reaches_the_start() {
        let mut s = stepped();

        s.update(&[down(200.0, 120.0)]);
        assert_eq!(s.value(), 0.5);

        s.update(&[down(100.0, 120.0)]);
        assert_eq!(s.value(), 0.0);
    }

    /// A drag that runs off the track still moves the knob, all the way to the
    /// end -- unlike a button, which stops so that letting go cancels.
    #[test]
    fn dragging_past_the_end_clamps_to_the_end() {
        let mut s = stepped();

        s.update(&[down(200.0, 120.0)]);
        s.update(&[motion(900.0, 120.0)]);
        assert_eq!(s.value(), 1.0);
    }

    #[test]
    fn dragging_past_the_start_clamps_to_the_start() {
        let mut s = stepped();

        s.update(&[down(200.0, 120.0)]);
        s.update(&[motion(-500.0, 120.0)]);
        assert_eq!(s.value(), 0.0);
    }

    /// Moving the mouse without a press changes nothing.
    #[test]
    fn moving_without_press_changes_nothing() {
        let mut s = stepped();

        assert!(!s.update(&[motion(250.0, 120.0)]));
        assert_eq!(s.value(), 0.0);
        assert_eq!(s.state(), SliderState::Hover);
    }

    /// The press reports movement only once per real change, so a caller can
    /// use the return value to drive a sound effect without buzzing.
    #[test]
    fn a_held_still_knob_stops_reporting_movement() {
        let mut s = stepped();

        s.update(&[down(200.0, 120.0)]);
        assert!(!s.update(&[motion(200.0, 120.0)]));
    }

    /// Let go, and the drag is over: moving afterwards does not move the knob.
    #[test]
    fn releasing_ends_the_drag() {
        let mut s = stepped();

        s.update(&[down(200.0, 120.0)]);
        s.update(&[up(200.0, 120.0)]);
        assert_eq!(s.state(), SliderState::Hover);

        assert!(!s.update(&[motion(300.0, 120.0)]));
        assert_eq!(s.value(), 0.5);
    }

    /// Alt-tab mid-drag: the knob stops following a pointer nobody can see.
    #[test]
    fn losing_focus_ends_the_drag() {
        let mut s = stepped();

        s.update(&[down(200.0, 120.0)]);
        s.update(&[Event::FocusLost]);
        assert_eq!(s.state(), SliderState::Idle);

        assert!(!s.update(&[motion(300.0, 120.0)]));
        assert_eq!(s.value(), 0.5);
    }

    /// The right button belongs to the game, not to the widget.
    #[test]
    fn the_right_button_does_not_drag() {
        let mut s = stepped();

        s.update(&[Event::MouseButtonDown {
            button: MouseButton::Right,
            x: 300.0,
            y: 120.0,
            clicks: 1,
        }]);
        assert_eq!(s.state(), SliderState::Idle);
        assert_eq!(s.value(), 0.0);
    }

    /// A press outside the track does not grab the knob either, so dragging
    /// across a slider in the game does not hijack it.
    #[test]
    fn pressing_outside_does_not_grab_the_knob() {
        let mut s = stepped();

        s.update(&[down(OUT.0, OUT.1)]);
        assert_eq!(s.state(), SliderState::Idle);

        assert!(!s.update(&[motion(300.0, 120.0)]));
        assert_eq!(s.value(), 0.0);
    }

    /// A frame may carry a whole batch of events, and the drag still works.
    #[test]
    fn a_drag_split_across_the_batch_registers() {
        let mut s = stepped();

        let frame = vec![
            motion(200.0, 120.0),
            down(200.0, 120.0),
            motion(300.0, 120.0),
        ];
        assert!(s.update(&frame));
        assert_eq!(s.value(), 1.0);
    }

    /// The value lands on a rung of the step grid, never between two of them.
    ///
    /// 253 px is 83% of the way along a 200 px track, which lands between rungs
    /// at 0.05 -- so the knob must have gone to 0.85, not to 0.83.
    #[test]
    fn stepping_snaps_to_the_grid() {
        let mut s = stepped();

        s.update(&[down(253.0, 120.0)]);

        assert_eq!(s.value(), 0.85);
        assert_eq!((s.value() / 0.05).round(), s.value() / 0.05);
    }

    /// A step of zero leaves the value continuous, which is what a volume
    /// slider wants.
    #[test]
    fn a_zero_step_is_continuous() {
        let mut s = slider().with_range(0.0, 1.0, 0.0);

        s.update(&[down(237.0, 120.0)]);
        assert!((s.value() - 0.73125).abs() < 1e-5);
    }

    /// Out-of-range numbers are pulled back in rather than accepted, so the
    /// knob can never sit somewhere the value is not.
    #[test]
    fn setting_a_value_clamps_it() {
        let mut s = slider().with_range(0.0, 10.0, 0.0);

        s.set_value(-5.0);
        assert_eq!(s.value(), 0.0);

        s.set_value(99.0);
        assert_eq!(s.value(), 10.0);
    }

    /// Narrowing the range drags the value back with it.
    #[test]
    fn narrowing_the_range_pulls_the_value_in() {
        let mut s = slider().with_range(0.0, 100.0, 0.0);
        s.set_value(80.0);

        let s = s.with_range(0.0, 10.0, 0.0);
        assert_eq!(s.value(), 10.0);
    }

    /// `fraction` is the value over a different range, same knob.
    #[test]
    fn fraction_normalises_the_range() {
        let mut s = slider().with_range(10.0, 20.0, 0.0);
        s.set_value(15.0);

        assert_eq!(s.fraction(), 0.5);
    }

    /// An empty range has no midpoint to divide by.
    #[test]
    fn an_empty_range_reports_zero() {
        let s = slider().with_range(5.0, 5.0, 0.0);
        assert_eq!(s.fraction(), 0.0);
        assert_eq!(s.value(), 5.0);
    }

    /// A track narrower than its own knob still works: it just does not move,
    /// rather than sliding the knob off into negative coordinates.
    #[test]
    fn a_track_narrower_than_its_knob_does_not_move() {
        let mut s = stepped().with_rect(Rect::new(100.0, 100.0, 10.0, 40.0));

        s.update(&[down(105.0, 120.0)]);
        assert_eq!(s.value(), 0.0);
    }

    #[test]
    fn the_rect_is_kept_as_given() {
        let rect = Rect::new(12.0, 34.0, 560.0, 24.0);
        assert_eq!(Slider::new(rect).rect(), rect);
    }

    /// The knob is a square as tall as the track, which is also what makes the
    /// travel shorter than the track.
    #[test]
    fn the_knob_fills_the_track_height() {
        let s = slider().with_range(0.0, 100.0, 0.0);
        assert_eq!(s.knob_side(), 40.0);
    }

    /// A custom palette does not change what counts as a move.
    #[test]
    fn colours_do_not_affect_the_value() {
        let mut s = stepped().with_colors(Color::BLACK, Color::WHITE, Color::RED, Color::BLUE);

        assert!(s.update(&[down(280.0, 120.0)]));
        assert_eq!(s.value(), 1.0);
    }

    /// Replaces the track box, for the test that needs a track thinner than its
    /// own knob.
    impl Slider {
        fn with_rect(mut self, rect: Rect) -> Self {
            self.rect = rect;
            self
        }
    }
}
