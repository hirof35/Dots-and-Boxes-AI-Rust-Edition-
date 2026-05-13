use eframe::egui;
use std::time::Instant;

// --- 型定義 ---
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Player { Human, AI }

impl Player {
    fn opponent(self) -> Self {
        match self { Player::Human => Player::AI, Player::AI => Player::Human }
    }
}

#[derive(Clone, PartialEq)]
struct GameState {
    size: usize,
    h_lines: Vec<Vec<Option<Player>>>,
    v_lines: Vec<Vec<Option<Player>>>,
    boxes: Vec<Vec<Option<Player>>>,
    current_turn: Player,
    human_score: i32,
    ai_score: i32,
}

impl GameState {
    fn new(size: usize) -> Self {
        Self {
            size,
            h_lines: vec![vec![None; size]; size + 1],
            v_lines: vec![vec![None; size + 1]; size],
            boxes: vec![vec![None; size]; size],
            current_turn: Player::Human,
            human_score: 0,
            ai_score: 0,
        }
    }

    fn make_move(&mut self, row: usize, col: usize, is_horizontal: bool) -> bool {
        let lines = if is_horizontal { &mut self.h_lines } else { &mut self.v_lines };
        if lines[row][col].is_some() { return false; }
        
        lines[row][col] = Some(self.current_turn);
        let mut box_completed = false;

        let mut check_box = |r: usize, c: usize, state: &mut GameState| {
            if r < state.size && c < state.size {
                if state.h_lines[r][c].is_some() && state.h_lines[r+1][c].is_some() && 
                   state.v_lines[r][c].is_some() && state.v_lines[r][c+1].is_some() {
                    if state.boxes[r][c].is_none() {
                        state.boxes[r][c] = Some(state.current_turn);
                        if state.current_turn == Player::Human { state.human_score += 1; } else { state.ai_score += 1; }
                        box_completed = true;
                    }
                }
            }
        };

        if is_horizontal {
            if row > 0 { check_box(row - 1, col, self); }
            check_box(row, col, self);
        } else {
            if col > 0 { check_box(row, col - 1, self); }
            check_box(row, col, self);
        }

        if !box_completed {
            self.current_turn = self.current_turn.opponent();
        }
        true
    }

    fn is_full(&self) -> bool {
        self.human_score + self.ai_score == (self.size * self.size) as i32
    }

    fn evaluate(&self) -> i32 {
        self.ai_score - self.human_score
    }
}

// --- AI ロジック ---
fn minimax(state: &GameState, depth: usize, mut alpha: i32, mut beta: i32, is_max: bool, nodes: &mut u64) -> i32 {
    *nodes += 1;
    if depth == 0 || state.is_full() {
        return state.evaluate();
    }

    let mut moves = vec![];
    for r in 0..state.size + 1 { for c in 0..state.size { if state.h_lines[r][c].is_none() { moves.push((r, c, true)); } } }
    for r in 0..state.size { for c in 0..state.size + 1 { if state.v_lines[r][c].is_none() { moves.push((r, c, false)); } } }

    if is_max {
        let mut max_eval = i32::MIN;
        for (r, c, is_h) in moves {
            let mut next_state = state.clone();
            next_state.make_move(r, c, is_h);
            let eval = minimax(&next_state, depth - 1, alpha, beta, next_state.current_turn == Player::AI, nodes);
            max_eval = max_eval.max(eval);
            alpha = alpha.max(eval);
            if beta <= alpha { break; }
        }
        max_eval
    } else {
        let mut min_eval = i32::MAX;
        for (r, c, is_h) in moves {
            let mut next_state = state.clone();
            next_state.make_move(r, c, is_h);
            let eval = minimax(&next_state, depth - 1, alpha, beta, next_state.current_turn == Player::AI, nodes);
            min_eval = min_eval.min(eval);
            beta = beta.min(eval);
            if beta <= alpha { break; }
        }
        min_eval
    }
}

// --- GUI アプリケーション ---
struct DotsApp {
    game: GameState,
    history: Vec<GameState>, // Undo用の履歴
    search_depth: usize,
    last_ai_time: u128,
    last_ai_nodes: u64,
}

impl Default for DotsApp {
    fn default() -> Self {
        Self {
            game: GameState::new(3),
            history: Vec::new(),
            search_depth: 4,
            last_ai_time: 0,
            last_ai_nodes: 0,
        }
    }
}

impl eframe::App for DotsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let get_color = |owner: Option<Player>| {
            match owner {
                Some(Player::Human) => egui::Color32::from_rgb(0, 100, 255), // 青
                Some(Player::AI) => egui::Color32::from_rgb(255, 50, 50),    // 赤
                None => egui::Color32::from_gray(50),                       // グレー
            }
        };

        egui::SidePanel::left("panel").show(ctx, |ui| {
            ui.heading("Dots & Boxes AI");
            ui.separator();
            ui.label(format!("スコア: 人間 {} - {} AI", self.game.human_score, self.game.ai_score));
            ui.label(format!("手番: {:?}", self.game.current_turn));
            
            ui.add(egui::Slider::new(&mut self.search_depth, 1..=8).text("探索の深さ"));
            
            ui.separator();
            
            // --- Undo ボタン ---
            let can_undo = !self.history.is_empty() && self.game.current_turn == Player::Human;
            if ui.add_enabled(can_undo, egui::Button::new("⟲ Undo (Ctrl+Z)")).clicked() {
                self.undo();
            }

            if ui.button("♻ リセット").clicked() {
                self.game = GameState::new(3);
                self.history.clear();
            }

            ui.separator();
            ui.label("AI 統計:");
            ui.label(format!("思考時間: {} ms", self.last_ai_time));
            ui.label(format!("探索ノード: {}", self.last_ai_nodes));
            if self.last_ai_time > 0 {
                ui.label(format!("速度: {} n/s", (self.last_ai_nodes as f64 / (self.last_ai_time as f64 / 1000.0)) as u64));
            }
        });

        // Ctrl+Z ショートカット
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Z)) {
            self.undo();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.available_rect_before_wrap();
            let cell_size = rect.width().min(rect.height()) / (self.game.size as f32 + 1.0);
            let offset = rect.min + egui::vec2(cell_size, cell_size);

            // 1. ボックスの塗りつぶし
            for r in 0..self.game.size {
                for c in 0..self.game.size {
                    if let Some(owner) = self.game.boxes[r][c] {
                        let top_left = offset + egui::vec2(c as f32 * cell_size, r as f32 * cell_size);
                        let box_rect = egui::Rect::from_min_size(top_left, egui::vec2(cell_size, cell_size));
                        let color = get_color(Some(owner)).gamma_multiply(0.3);
                        ui.painter().rect_filled(box_rect, 0.0, color);
                    }
                }
            }

            // 2. 線とドット（クリック判定）
            for r in 0..=self.game.size {
                for c in 0..=self.game.size {
                    let pos = offset + egui::vec2(c as f32 * cell_size, r as f32 * cell_size);
                    
                    if c < self.game.size {
                        let line_rect = egui::Rect::from_center_size(pos + egui::vec2(cell_size/2.0, 0.0), egui::vec2(cell_size * 0.8, 8.0));
                        if ui.allocate_rect(line_rect, egui::Sense::click()).clicked() && self.game.current_turn == Player::Human {
                            self.history.push(self.game.clone()); // 移動前に保存
                            self.game.make_move(r, c, true);
                        }
                        ui.painter().rect_filled(line_rect, 2.0, get_color(self.game.h_lines[r][c]));
                    }

                    if r < self.game.size {
                        let line_rect = egui::Rect::from_center_size(pos + egui::vec2(0.0, cell_size/2.0), egui::vec2(8.0, cell_size * 0.8));
                        if ui.allocate_rect(line_rect, egui::Sense::click()).clicked() && self.game.current_turn == Player::Human {
                            self.history.push(self.game.clone()); // 移動前に保存
                            self.game.make_move(r, c, false);
                        }
                        ui.painter().rect_filled(line_rect, 2.0, get_color(self.game.v_lines[r][c]));
                    }
                    ui.painter().circle_filled(pos, 5.0, egui::Color32::YELLOW);
                }
            }

            // 3. AI の思考
            if self.game.current_turn == Player::AI && !self.game.is_full() {
                let start = Instant::now();
                let mut nodes = 0;
                let mut best_score = i32::MIN;
                let mut best_move = None;

                let mut moves = vec![];
                for r in 0..self.game.size + 1 { for c in 0..self.game.size { if self.game.h_lines[r][c].is_none() { moves.push((r, c, true)); } } }
                for r in 0..self.game.size { for c in 0..self.game.size + 1 { if self.game.v_lines[r][c].is_none() { moves.push((r, c, false)); } } }

                for (r, c, is_h) in moves {
                    let mut next_state = self.game.clone();
                    next_state.make_move(r, c, is_h);
                    let score = minimax(&next_state, self.search_depth - 1, i32::MIN, i32::MAX, next_state.current_turn == Player::AI, &mut nodes);
                    if score > best_score {
                        best_score = score;
                        best_move = Some((r, c, is_h));
                    }
                }

                if let Some((r, c, is_h)) = best_move {
                    self.history.push(self.game.clone()); // AIの移動も履歴に入れる
                    self.game.make_move(r, c, is_h);
                }
                self.last_ai_time = start.elapsed().as_millis();
                self.last_ai_nodes = nodes;
                ctx.request_repaint();
            }
        });
    }
}

impl DotsApp {
    fn undo(&mut self) {
        if let Some(prev) = self.history.pop() {
            self.game = prev;
            // AIが動いた直後の自分のターンでUndoする場合、AIの手と自分の手の両方を戻す必要がある
            while !self.history.is_empty() && self.game.current_turn == Player::AI {
                if let Some(prev_inner) = self.history.pop() {
                    self.game = prev_inner;
                }
            }
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native("Dots and Boxes", options, Box::new(|_cc| Box::new(DotsApp::default())))
}
