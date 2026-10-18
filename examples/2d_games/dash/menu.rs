//! Меню перед игрой: заголовок и две кнопки.
//!
//! Шрифт тут -- SDL3_ttf, подключённый как dev-зависимость. В библиотеке его
//! нет, и это осознанно: разбор шрифтов стоит двадцать шесть тысяч строк, а
//! крейт не должен заставлять платить за него того, кому текст не нужен.
//! Меню получает готовые картинки, и `Button` ничего о шрифтах не знает.

use frglib::{Button, Color, Draw, Error, Event, Image, Object, Rect};
use sdl3::pixels::Color as SdlColor;
use sdl3::ttf::Font;

/// Что нажали.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    /// Ничего.
    None,
    /// Начать игру.
    Play,
    /// Выйти из программы.
    Exit,
}

/// Меню целиком: картинки, кнопки и заголовок.
pub struct Menu {
    play: Button,
    exit: Button,
    play_label: Image,
    exit_label: Image,
    title: Image,
    hint: Image,
    /// Размер окна, под который кнопки уже расставлены.
    ///
    /// Нужен, чтобы [`layout`](Self::layout) не пересобирал кнопки заново на
    /// каждом кадре. `Button::new` начинает с состояния `Idle`, так что
    /// пересборка каждый кадр стирала бы наведение -- кнопка моргала бы, --
    /// и, что хуже, стирала бы `Pressed` до того, как придёт `MouseButtonUp`.
    /// Нажатие перестало бы доходить вовсе.
    laid_out_for: (u32, u32),
}

/// Рисует строку в картинку, готовую для `Button` или для вывода на экран.
fn caption(font: &Font, text: &str, color: SdlColor) -> Result<Image, Box<dyn std::error::Error>> {
    // Размер берётся при загрузке шрифта, поэтому в аргументах его нет.
    // `solid`, а не `blended`: пиксель в пиксель, без сглаживания.
    Ok(Image::from_surface(font.render(text).solid(color)?))
}

impl Menu {
    /// Собирает меню под окно `view`.
    ///
    /// Кнопки выровнены по центру, но лежат в координатах экрана: если окно
    /// изменят, меню поедет, и это заметно. Поэтому оно и рисуется через
    /// `screen_space`, а не вместе с миром.
    pub fn new(font: &Font, view: (u32, u32)) -> Result<Menu, Box<dyn std::error::Error>> {
        let (vw, vh) = (view.0 as f32, view.1 as f32);

        let white = SdlColor::RGBA(240, 243, 250, 255);
        let dim = SdlColor::RGBA(132, 143, 168, 255);

        let bw = 260.0;
        let bh = 60.0;
        let x = (vw - bw) / 2.0;
        let top = vh / 2.0 - 40.0;

        let play_label = caption(font, "Играть", white)?;
        let exit_label = caption(font, "Выход", white)?;
        let title = caption(font, "FROGY DASH", white)?;
        let hint = caption(font, "пробел — прыжок", dim)?;

        Ok(Menu {
            play: Button::new(Rect::new(x, top, bw, bh)),
            exit: Button::new(Rect::new(x, top + bh + 24.0, bw, bh)).with_colors(
                Color::rgb(66, 34, 40),
                Color::rgb(118, 52, 60),
                Color::rgb(178, 72, 80),
            ),
            play_label,
            exit_label,
            title,
            hint,
            laid_out_for: view,
        })
    }

    /// Пересобирает раскладку, но только если размер окна действительно
    /// изменился.
    ///
    /// Раньше это вызывалось каждый кадр, и кнопки пересоздавались заново --
    /// вместе со своим состоянием. Теперь при неизменном окне метод ничего не
    /// делает, и наведение с нажатием живут между кадрами.
    pub fn layout(&mut self, view: (u32, u32)) {
        if self.laid_out_for == view {
            return;
        }

        let (vw, vh) = (view.0 as f32, view.1 as f32);
        let bw = 260.0;
        let bh = 60.0;
        let x = (vw - bw) / 2.0;
        let top = vh / 2.0 - 40.0;

        self.play = Button::new(Rect::new(x, top, bw, bh));
        self.exit = Button::new(Rect::new(x, top + bh + 24.0, bw, bh)).with_colors(
            Color::rgb(66, 34, 40),
            Color::rgb(118, 52, 60),
            Color::rgb(178, 72, 80),
        );

        self.laid_out_for = view;
    }

    /// Кормит события и говорит, что нажали.
    ///
    /// Порядок важен: сначала «Выход». Если бы он проверялся вторым, кадр
    /// пересчитывал бы обе, и результат зависел бы от того, как написан
    /// `if`.
    pub fn update(&mut self, events: &[Event]) -> Choice {
        if self.exit.update(events) {
            return Choice::Exit;
        }
        if self.play.update(events) {
            return Choice::Play;
        }
        Choice::None
    }

    /// Рисует меню поверх всего: сначала затемнение, потом заголовок, потом
    /// кнопки.
    pub fn draw(&self, d: &mut Draw<'_>, view: (u32, u32)) -> Result<(), Error> {
        let (vw, vh) = (view.0 as f32, view.1 as f32);

        let title_size = self.title.size();
        let hint_size = self.hint.size();

        d.screen_space(|d| {
            // Затемнение, чтобы меню читалось поверх любого кадра игры.
            d.fill_object(
                &Object::from_xywh(vw / 2.0, vh / 2.0, vw * 2.0, vh * 2.0),
                Color::rgba(8, 10, 14, 220),
            )?;

            d.fill_textured_object(
                &Object::from_xywh(
                    vw / 2.0,
                    vh / 2.0 - 150.0,
                    title_size.0 as f32,
                    title_size.1 as f32,
                ),
                &self.title,
            )?;

            d.fill_textured_object(
                &Object::from_xywh(
                    vw / 2.0,
                    vh / 2.0 + 200.0,
                    hint_size.0 as f32,
                    hint_size.1 as f32,
                ),
                &self.hint,
            )?;

            self.play.draw(d, &self.play_label)?;
            self.exit.draw(d, &self.exit_label)
        })
    }
}
