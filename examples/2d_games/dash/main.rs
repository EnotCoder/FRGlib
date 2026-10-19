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

mod level;
mod menu;
mod player;

use std::path::PathBuf;

use frglib::{
    Camera, Color, Draw, Event, FrameClock, Image, Keycode, Object, Texture, Vec2, Window,
};

use level::{hit, parse, top_of, Tile, BODY, COLS, GROUND, GROUND_SCREEN, LEVEL_TOP, ROWS, TILE};
use player::{
    Player, State, GRAVITY, JUMP, LAND_TOLERANCE, MENU_SCROLL, SCREEN_X, SPEED, SPEED_SPIN,
    SPIKE_INSET,
};

fn asset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/2d_games/assets/dash")
        .join(name)
}

/// Ассет, общий для примеров: шрифт лежит один на всех.
fn shared_asset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/assets")
        .join(name)
}

/// Сторона спрайта в пикселях файла.
const SRC: u32 = 16;

/// Высота пола в файле.
const FLOOR_H: u32 = 48;

/// Где в файле кончается каменная шапка и начинается грунт.
const CAP: u32 = 18;

/// Высота каменной шапки на экране.
const CAP_H: f32 = 36.0;

/// Высота одного куска грунта на экране.
const DIRT_H: f32 = 60.0;

/// Небо.
const SKY_TOP: Color = Color::rgb(58, 152, 233);
const SKY_LOW: Color = Color::rgb(191, 233, 252);
const SKY_BANDS: usize = 8;

/// Смешивает два цвета: `0` — `a`, `1` — `b`.
fn mix(a: Color, b: Color, t: f32) -> Color {
    let ch = |x: u8, y: u8| {
        (x as f32 + (y as f32 - x as f32) * t)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color::rgba(ch(a.r, b.r), ch(a.g, b.g), ch(a.b, b.b), 255)
}

/// Текстуры мира, которыми заливается уровень.
///
/// Отдельная структура, а не четыре отдельных аргумента функции: их иначе
/// пришлось бы перечислять в обоих местах, где рисуется мир, и легко
/// перепутать местами.
#[derive(Clone, Copy)]
struct Tiles {
    top: Texture,
    body: Texture,
    spike: Texture,
}

impl Tiles {
    /// Рисует небо, землю и уровень при камере `camera`.
    ///
    /// Куб сюда не входит: у меню мира без игрока, и это позволяет ему ехать
    /// под панелью.
    fn draw(
        &self,
        d: &mut Draw<'_>,
        grid: &[Vec<Tile>],
        camera: Camera,
        view: (u32, u32),
    ) -> std::result::Result<(), frglib::Error> {
        let (vw, vh) = (view.0 as f32, view.1 as f32);

        // Видимый кусок мира: камера стоит в центре окна, поэтому в кадр
        // попадает ровно пол-окна в каждую сторону. Рисовать надо в этих
        // мировых координатах -- `Draw` сам спроецирует их на экран.
        let left = camera.position.x - vw / 2.0;
        let top = camera.position.y - vh / 2.0;
        let bottom = camera.position.y + vh / 2.0;

        // Видимые столбцы -- со знаком: камера спокойно выезжает и за левый
        // край уровня. Земля отсюда не кончается никогда, а шипы и платформы
        // есть только внутри уровня, и их обрезаем отдельно.
        let vis0 = (left / TILE).floor() as isize;
        let vis1 = ((camera.position.x + vw / 2.0) / TILE).ceil() as isize;

        let col0 = vis0.max(0) as usize;
        let col1 = (vis1.min(COLS as isize)).max(0) as usize;
        let row0 = ((top - LEVEL_TOP) / TILE).floor().max(0.0) as usize;
        let row1 = (((bottom - LEVEL_TOP) / TILE).ceil().max(0.0)).min(ROWS as f32) as usize;

        d.set_camera(camera);

        // Небо полосами: дешёвый градиент из одного цвета не выйдет.
        let band = vh / SKY_BANDS as f32;
        for i in 0..SKY_BANDS {
            let t = i as f32 / (SKY_BANDS - 1) as f32;
            d.fill_object(
                &Object::from_xywh(
                    camera.position.x,
                    top + band * (i as f32 + 0.5),
                    vw + TILE,
                    band + 1.0,
                ),
                mix(SKY_TOP, SKY_LOW, t),
            )?;
        }

        // Земля: каменная шапка по верхнему краю, грунт вниз до низа окна.
        for col in vis0..vis1 {
            let x = col as f32 * TILE + TILE / 2.0;
            d.fill_object_texture(
                &Object::from_xywh(x, GROUND + CAP_H / 2.0, TILE, CAP_H),
                self.top,
            )?;
        }

        let mut y = GROUND + CAP_H;
        while y < bottom + DIRT_H {
            for col in vis0..vis1 {
                let x = col as f32 * TILE + TILE / 2.0;
                d.fill_object_texture(
                    &Object::from_xywh(x, y + DIRT_H / 2.0, TILE, DIRT_H),
                    self.body,
                )?;
            }
            y += DIRT_H;
        }

        for (row, cells) in grid.iter().enumerate().take(row1).skip(row0) {
            let y = top_of(row) + TILE / 2.0;

            for (col, tile) in cells.iter().enumerate().take(col1).skip(col0) {
                let cell = Object::from_xywh(col as f32 * TILE + TILE / 2.0, y, TILE, TILE);

                match tile {
                    Tile::Spike => d.fill_object_texture(&cell, self.spike)?,
                    // Платформа -- тот же камень, что и верх земли, но земля
                    // под ней, так что читается как выступ.
                    Tile::Platform => d.fill_object_texture(&cell, self.top)?,
                    Tile::Empty => {}
                }
            }
        }

        Ok(())
    }
}

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
    let tiles = Tiles {
        top: top_tex,
        body: body_tex,
        spike: spike_tex,
    };

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

        match player.state {
            State::Run => {
                if jump && player.grounded {
                    player.vy = -JUMP;
                    player.grounded = false;
                }

                player.vy += GRAVITY * dt;
                player.x += SPEED * dt;

                let previous_bottom = player.y + BODY / 2.0;
                player.y += player.vy * dt;

                // Крутим куб в полёте: у настоящего dash куб вертится.
                if !player.grounded {
                    player.spin += SPEED_SPIN * dt;
                }

                let (col0, col1) = player.columns();
                let half = BODY / 2.0;

                for col in col0.max(0)..=col1.min(COLS as isize - 1) {
                    let row_lo = ((player.y - half - LEVEL_TOP) / TILE).floor() as isize;
                    let row_hi = ((player.y + half - LEVEL_TOP) / TILE).floor() as isize;

                    for row in row_lo.max(0)..=row_hi.min(ROWS as isize - 1) {
                        let col = col as usize;
                        let row = row as usize;

                        match grid[row][col] {
                            Tile::Spike => {
                                let center = Vec2::new(
                                    col as f32 * TILE + TILE / 2.0,
                                    top_of(row) + TILE / 2.0,
                                );
                                let reach = TILE / 2.0 - SPIKE_INSET;
                                let reach = Vec2::new(reach, reach);
                                if hit(player.center(), Vec2::new(half, half), center, reach) {
                                    player.state = State::Dead;
                                    println!("шип. Столбец {col}, пробел — заново.");
                                }
                            }
                            Tile::Platform => {
                                let top = top_of(row);
                                let bottom = top + TILE;
                                let left = col as f32 * TILE;
                                let right = left + TILE;

                                let sideways = player.x + half > left && player.x - half < right;

                                if !sideways {
                                    continue;
                                }

                                let lands = previous_bottom <= top + LAND_TOLERANCE
                                    && player.y + half >= top
                                    && player.vy > 0.0;

                                if lands {
                                    player.y = top - half;
                                    player.vy = 0.0;
                                    player.grounded = true;
                                    player.spin = 0.0;
                                } else if player.y - half < bottom {
                                    // Врезались в бок платформы.
                                    player.state = State::Dead;
                                    println!("бок платформы. Столбец {col}, пробел — заново.");
                                }
                            }
                            Tile::Empty => {}
                        }
                    }
                }

                // Земля: одна бесконечная плита под уровнем.
                if player.y + half >= GROUND {
                    player.y = GROUND - half;
                    player.vy = 0.0;
                    player.grounded = true;
                    player.spin = 0.0;
                }

                if player.state == State::Run && player.x >= COLS as f32 * TILE {
                    player.state = State::Won;
                    println!("уровень пройден: {COLS} столбцов.");
                }
            }
            // Смерть и финиш обрабатываются выше: они возвращают в меню.
            State::Dead | State::Won => {}
        }

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
            let mut body = Object::from_xywh(player.x, player.y, BODY, BODY);
            body.rotation = player.spin;

            if player.state == State::Dead {
                d.fill_object_texture(&body, cube_dead_tex)?;
            } else {
                d.fill_object_texture(&body, cube_tex)?;
            }

            Ok(())
        })?;

        clock.tick();
    }
}
