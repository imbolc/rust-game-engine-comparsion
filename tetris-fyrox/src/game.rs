pub const WIDTH: usize = 10;
pub const HEIGHT: usize = 20;

// Coordinates start at the top left; colors match these seven shape indices.
const SHAPES: [[(i32, i32); 4]; 7] = [
    [(0, 1), (1, 1), (2, 1), (3, 1)], // I
    [(1, 0), (2, 0), (1, 1), (2, 1)], // O
    [(1, 0), (0, 1), (1, 1), (2, 1)], // T
    [(1, 0), (2, 0), (0, 1), (1, 1)], // S
    [(0, 0), (1, 0), (1, 1), (2, 1)], // Z
    [(0, 0), (0, 1), (1, 1), (2, 1)], // J
    [(2, 0), (0, 1), (1, 1), (2, 1)], // L
];

#[derive(Clone, Copy)]
struct Piece {
    kind: usize,
    cells: [(i32, i32); 4],
    x: i32,
    y: i32,
}

impl Piece {
    fn new(kind: usize) -> Self {
        Self {
            kind,
            cells: SHAPES[kind],
            x: WIDTH as i32 / 2 - 2,
            y: 0,
        }
    }

    fn blocks(self) -> [(i32, i32); 4] {
        self.cells.map(|(x, y)| (x + self.x, y + self.y))
    }
}

pub struct Game {
    pub board: [[u8; WIDTH]; HEIGHT],
    pub score: u32,
    pub over: bool,
    active: Piece,
    bag: [usize; 7],
    remaining: usize,
    random: u64,
}

impl Game {
    pub fn new(seed: u64) -> Self {
        let mut game = Self {
            board: [[0; WIDTH]; HEIGHT],
            score: 0,
            over: false,
            active: Piece::new(0),
            bag: [0, 1, 2, 3, 4, 5, 6],
            remaining: 0,
            random: seed,
        };
        game.spawn();
        game
    }

    pub fn blocks(&self) -> [(i32, i32); 4] {
        self.active.blocks()
    }

    pub fn color(&self) -> u8 {
        self.active.kind as u8 + 1
    }

    pub fn move_piece(&mut self, dx: i32) -> bool {
        let mut next = self.active;
        next.x += dx;
        self.try_piece(next)
    }

    pub fn rotate(&mut self) -> bool {
        if self.over {
            return false;
        }
        if self.active.kind == 1 {
            return true;
        }
        let mut next = self.active;
        let last = if next.kind == 0 { 3 } else { 2 };
        next.cells = next.cells.map(|(x, y)| (last - y, x));
        for dx in [0, -1, 1, -2, 2] {
            let mut kicked = next;
            kicked.x += dx;
            if self.try_piece(kicked) {
                return true;
            }
        }
        false
    }

    /// Advance one row, locking and spawning if the piece has landed.
    /// Returns the number of cleared rows so the caller can play a sound.
    pub fn step(&mut self) -> u32 {
        if self.over {
            return 0;
        }
        let mut next = self.active;
        next.y += 1;
        if self.try_piece(next) { 0 } else { self.lock() }
    }

    pub fn drop(&mut self) -> u32 {
        if self.over {
            return 0;
        }
        loop {
            let mut next = self.active;
            next.y += 1;
            if !self.try_piece(next) {
                return self.lock();
            }
        }
    }

    fn fits(&self, piece: Piece) -> bool {
        piece.blocks().into_iter().all(|(x, y)| {
            x >= 0
                && x < WIDTH as i32
                && y >= 0
                && y < HEIGHT as i32
                && self.board[y as usize][x as usize] == 0
        })
    }

    fn try_piece(&mut self, next: Piece) -> bool {
        if self.over || !self.fits(next) {
            return false;
        }
        self.active = next;
        true
    }

    fn lock(&mut self) -> u32 {
        for (x, y) in self.blocks() {
            self.board[y as usize][x as usize] = self.color();
        }
        let mut kept = HEIGHT;
        for row in (0..HEIGHT).rev() {
            if self.board[row].contains(&0) {
                kept -= 1;
                self.board[kept] = self.board[row];
            }
        }
        self.board[..kept].fill([0; WIDTH]);
        let cleared = kept as u32;
        self.score += cleared * 100;
        self.spawn();
        cleared
    }

    fn spawn(&mut self) {
        if self.remaining == 0 {
            for i in (1..self.bag.len()).rev() {
                self.random = self
                    .random
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let j = (self.random >> 32) as usize % (i + 1);
                self.bag.swap(i, j);
            }
            self.remaining = self.bag.len();
        }
        self.remaining -= 1;
        self.active = Piece::new(self.bag[self.remaining]);
        self.over = !self.fits(self.active);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_shapes_have_four_distinct_cells_and_bags_have_every_shape() {
        for shape in SHAPES {
            for i in 0..shape.len() {
                assert!(!shape[..i].contains(&shape[i]));
            }
        }
        let mut game = Game::new(42);
        for _ in 0..3 {
            let mut colors = Vec::new();
            for _ in 0..7 {
                colors.push(game.color());
                game.spawn();
            }
            colors.sort();
            assert_eq!(colors, [1, 2, 3, 4, 5, 6, 7]);
        }
    }

    #[test]
    fn pieces_cannot_cross_walls_or_occupied_cells() {
        let mut game = Game::new(1);
        game.active = Piece::new(0);
        while game.move_piece(-1) {}
        assert_eq!(game.blocks()[0].0, 0);
        assert!(!game.move_piece(-1));
        game.board[1][4] = 2;
        assert!(!game.move_piece(1));
        assert_eq!(game.blocks()[0].0, 0);
    }

    #[test]
    fn hard_drop_locks_piece_at_the_floor_and_spawns_another() {
        let mut game = Game::new(1);
        game.active = Piece::new(0);
        assert_eq!(game.drop(), 0);
        assert_eq!(game.board[HEIGHT - 1][3..7], [1; 4]);
        assert_eq!(game.board.iter().flatten().filter(|&&c| c != 0).count(), 4);
        assert!(game.blocks().into_iter().all(|(_, y)| y < 4));
    }

    #[test]
    fn completed_rows_disappear_score_increases_and_rows_above_fall() {
        let mut game = Game::new(1);
        game.active = Piece::new(1);
        for row in &mut game.board[HEIGHT - 2..] {
            row.fill(3);
            row[4] = 0;
            row[5] = 0;
        }
        game.board[HEIGHT - 3][0] = 6;
        assert_eq!(game.drop(), 2);
        assert_eq!(game.score, 200);
        assert_eq!(game.board[HEIGHT - 1][0], 6);
        assert_eq!(game.board.iter().flatten().filter(|&&c| c != 0).count(), 1);
    }

    #[test]
    fn clearing_separated_rows_preserves_surviving_row_order() {
        let mut game = Game::new(1);
        game.active = Piece {
            kind: 0,
            cells: [(0, 0), (0, 1), (0, 2), (0, 3)],
            x: 4,
            y: 0,
        };
        for y in [HEIGHT - 1, HEIGHT - 3] {
            game.board[y].fill(3);
            game.board[y][4] = 0;
        }
        game.board[HEIGHT - 2][0] = 6;
        game.board[HEIGHT - 4][9] = 7;
        game.board[HEIGHT - 5][2] = 2;
        assert_eq!(game.drop(), 2);
        let mut expected = [[0; WIDTH]; HEIGHT];
        expected[HEIGHT - 1][0] = 6;
        expected[HEIGHT - 1][4] = 1;
        expected[HEIGHT - 2][9] = 7;
        expected[HEIGHT - 2][4] = 1;
        expected[HEIGHT - 3][2] = 2;
        assert_eq!(game.board, expected);
        assert_eq!(game.score, 200);
    }

    #[test]
    fn blocked_spawn_ends_game_and_input_stops() {
        let mut game = Game::new(1);
        game.board[0].fill(4);
        game.board[1].fill(4);
        game.spawn();
        assert!(game.over);
        let before = game.board;
        assert!(!game.move_piece(-1));
        assert!(!game.rotate());
        assert_eq!(game.step(), 0);
        assert_eq!(game.drop(), 0);
        assert_eq!(game.board, before);
    }

    #[test]
    fn rotation_kicks_from_a_wall_and_is_rejected_by_a_stack() {
        let mut game = Game::new(1);
        game.active = Piece::new(0);
        assert!(game.rotate());
        while game.move_piece(-1) {}
        assert!(game.rotate());
        assert!(game.blocks().into_iter().all(|(x, _)| x >= 0));
        game.board[0..4].fill([7; WIDTH]);
        let before = game.blocks();
        assert!(!game.rotate());
        assert_eq!(game.blocks(), before);
    }
}
