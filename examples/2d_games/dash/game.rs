//! Обновление игры: прыжок, гравитация и столкновения куба с уровнем.

use crate::level::{top_of, Tile, BODY, COLS, GROUND, LEVEL_TOP, ROWS, TILE};
use crate::player::{Player, State, GRAVITY, JUMP, LAND_TOLERANCE, SPEED, SPEED_SPIN, SPIKE_INSET};

/// Продвигает игрока на один кадр.
///
/// `jump` -- нажата ли сейчас любая из клавиш прыжка. В меню эту функцию не
/// зовут: там мир просто едет, а куб не бежит.
pub fn update(player: &mut Player, grid: &[Vec<Tile>], jump: bool, dt: f32) {
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

            let half = BODY / 2.0;
            let (col0, col1) = (
                ((player.x - half) / TILE).floor() as isize,
                ((player.x + half) / TILE).floor() as isize,
            );
            let half = BODY / 2.0;

            for col in col0.max(0)..=col1.min(COLS as isize - 1) {
                let row_lo = ((player.y - half - LEVEL_TOP) / TILE).floor() as isize;
                let row_hi = ((player.y + half - LEVEL_TOP) / TILE).floor() as isize;

                for row in row_lo.max(0)..=row_hi.min(ROWS as isize - 1) {
                    let col = col as usize;
                    let row = row as usize;

                    match grid[row][col] {
                        Tile::Spike => {
                            let spike_rect = frglib::Rect::new(
                                col as f32 * TILE + SPIKE_INSET,
                                top_of(row) + SPIKE_INSET,
                                TILE - SPIKE_INSET * 2.0,
                                TILE - SPIKE_INSET * 2.0,
                            );
                            let player_rect =
                                frglib::Rect::new(player.x - half, player.y - half, BODY, BODY);
                            if player_rect.overlaps(&spike_rect) {
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
        // Смерть и финиш обрабатываются в main: они возвращают в меню.
        State::Dead | State::Won => {}
    }
}
