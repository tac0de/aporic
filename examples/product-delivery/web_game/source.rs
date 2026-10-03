// Illustrative Rust state model; this file is not a built product.
#[derive(Clone, Copy)]
struct Game { score: u32, paused: bool, visible: bool }
impl Game {
    fn tick(&mut self, points: u32) {
        if self.visible && !self.paused { self.score = self.score.saturating_add(points); }
    }
    fn visibility(&mut self, visible: bool) { self.visible = visible; self.paused = true; }
}
