//! Fgoty Dash: куб бежит вправо сам, игрок только прыгает.
//!
//! Пробел, стрелка вверх или W -- прыжок. Сила всегда одна: нажал или зажал,
//! одинаково. Шип на пути -- смерть, пробел начинает уровень заново.
//!
//! Как в настоящем Geometry Dash, мир едет влево, а куб держится на одном
//! месте экрана: игрок по горизонтали не управляет совсем, вся задача --
//! угадать момент прыжка.
//!
//! Запуск: `cargo run --features cached-textures --example 2d_games-fgoty-dash`

mod draw;
mod game;
mod level;
mod menu;
mod player;

use std::path::PathBuf;

use frglib::{
    Camera, Event, FrameClock, Image, Keycode, Vec2, Window,
};

use draw::{draw_player, Tiles, SKY_TOP};
use level::{parse, COLS, GROUND, GROUND_SCREEN, TILE};
use player::{Player, State, MENU_SCROLL, SCREEN_X};

fn asset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/2d_games/assets/dash")
        .join(name)
}

/// Ассет, общий для примеров: шрифт лежит один на всех.
fn shared_asset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/2d_games/assets")
        .join(name)
}

/// Сторона спрайта в пикселях файла.
const SRC: u32 = 16;

/// Высота пола в файле.
const FLOOR_H: u32 = 48;

/// Где в файле кончается каменная шапка и начинается грунт.
const CAP: u32 = 18;

/// Что показывать: меню или саму игру.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Menu,
    Play,
}

/// Все три источника ошибок -- sdl3, шрифт и сам frglib -- реализуют
/// `std::error::Error`, так что один ящик вместо собственного типа ошибки на
/// тридцать строк.
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let cube = Image::from_file(asset("fgoty_cube.png"))?;
    let cube_dead = Image::from_file(asset("fgoty_cube_dead.png"))?;
    let spike = Image::from_file(asset("kill_block.png"))?;
    let floor = Image::from_file(asset("floor.png"))?;

    // Пол из картинки 16x48: каменная шапка сверху, грунт под ней.
    //
    // Шапка в файле выше, чем кажется: серая строка 17 ещё серая, грунт
    // начинается с 18-й. Режем по границе цвета, иначе сверху каждого
    // повтора земли идёт каменная полоска.
    let floor_top = floor.crop(0, 0, SRC, CAP)?;
    let floor_body = floor.crop(0, CAP, SRC, FLOOR_H - CAP)?;

    let mut window = Window::builder("fgoty dash")
        .size(960, 540)
        .background(SKY_TOP)
        .pixel_art(true)
        .build()?;

    let cube_tex = window.upload(&cube)?;
    let cube_dead_tex = window.upload(&cube_dead)?;
    let spike_tex = window.upload(&spike)?;
    let top_tex = window.upload(&floor_top)?;
    let body_tex = window.upload(&floor_body)?;

    // Шрифт и меню живут за счёт dev-зависимости SDL3_ttf: в самом крейте её
    // нет. Один шрифт на все примеры -- `examples/assets/font.otf`.
    let ttf = sdl3::ttf::init()?;
    let font = ttf.load_font(shared_asset("font.otf"), 32.0)?;
    // Заголовок крупнее подписей: тот же файл, другой кегль. Один объект
    // `Font` на оба не годится -- размер у него общий на весь объект.
    let title_font = ttf.load_font(shared_asset("font.otf"), 64.0)?;
    let mut menu = menu::Menu::new(&font, &title_font, window.size())?;

    let grid = parse();
    let tiles = Tiles::new(top_tex, body_tex, spike_tex);

    let mut pressed: Vec<Keycode> = Vec::new();
    let mut clock = FrameClock::new(60.0);
    let mut player = Player::new();
    let mut camera = Camera::new(Vec2::ZERO);

    // Игра начинается с меню, а не с бега: иначе первый кадр -- это уже
    // проигрыш на первом шипе.
    let mut screen = Screen::Menu;

    // Камера меню едет сама, чтобы под панелью был живой мир.
    let mut menu_camera = Camera::new(Vec2::ZERO);
    menu_camera.zoom = 1.3;

    // Начинаем не с нуля, а с первой группы шипов: пустое поле под меню
    // выглядит как заставка, а шипы сразу показывают, что это игра.
    // Мировая X шипов -- 832, камера стоит на `scroll - 200`.
    let mut menu_scroll = 1048.0f32;

    loop {
        let dt = clock.dt();

        let events = window.poll()?;

        for event in &events {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),
                Event::KeyDown { key, .. } => {
                    if !pressed.contains(key) {
                        pressed.push(*key);
                    }
                }
                Event::KeyUp { key, .. } => pressed.retain(|&held| held != *key),
                _ => {}
            }
        }

        let held = |key| pressed.contains(&key);
        let jump = [Keycode::Space, Keycode::Up, Keycode::W, Keycode::Z]
            .iter()
            .any(|&key| held(key));

        // Меню забирает мышь себе, пока открыто: игра под ним не должна
        // реагировать на прыжки и не должна получать клики.
        if screen == Screen::Menu {
            menu.layout(window.size());
            match menu.update(&events) {
                menu::Choice::Exit => return Ok(()),
                menu::Choice::Play => {
                    player = Player::new();
                    screen = Screen::Play;
                }
                menu::Choice::None => {}
            }
        } else if player.state != State::Run {
            // Шип или финиш -- возвращаемся в меню, а не крутим рестарт.
            screen = Screen::Menu;
        }

        if screen == Screen::Menu {
            // Размер читаем до `frame_with`: замыкание берёт окно на
            // изменение, а внутри осталось бы обращаться к нему.
            let view = window.size();

            // Мир под меню едет медленно и зацикливается: панель стоит на
            // месте, а под ней видно, что это игра, а не заставка.
            menu_scroll += MENU_SCROLL * dt;
            let span = COLS as f32 * TILE;
            if menu_scroll > span - 600.0 {
                menu_scroll = 0.0;
            }
            menu_camera.position.x = menu_scroll + SCREEN_X;
            // По вертикали камера меню стоит как в игре: земля лежит в мире на
            // `GROUND`, а не на нуле, и без этого она уезжает за верхний край
            // кадра -- под панелью было бы пустое небо.
            menu_camera.position.y = GROUND - view.1 as f32 * (GROUND_SCREEN - 0.5);

            clock.tick();
            window.frame_with(|d| {
                tiles.draw(d, &grid, menu_camera, view)?;
                menu.draw(d, view)?;
                Ok(())
            })?;
            continue;
        }

        game::update(&mut player, &grid, jump, dt);

        // Мир едет влево, потому что игрок едет вправо: камера держит его на
        // одной и той же точке экрана.
        camera.position.x = player.x - SCREEN_X;

        // Земля стоит на трёх четвертях высоты окна, а не на фиксированной
        // мировой координате. Окно тут WM раздувает до 925x991 вместо
        // запрошенных 960x540, и с жёстким GROUND низ уезжал за кадр.
        let view = window.size();
        camera.position.y = GROUND - view.1 as f32 * (GROUND_SCREEN - 0.5);

        window.frame_with(|d| {
            tiles.draw(d, &grid, camera, view)?;

            // Куб. После смерти красим, чтобы было видно без текста: в
            // библиотеке нет шрифтов.
            draw_player(d, &player, cube_tex, cube_dead_tex)?;

            Ok(())
        })?;

        clock.tick();
    }
}
