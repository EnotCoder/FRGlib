//! A rectangular button with a caption, clicked on release.

use crate::event::MouseButton;
use crate::graphics::{Color, Image, Object};
use crate::math::Rect;
use crate::window::{Draw, Error};
use crate::Event;

/// How a button looks right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    /// Not under the cursor, not pressed.
    Idle,
    /// Under the cursor.
    Hover,
    /// Held down.
    Pressed,
}

/// A rectangle that does something when it is clicked.
///
/// The button knows where it is and what state it is in, but not what is
/// written on it: [`draw`](Self::draw) is handed the caption. That keeps fonts
/// on the caller's side of the line, since this crate has never made anybody
/// pay for a text parser they did not ask for, and it keeps the click logic
/// testable without a window.
///
/// ```no_run
/// # fn main() -> Result<(), frglib::Error> {
/// # let events: Vec<frglib::Event> = Vec::new();
/// use frglib::{Button, Rect};
///
/// let mut play = Button::new(Rect::new(40.0, 200.0, 180.0, 44.0));
///
/// // ...inside the frame loop:
/// if play.update(&events) {
///     // the click landed on this frame
/// }
/// # Ok(())
/// # }
/// ```
///
/// Coordinates are screen ones. A [`Draw`] carrying a camera would drag a menu
/// around with the world, so [`Button::draw`] takes the camera off for the
/// length of the call and puts it back afterwards.
pub struct Button {
    rect: Rect,
    idle: Color,
    hover: Color,
    pressed: Color,
    state: ButtonState,
}

impl Button {
    /// A button covering `rect`.
    ///
    /// It draws nothing until [`draw`](Self::draw) is given a caption, which
    /// can be an [`Image`] of a word, an icon, or anything else square.
    pub fn new(rect: Rect) -> Self {
        Button {
            rect,
            idle: Color::rgb(48, 54, 70),
            hover: Color::rgb(78, 98, 140),
            pressed: Color::rgb(150, 106, 58),
            state: ButtonState::Idle,
        }
    }

    /// Sets the three background colours.
    pub fn with_colors(mut self, idle: Color, hover: Color, pressed: Color) -> Self {
        self.idle = idle;
        self.hover = hover;
        self.pressed = pressed;
        self
    }

    /// Feeds this frame's events in, and reports whether a click landed.
    ///
    /// The click lands on release rather than on press, and only if the cursor
    /// is still over the button by then. Pressing, dragging away and letting go
    /// elsewhere is what every other interface does; the other way round makes
    /// buttons fire by accident while the player is trying to move something.
    ///
    /// Only the left button counts. Losing the window focus drops any hover or
    /// hold, so a button does not stay lit after an alt-tab.
    pub fn update(&mut self, events: &[Event]) -> bool {
        let mut clicked = false;

        for event in events {
            match event {
                Event::MouseMotion { x, y } => {
                    // A held button keeps its look while the cursor moves off
                    // it, so the finger down is still registered as press.
                    if self.state != ButtonState::Pressed {
                        self.state = if self.rect.contains(*x, *y) {
                            ButtonState::Hover
                        } else {
                            ButtonState::Idle
                        };
                    }
                }

                Event::MouseButtonDown { button, x, y, .. }
                    if *button == MouseButton::Left && self.rect.contains(*x, *y) =>
                {
                    self.state = ButtonState::Pressed;
                }

                Event::MouseButtonUp { button, x, y, .. } if *button == MouseButton::Left => {
                    if self.state == ButtonState::Pressed && self.rect.contains(*x, *y) {
                        clicked = true;
                    }
                    self.state = if self.rect.contains(*x, *y) {
                        ButtonState::Hover
                    } else {
                        ButtonState::Idle
                    };
                }

                // Alt-tab keeps the last mouse position but stops sending
                // anything, so without this a button stays lit, or stuck held,
                // until the pointer happens to move again.
                Event::FocusLost => self.state = ButtonState::Idle,

                _ => {}
            }
        }

        clicked
    }

    /// The state as of the last [`update`](Self::update).
    pub fn state(&self) -> ButtonState {
        self.state
    }

    /// The area the button covers, in screen pixels.
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// Draws the background and the caption, centred on each other.
    ///
    /// The caption keeps its own size, so a `rect` larger than the text leaves
    /// a margin rather than stretching the letters.
    pub fn draw(&self, d: &mut Draw<'_>, label: &Image) -> Result<(), Error> {
        let background = match self.state {
            ButtonState::Idle => self.idle,
            ButtonState::Hover => self.hover,
            ButtonState::Pressed => self.pressed,
        };

        let (lw, lh) = label.size();
        let cx = self.rect.x + self.rect.w / 2.0;
        let cy = self.rect.y + self.rect.h / 2.0;

        d.screen_space(|d| {
            d.fill_object(
                &Object::from_xywh(cx, cy, self.rect.w, self.rect.h),
                background,
            )?;

            d.fill_textured_object(&Object::from_xywh(cx, cy, lw as f32, lh as f32), label)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The button under test, in the middle of a notional screen.
    fn button() -> Button {
        Button::new(Rect::new(100.0, 100.0, 100.0, 40.0))
    }

    /// Somewhere inside the button.
    const IN: (f32, f32) = (150.0, 120.0);

    /// Somewhere well outside it.
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

    /// Moving onto the button lights it, moving off puts it out.
    #[test]
    fn hovering_follows_the_cursor() {
        let mut b = button();

        assert!(!b.update(&[motion(IN.0, IN.1)]));
        assert_eq!(b.state(), ButtonState::Hover);

        assert!(!b.update(&[motion(OUT.0, OUT.1)]));
        assert_eq!(b.state(), ButtonState::Idle);
    }

    /// Press and release over the button: a click.
    #[test]
    fn a_press_and_release_over_the_button_clicks() {
        let mut b = button();

        assert!(!b.update(&[down(IN.0, IN.1)]));
        assert_eq!(b.state(), ButtonState::Pressed);
        assert!(b.update(&[up(IN.0, IN.1)]));
    }

    /// Let go somewhere else: not a click, even though the press was ours.
    #[test]
    fn dragging_off_before_release_does_not_click() {
        let mut b = button();

        b.update(&[down(IN.0, IN.1)]);
        assert!(!b.update(&[up(OUT.0, OUT.1)]));
        assert_eq!(b.state(), ButtonState::Idle);
    }

    /// A press that began elsewhere is not ours to finish.
    #[test]
    fn pressing_outside_and_releasing_inside_does_not_click() {
        let mut b = button();

        b.update(&[down(OUT.0, OUT.1)]);
        assert!(!b.update(&[up(IN.0, IN.1)]));
    }

    #[test]
    fn pressing_outside_does_not_even_look_pressed() {
        let mut b = button();

        b.update(&[down(OUT.0, OUT.1)]);
        assert_eq!(b.state(), ButtonState::Idle);
    }

    /// Alt-tab, and the button is neither lit nor stuck held.
    #[test]
    fn losing_focus_clears_the_state() {
        let mut b = button();

        b.update(&[down(IN.0, IN.1)]);
        assert_eq!(b.state(), ButtonState::Pressed);

        b.update(&[Event::FocusLost]);
        assert_eq!(b.state(), ButtonState::Idle);
    }

    /// The right button belongs to the game, not to the widget.
    #[test]
    fn the_right_button_does_not_press() {
        let mut b = button();

        b.update(&[Event::MouseButtonDown {
            button: MouseButton::Right,
            x: IN.0,
            y: IN.1,
            clicks: 1,
        }]);
        assert_eq!(b.state(), ButtonState::Idle);
    }

    /// A frame may carry a whole batch of events, and the click still comes
    /// out exactly once.
    #[test]
    fn a_click_split_across_the_batch_registers() {
        let mut b = button();
        let frame = vec![motion(IN.0, IN.1), down(IN.0, IN.1), up(IN.0, IN.1)];

        assert!(b.update(&frame));
    }

    /// Holding the button down and moving off leaves it pressed, because the
    /// press was registered and has not been released yet.
    #[test]
    fn moving_off_while_held_keeps_it_pressed() {
        let mut b = button();

        b.update(&[down(IN.0, IN.1)]);
        b.update(&[motion(OUT.0, OUT.1)]);

        assert_eq!(b.state(), ButtonState::Pressed);
    }

    #[test]
    fn the_rect_is_kept_as_given() {
        let rect = Rect::new(12.0, 34.0, 56.0, 78.0);

        assert_eq!(Button::new(rect).rect(), rect);
    }

    /// A custom palette does not change what counts as a click.
    #[test]
    fn colours_do_not_affect_the_click() {
        let mut b = button().with_colors(Color::BLACK, Color::WHITE, Color::RED);

        assert!(b.update(&[down(IN.0, IN.1), up(IN.0, IN.1)]));
    }
}
