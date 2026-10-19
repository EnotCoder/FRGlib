//! Меню перед игрой: заголовок и две кнопки.
//!
//! Шрифт тут -- SDL3_ttf, подключённый как dev-зависимость. В библиотеке его
//! нет, и это осознанно: разбор шрифтов стоит двадцать шесть тысяч строк, а
//! крейт не должен заставлять платить за него того, кому текст не нужен.
//! Меню получает готовые картинки, и `Button` ничего о шрифтах не знает.

use frglib::{Button, ButtonState, Color, Draw, Error, Event, Image, Object, Rect};
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
    /// Тот же заголовок, но тёмный: рисуется со сдвигом и даёт тень.
    title_shadow: Image,
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

/// Цвет рамки панели: светлее и тела, и фона, поэтому окно читается как окно.
const PANEL_EDGE: Color = Color::rgb(58, 68, 92);

/// Тонкая линия внутри панели.
const PANEL_INNER: Color = Color::rgb(38, 45, 62);

/// Фаска на кнопке: сверху светлее, снизу темнее.
const BEVEL_LIGHT: Color = Color::rgb(126, 142, 176);
const BEVEL_DARK: Color = Color::rgb(14, 17, 24);

/// Зазор между нижним краем заголовка и верхним краем панели.
const TITLE_GAP: f32 = 22.0;

/// Светлая полоса сверху и тёмная снизу: кнопка перестаёт быть плоским
/// прямоугольником.
///
/// Рисуется **поверх** заливки кнопки, а не под ней, иначе заливка её закроет.
fn bevel(d: &mut Draw<'_>, rect: Rect, hover: bool) -> Result<(), Error> {
    let cx = rect.x + rect.w / 2.0;
    let w = rect.w - 10.0;
    let thickness = 3.0;

    // На наведённой кнопке фаска ярче -- иначе подсветку почти не видать.
    let light = if hover {
        Color::rgb(196, 214, 248)
    } else {
        BEVEL_LIGHT
    };

    d.fill_object(&Object::from_xywh(cx, rect.y + 3.0, w, thickness), light)?;
    d.fill_object(
        &Object::from_xywh(cx, rect.y + rect.h - 3.0, w, thickness),
        BEVEL_DARK,
    )
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
    /// Шрифты разные не по вкусу, а по должности: подписи в кнопках должны
    /// оставаться мелкими, а заголовок обязан быть крупнее панели, иначе меню
    /// читается как служебное окно. Это один и тот же файл, просто два
    /// экземпляра `Font`.
    ///
    /// Кнопки выровнены по центру, но лежат в координатах экрана: если окно
    /// изменят, меню поедет, и это заметно. Поэтому оно и рисуется через
    /// `screen_space`, а не вместе с миром.
    pub fn new(
        label_font: &Font,
        title_font: &Font,
        view: (u32, u32),
    ) -> Result<Menu, Box<dyn std::error::Error>> {
        let (vw, vh) = (view.0 as f32, view.1 as f32);

        let white = SdlColor::RGBA(240, 243, 250, 255);
        let dim = SdlColor::RGBA(132, 143, 168, 255);
        let shadow = SdlColor::RGBA(8, 10, 16, 255);

        let bw = 260.0;
        let bh = 60.0;
        let x = (vw - bw) / 2.0;
        let top = vh / 2.0 - 40.0;

        let play_label = caption(label_font, "Играть", white)?;
        let exit_label = caption(label_font, "Выход", white)?;
        let title = caption(title_font, "FGOTY DASH", white)?;
        let title_shadow = caption(title_font, "FGOTY DASH", shadow)?;
        let hint = caption(label_font, "пробел — прыжок", dim)?;

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
            title_shadow,
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

    /// Рисует меню поверх мира: затемнение, панель, заголовок с тенью, кнопки.
    pub fn draw(&self, d: &mut Draw<'_>, view: (u32, u32)) -> Result<(), Error> {
        let (vw, vh) = (view.0 as f32, view.1 as f32);

        let title_size = self.title.size();
        let title_shadow_size = self.title_shadow.size();
        let hint_size = self.hint.size();

        // Панель вокруг кнопок: шире их на 40, чтобы текст не лип к рамке.
        let play = self.play.rect();
        let exit = self.exit.rect();
        let panel_w = (play.w.max(exit.w)) + 80.0;
        let panel_h = (exit.y + exit.h - play.y) + 100.0;
        let panel_x = (vw - panel_w) / 2.0;
        let panel_y = play.y - 80.0;
        let panel = Rect::new(panel_x, panel_y, panel_w, panel_h);
        let mid = Object::from_xywh(
            panel.x + panel.w / 2.0,
            panel.y + panel.h / 2.0,
            panel.w,
            panel.h,
        );

        // Зазор от заголовка до панели считаем от его настоящей высоты, а не
        // константой: кегль заголовка задаётся шрифтом, и при правке того
        // шрифта висящий в небе заголовок не должен поехать.
        let title_y = panel.y - TITLE_GAP - title_size.1 as f32 / 2.0;
        let hint_y = panel.y + panel.h + 40.0;

        d.screen_space(|d| {
            // Мир под меню живой, и это панель должна читаться поверх любого
            // кадра, а не наоборот: затемнение держим лёгким, иначе шипы и
            // каменный пол под ним просто пропадают.
            d.fill_object(
                &Object::from_xywh(vw / 2.0, vh / 2.0, vw * 2.0, vh * 2.0),
                Color::rgba(6, 8, 12, 132),
            )?;

            // Панель: рамка, тело, тонкая внутренняя линия.
            d.fill_object(
                &Object::from_xywh(mid.position.x, mid.position.y, panel.w + 8.0, panel.h + 8.0),
                PANEL_EDGE,
            )?;
            d.fill_object(&mid, Color::rgba(19, 22, 30, 244))?;
            d.draw_object(
                &Object::from_xywh(
                    mid.position.x,
                    mid.position.y,
                    panel.w - 10.0,
                    panel.h - 10.0,
                ),
                PANEL_INNER,
            )?;

            // Заголовок с тенью: тёмный со сдвигом, светлый поверх.
            d.fill_textured_object(
                &Object::from_xywh(
                    vw / 2.0 + 3.0,
                    title_y + 3.0,
                    title_shadow_size.0 as f32,
                    title_shadow_size.1 as f32,
                ),
                &self.title_shadow,
            )?;
            d.fill_textured_object(
                &Object::from_xywh(vw / 2.0, title_y, title_size.0 as f32, title_size.1 as f32),
                &self.title,
            )?;

            self.play.draw(d, &self.play_label)?;
            self.exit.draw(d, &self.exit_label)?;

            // Фаски поверх заливок -- иначе кнопка снова станет плоской.
            bevel(d, play, self.play.state() != ButtonState::Idle)?;
            bevel(d, exit, self.exit.state() != ButtonState::Idle)?;

            d.fill_textured_object(
                &Object::from_xywh(vw / 2.0, hint_y, hint_size.0 as f32, hint_size.1 as f32),
                &self.hint,
            )
        })
    }
}
