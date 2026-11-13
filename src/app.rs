//! The practice session: its state, and how it responds to keys.
//!
//! Nothing here draws anything; see `ui` for that.

use rand::rngs::StdRng;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{generate::AddSub, problem::Problem};

/// The longest answer the input accepts. Any answer we ask for fits easily.
pub const MAX_INPUT: usize = 12;

pub struct App {
    settings: AddSub,
    rng: StdRng,
    pub problem: Problem,
    pub input: String,
    pub score: u32,
    pub quit: bool,
}

impl App {
    pub fn new(settings: AddSub, mut rng: StdRng) -> Self {
        let problem = settings.generate(&mut rng);
        Self {
            settings,
            rng,
            problem,
            input: String::new(),
            score: 0,
            quit: false,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if !key.is_press() {
            return;
        }
        match key.code {
            KeyCode::Esc => self.quit = true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => self.quit = true,
            KeyCode::Char(c) => self.type_char(c),
            KeyCode::Backspace => {
                self.input.pop();
            }
            _ => {}
        }
    }

    /// Accepts digits, plus a minus sign at the start. Checks the answer
    /// after every keystroke, so a correct answer moves on without Enter.
    fn type_char(&mut self, c: char) {
        let allowed = c.is_ascii_digit() || (c == '-' && self.input.is_empty());
        if !allowed || self.input.len() >= MAX_INPUT {
            return;
        }
        self.input.push(c);
        if self.input.parse() == Ok(self.problem.answer()) {
            self.score += 1;
            self.input.clear();
            self.problem = self.settings.generate(&mut self.rng);
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng;
    use ratatui::crossterm::event::KeyEventKind;

    use super::*;

    fn new_app() -> App {
        App::new(AddSub::default(), StdRng::seed_from_u64(0))
    }

    fn press(app: &mut App, code: KeyCode) {
        app.handle_key(KeyEvent::from(code));
    }

    fn type_str(app: &mut App, s: &str) {
        s.chars().for_each(|c| press(app, KeyCode::Char(c)));
    }

    #[test]
    fn correct_answer_advances_without_enter() {
        let mut app = new_app();
        let first = app.problem;
        type_str(&mut app, &first.answer().to_string());
        assert_eq!(app.score, 1);
        assert!(app.input.is_empty());
        assert_ne!(
            app.problem, first,
            "seed 0 should not repeat the first problem"
        );
    }

    #[test]
    fn wrong_answer_stays_put() {
        let mut app = new_app();
        let first = app.problem;
        type_str(&mut app, &(first.answer() + 1).to_string());
        assert_eq!(app.score, 0);
        assert_eq!(app.problem, first);
    }

    #[test]
    fn backspace_fixes_a_typo() {
        let mut app = new_app();
        let answer = app.problem.answer().to_string();
        type_str(&mut app, "9999");
        (0..4).for_each(|_| press(&mut app, KeyCode::Backspace));
        type_str(&mut app, &answer);
        assert_eq!(app.score, 1);
    }

    #[test]
    fn ignores_non_answer_characters() {
        let mut app = new_app();
        type_str(&mut app, "a 1-.");
        assert_eq!(app.input, "1");
        type_str(&mut app, &"9".repeat(20));
        assert_eq!(app.input.len(), MAX_INPUT);
    }

    #[test]
    fn minus_only_at_the_start() {
        let mut app = new_app();
        type_str(&mut app, "-");
        assert_eq!(app.input, "-");
    }

    #[test]
    fn esc_and_ctrl_c_quit() {
        let mut app = new_app();
        press(&mut app, KeyCode::Esc);
        assert!(app.quit);

        let mut app = new_app();
        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(app.quit);
        assert!(app.input.is_empty());
    }

    #[test]
    fn ignores_key_releases() {
        let mut app = new_app();
        let mut key = KeyEvent::from(KeyCode::Char('1'));
        key.kind = KeyEventKind::Release;
        app.handle_key(key);
        assert!(app.input.is_empty());
    }
}
