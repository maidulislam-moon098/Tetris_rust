use macroquad::prelude::*;
use macroquad::window::Conf;
use std::collections::VecDeque;

const BOARD_WIDTH: usize = 25;
const BOARD_HEIGHT: usize = 40;
const BLOCK_SIZE: f32 = 15.0;
const BLOCK_SPACING: f32 = 1.5; 
const BOARD_OFFSET_X: f32 = 35.0;
const BOARD_OFFSET_Y: f32 = 50.0;

#[derive(Clone, Copy, PartialEq)]
enum Cell {
    Empty,
    Filled(PieceType),
    Ghost,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum PieceType {
    I, O, T, S, Z, J, L
}

#[derive(Clone)]
struct Piece {
    piece_type: PieceType,
    x: i32,
    y: i32,
    rotation: usize,
}

struct Game {
    board: [[Cell; BOARD_WIDTH]; BOARD_HEIGHT],
    current_piece: Option<Piece>,
    hold_piece: Option<PieceType>,
    can_hold: bool,
    next_pieces: VecDeque<PieceType>,
    score: u64,
    level: u32,
    lines_cleared: u32,
    game_over: bool,
    last_fall: f64,
    fall_timer: f64,
    lock_delay: Option<f64>,
    lock_delay_duration: f64,
    last_input_time: f64,
    input_repeat_delay: f64,
    paused: bool,
}

impl Game {
    fn new() -> Self {
        let mut game = Game {
            board: [[Cell::Empty; BOARD_WIDTH]; BOARD_HEIGHT],
            current_piece: None,
            hold_piece: None,
            can_hold: true,
            next_pieces: VecDeque::new(),
            score: 0,
            level: 1,
            lines_cleared: 0,
            game_over: false,
            last_fall: 0.0,
            fall_timer: 0.0,
            lock_delay: None,
            lock_delay_duration: 0.5,
            last_input_time: 0.0,
            input_repeat_delay: 0.15,
            paused: false,
        };
        game.refill_piece_bag();
        game.spawn_piece();
        game
    }
    fn refill_piece_bag(&mut self) {
        use PieceType::*;
        let mut bag = vec![I, O, T, S, Z, J, L];
        
        for i in (1..bag.len()).rev() {
            let j = rand::gen_range(0, i + 1); 
            bag.swap(i, j);
        }
        
        for piece in bag {
            self.next_pieces.push_back(piece);
        }
    }

    fn get_piece_shapes(piece_type: PieceType) -> [Vec<(i32, i32)>; 4] {
        use PieceType::*;
        match piece_type {
            I => [
                vec![(0,1), (1,1), (2,1), (3,1)],
                vec![(2,0), (2,1), (2,2), (2,3)],
                vec![(0,2), (1,2), (2,2), (3,2)],
                vec![(1,0), (1,1), (1,2), (1,3)],
            ],
            O => [
                vec![(1,0), (2,0), (1,1), (2,1)],
                vec![(1,0), (2,0), (1,1), (2,1)],
                vec![(1,0), (2,0), (1,1), (2,1)],
                vec![(1,0), (2,0), (1,1), (2,1)],
            ],
            T => [
                vec![(1,0), (0,1), (1,1), (2,1)],
                vec![(1,0), (1,1), (2,1), (1,2)],
                vec![(0,1), (1,1), (2,1), (1,2)],
                vec![(1,0), (0,1), (1,1), (1,2)],
            ],
            S => [
                vec![(1,0), (2,0), (0,1), (1,1)],
                vec![(1,0), (1,1), (2,1), (2,2)],
                vec![(1,1), (2,1), (0,2), (1,2)],
                vec![(0,0), (0,1), (1,1), (1,2)],
            ],
            Z => [
                vec![(0,0), (1,0), (1,1), (2,1)],
                vec![(2,0), (1,1), (2,1), (1,2)],
                vec![(0,1), (1,1), (1,2), (2,2)],
                vec![(1,0), (0,1), (1,1), (0,2)],
            ],

            J => [
                vec![(0,0), (0,1), (1,1), (2,1)],
                vec![(1,0), (2,0), (1,1), (1,2)],
                vec![(0,1), (1,1), (2,1), (2,2)],
                vec![(1,0), (1,1), (0,2), (1,2)],
            ],

            L => [
                vec![(2,0), (0,1), (1,1), (2,1)],
                vec![(1,0), (1,1), (1,2), (2,2)],
                vec![(0,1), (1,1), (2,1), (0,2)],
                vec![(0,0), (1,0), (1,1), (1,2)],
            ],
        }
    }

    fn get_piece_color(_piece_type: PieceType) -> Color {
        Color::from_rgba(0, 255, 0, 255)
    }

    fn spawn_piece(&mut self) {
        if self.next_pieces.len() < 3 {
            self.refill_piece_bag();
        }
        
        if let Some(piece_type) = self.next_pieces.pop_front() {
            let new_piece = Piece {
                piece_type,
                x: (BOARD_WIDTH / 2) as i32 - 1,
                y: 0,
                rotation: 0,
            };

            if self.is_valid_position(&new_piece) {
                self.current_piece = Some(new_piece);
                self.can_hold = true;
                self.lock_delay = None;
            } else {
                self.game_over = true;
            }
        }
    }

    fn get_piece_shape(&self, piece: &Piece) -> Vec<(i32, i32)> {
        Self::get_piece_shapes(piece.piece_type)[piece.rotation].clone()
    }

    fn is_valid_position(&self, piece: &Piece) -> bool {
        let shape = self.get_piece_shape(piece);
        
        for (dx, dy) in shape {
            let x = piece.x + dx;
            let y = piece.y + dy;
            
            if x < 0 || x >= BOARD_WIDTH as i32 || y >= BOARD_HEIGHT as i32 {
                return false;
            }
            
            if y >= 0 && matches!(self.board[y as usize][x as usize], Cell::Filled(_)) {
                return false;
            }
        }
        true
    }

    fn move_piece(&mut self, dx: i32, dy: i32) -> bool {
        if let Some(piece) = &self.current_piece {
            let mut new_piece = piece.clone();
            new_piece.x += dx;
            new_piece.y += dy;
            
            if self.is_valid_position(&new_piece) {
                self.current_piece = Some(new_piece);
                
                if dy > 0 {
                    self.lock_delay = None;
                }
                
                true
            } else {
                if dy > 0 && self.lock_delay.is_none() {
                    self.lock_delay = Some(get_time());
                }
                false
            }
        } else {
            false
        }
    }

    fn rotate_piece(&mut self, clockwise: bool) -> bool {
        if let Some(piece) = &self.current_piece {
            let mut new_piece = piece.clone();
            
            if clockwise {
                new_piece.rotation = (new_piece.rotation + 1) % 4;
            } else {
                new_piece.rotation = (new_piece.rotation + 3) % 4;
            }
            
            if self.is_valid_position(&new_piece) {
                self.current_piece = Some(new_piece);
                self.lock_delay = None;
                return true;
            }
            let kicks = vec![(0, 0), (-1, 0), (1, 0), (0, -1), (-1, -1), (1, -1)];
            for (kick_x, kick_y) in kicks {
                new_piece.x = piece.x + kick_x;
                new_piece.y = piece.y + kick_y;
                
                if self.is_valid_position(&new_piece) {
                    self.current_piece = Some(new_piece);
                    self.lock_delay = None;
                    return true;
                }
            }
        }
        false
    }

    fn hard_drop(&mut self) {
        if let Some(piece) = &self.current_piece {
            let mut drop_distance = 0;
            let mut test_piece = piece.clone();
            
            while self.is_valid_position(&test_piece) {
                test_piece.y += 1;
                drop_distance += 1;
            }
            
            if drop_distance > 1 {
                drop_distance -= 1;
                self.score += (drop_distance * 2) as u64;
                
                if let Some(current) = &mut self.current_piece {
                    current.y += drop_distance;
                }
            }
            
            self.lock_piece();
        }
    }

    fn hold_piece(&mut self) {
        if !self.can_hold {
            return;
        }
        
        if let Some(current) = &self.current_piece {
            let current_type = current.piece_type;
            
            if let Some(held_type) = self.hold_piece {
                let new_piece = Piece {
                    piece_type: held_type,
                    x: (BOARD_WIDTH / 2) as i32 - 1,
                    y: 0,
                    rotation: 0,
                };
                
                self.current_piece = Some(new_piece);
                self.hold_piece = Some(current_type);
            } else {
                self.hold_piece = Some(current_type);
                self.spawn_piece();
            }
            
            self.can_hold = false;
            self.lock_delay = None;
        }
    }

    fn get_ghost_piece(&self) -> Option<Piece> {
        if let Some(piece) = &self.current_piece {
            let mut ghost = piece.clone();
            
            while self.is_valid_position(&ghost) {
                ghost.y += 1;
            }
            ghost.y -= 1;
            
            Some(ghost)
        } else {
            None
        }
    }

    fn lock_piece(&mut self) {
        if let Some(piece) = &self.current_piece {
            let shape = self.get_piece_shape(piece);
            
            for (dx, dy) in shape {
                let x = (piece.x + dx) as usize;
                let y = (piece.y + dy) as usize;
                
                if y < BOARD_HEIGHT {
                    self.board[y][x] = Cell::Filled(piece.piece_type);
                }
            }
        }
        
        self.current_piece = None;
        self.lock_delay = None;
        self.clear_lines();
        self.spawn_piece();
    }

    fn clear_lines(&mut self) {
        let mut lines_to_clear = Vec::new();
        
        for y in 0..BOARD_HEIGHT {
            if self.board[y].iter().all(|&cell| matches!(cell, Cell::Filled(_))) {
                lines_to_clear.push(y);
            }
        }
        
        for &y in lines_to_clear.iter().rev() {
            for row in (1..=y).rev() {
                self.board[row] = self.board[row - 1];
            }
            self.board[0] = [Cell::Empty; BOARD_WIDTH];
        }
        
        let lines_cleared = lines_to_clear.len() as u32;
        self.lines_cleared += lines_cleared;
        
        let base_score = match lines_cleared {
            1 => 100,
            2 => 300,
            3 => 500,
            4 => 800,
            _ => 0,
        };
        
        self.score += (base_score * self.level as u64) as u64;
        self.level = (self.lines_cleared / 10) + 1;
    }

    fn get_fall_delay(&self) -> f64 {
        let base_delay = 1.0;
        let level_speedup = (self.level - 1) as f64 * 0.05;
        (base_delay - level_speedup).max(0.05)
    }

    fn update(&mut self) {
        if self.game_over || self.paused {
            return;
        }

        let current_time = get_time();
    
        if let Some(lock_time) = self.lock_delay {
            if current_time - lock_time >= self.lock_delay_duration {
                self.lock_piece();
                return;
            }
        }
        
        self.fall_timer += get_frame_time() as f64;
        if self.fall_timer >= self.get_fall_delay() {
            if !self.move_piece(0, 1) && self.lock_delay.is_none() {
                self.lock_delay = Some(current_time);
            }
            self.fall_timer = 0.0;
        }
    }

    fn handle_input(&mut self) {
        let current_time = get_time();
        
        if current_time - self.last_input_time < self.input_repeat_delay {
            return;
        }

        if is_key_pressed(KeyCode::P) {
            self.paused = !self.paused;
        }

        if self.game_over || self.paused {
            return;
        }

        if is_key_pressed(KeyCode::Left) || is_key_down(KeyCode::Left) {
            self.move_piece(-1, 0);
            self.last_input_time = current_time;
        }
        
        if is_key_pressed(KeyCode::Right) || is_key_down(KeyCode::Right) {
            self.move_piece(1, 0);
            self.last_input_time = current_time;
        }
        
        if is_key_pressed(KeyCode::Down) || is_key_down(KeyCode::Down) {
            if !self.move_piece(0, 1) && self.lock_delay.is_none() {
                self.lock_delay = Some(current_time);
            }
            self.score += 1;
            self.last_input_time = current_time;
        }
        
        if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::Z) {
            self.rotate_piece(true);
        }
        
        if is_key_pressed(KeyCode::X) {
            self.rotate_piece(false);
        }
        
        if is_key_pressed(KeyCode::Space) {
            self.hard_drop();
        }
        
        if is_key_pressed(KeyCode::C) {
            self.hold_piece();
        }
    }

    fn create_display_board(&self) -> [[Cell; BOARD_WIDTH]; BOARD_HEIGHT] {
        let mut display = self.board;
        if let Some(ghost) = self.get_ghost_piece() {
            if let Some(current) = &self.current_piece {
                if ghost.y != current.y {
                    let shape = self.get_piece_shape(&ghost);
                    for (dx, dy) in shape {
                        let x = ghost.x + dx;
                        let y = ghost.y + dy;
                        
                        if x >= 0 && x < BOARD_WIDTH as i32 && y >= 0 && y < BOARD_HEIGHT as i32 {
                            if display[y as usize][x as usize] == Cell::Empty {
                                display[y as usize][x as usize] = Cell::Ghost;
                            }
                        }
                    }
                }
            }
        }
        
        if let Some(piece) = &self.current_piece {
            let shape = self.get_piece_shape(piece);
            for (dx, dy) in shape {
                let x = piece.x + dx;
                let y = piece.y + dy;
                
                if x >= 0 && x < BOARD_WIDTH as i32 && y >= 0 && y < BOARD_HEIGHT as i32 {
                    display[y as usize][x as usize] = Cell::Filled(piece.piece_type);
                }
            }
        }
        
        display
    }

    fn render(&self) {
        clear_background(BLACK);
        
        let display = self.create_display_board();
        let bright_green = Color::from_rgba(0, 255, 0, 255);
        let medium_green = Color::from_rgba(0, 200, 0, 255);
        let dark_green = Color::from_rgba(0, 150, 0, 255);
        let ghost_green = Color::from_rgba(0, 100, 0, 100);
        for y in 0..BOARD_HEIGHT {
            for x in 0..BOARD_WIDTH {
                let cell_x = BOARD_OFFSET_X + x as f32 * BLOCK_SIZE;
                let cell_y = BOARD_OFFSET_Y + y as f32 * BLOCK_SIZE;
                match display[y][x] {
                    Cell::Empty => {},
                    Cell::Filled(_piece_type) => {
                        let block_size = BLOCK_SIZE - BLOCK_SPACING;
                        let offset = BLOCK_SPACING / 2.0;
                        draw_rectangle(
                            cell_x + offset, 
                            cell_y + offset, 
                            block_size, 
                            block_size, 
                            bright_green
                        );
                        draw_rectangle_lines(
                            cell_x + offset, 
                            cell_y + offset, 
                            block_size, 
                            block_size, 
                            1.0, 
                            dark_green
                        );
                    },
                    Cell::Ghost => {
                        let block_size = BLOCK_SIZE - BLOCK_SPACING;
                        let offset = BLOCK_SPACING / 2.0;
                        draw_rectangle(
                            cell_x + offset, 
                            cell_y + offset, 
                            block_size, 
                            block_size, 
                            ghost_green
                        );
                    }
                }
            }
        }
    
        let board_width = BOARD_WIDTH as f32 * BLOCK_SIZE;
        let board_height = BOARD_HEIGHT as f32 * BLOCK_SIZE;
        draw_rectangle_lines(BOARD_OFFSET_X - 2.0, BOARD_OFFSET_Y - 2.0, board_width + 4.0, board_height + 4.0, 3.0, bright_green);
        let ui_x = BOARD_OFFSET_X + board_width + 30.0;
        draw_text(&format!("SCORE"), ui_x, 80.0, 20.0, bright_green);
        draw_text(&format!("{}", self.score), ui_x, 100.0, 20.0, medium_green);
        draw_text(&format!("LEVEL"), ui_x, 140.0, 20.0, bright_green);
        draw_text(&format!("{}", self.level), ui_x, 160.0, 20.0, medium_green);
        draw_text(&format!("LINES"), ui_x, 200.0, 20.0, bright_green);
        draw_text(&format!("{}", self.lines_cleared), ui_x, 220.0, 20.0, medium_green);
        draw_text(&format!("HOLD"), ui_x, 280.0, 20.0, bright_green);
        if let Some(hold_type) = self.hold_piece {
            draw_text(&format!("{:?}", hold_type), ui_x, 300.0, 16.0, medium_green);
        }
        draw_text(&format!("NEXT"), ui_x, 360.0, 20.0, bright_green);
        for (i, &next_type) in self.next_pieces.iter().take(4).enumerate() {
            let y_pos = 380.0 + i as f32 * 25.0;
            draw_text(&format!("{:?}", next_type), ui_x, y_pos, 16.0, medium_green);
        }
        draw_text(&format!("CONTROLS"), ui_x, 520.0, 16.0, bright_green);
        draw_text(&format!("←→: Move"), ui_x, 540.0, 12.0, dark_green);
        draw_text(&format!("↓: Soft Drop"), ui_x, 555.0, 12.0, dark_green);
        draw_text(&format!("↑/Z: Rotate"), ui_x, 570.0, 12.0, dark_green);
        draw_text(&format!("Space: Hard Drop"), ui_x, 585.0, 12.0, dark_green);
        draw_text(&format!("C: Hold"), ui_x, 600.0, 12.0, dark_green);
        draw_text(&format!("P: Pause"), ui_x, 615.0, 12.0, dark_green);
        if self.paused {
            draw_text("PAUSED", BOARD_OFFSET_X + 30.0, BOARD_OFFSET_Y + board_height / 2.0, 40.0, bright_green);
            draw_text("Press P to resume", BOARD_OFFSET_X + 10.0, BOARD_OFFSET_Y + board_height / 2.0 + 50.0, 20.0, medium_green);
        }
        
        if self.game_over {
            draw_text("GAME OVER", BOARD_OFFSET_X + 20.0, BOARD_OFFSET_Y + board_height / 2.0, 40.0, Color::from_rgba(255, 0, 0, 255));
            draw_text(&format!("Final Score: {}", self.score), BOARD_OFFSET_X + 10.0, BOARD_OFFSET_Y + board_height / 2.0 + 50.0, 20.0, bright_green);
        }
        draw_text("TETRIS", 20.0, 30.0, 30.0, bright_green);
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Tetris".to_owned(),
        window_width: 580,
        window_height: 680,
        window_resizable: false,
        fullscreen: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let seed = (get_time() * 1000000.0) as u64;
    rand::srand(seed);
    let mut game = Game::new();
    loop {
        game.handle_input();
        game.update();
        game.render();
        
        next_frame().await
    }
}

