pub const MAX_PLY: u8 = 127;
pub const NUM_THREADS: u8 = 4;

pub mod negamax;
pub mod state;
pub mod see;
pub mod tt;

#[cfg(test)]
mod test {
    use std::time::{Duration, Instant};
    use crate::repr::game::Game;

    #[test]
    fn test_find_best_move_respects_time_limit() {
        let mut game = Game::new_infinite(None, None);
        let time_limit = Duration::from_millis(500);
        let start = Instant::now();
        let (mv, _eval, depth, nodes) = game.find_best_move(None, Some(time_limit), None);
        let elapsed = start.elapsed();
        eprintln!("500ms test: elapsed={:?}, depth={}, nodes={}", elapsed, depth, nodes);
        assert!(mv.is_some(), "expected a move");
        assert!(elapsed < Duration::from_millis(2000), "search took {:?}, way over the 500ms limit", elapsed);
    }

    #[test]
    fn test_find_best_move_depth15() {
        let mut game = Game::new_infinite(None, None);
        let time_limit = Duration::from_millis(6000);
        let start = Instant::now();
        let (mv, _eval, depth, nodes) = game.find_best_move(None, Some(time_limit), None);
        let elapsed = start.elapsed();
        eprintln!("5s test: elapsed={:?}, depth={}, nodes={}", elapsed, depth, nodes);
        assert!(mv.is_some(), "expected a move");
        assert!(elapsed < Duration::from_millis(10000), "search took {:?}, way over the 5s limit", elapsed);
    }

    #[test]
    fn test_find_best_move_depth15_with_stop() {
        use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
        let mut game = Game::new_infinite(None, None);
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = Arc::clone(&stop);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(2000));
            stop2.store(true, Ordering::Relaxed);
        });
        let start = Instant::now();
        let (mv, _eval, depth, nodes) = game.find_best_move(None, None, Some(stop));
        let elapsed = start.elapsed();
        eprintln!("stop-flag test: elapsed={:?}, depth={}, nodes={}", elapsed, depth, nodes);
        assert!(mv.is_some(), "expected a move");
        assert!(elapsed < Duration::from_millis(5000), "search took {:?}, should have stopped within ~2s", elapsed);
    }

    // Regression: helper threads must stop within ~2x time limit
    #[test]
    fn test_short_time_limit_stops_quickly() {
        let mut game = Game::new_infinite(None, None);
        let time_limit = Duration::from_millis(100);
        let start = Instant::now();
        let (mv, _eval, depth, nodes) = game.find_best_move(None, Some(time_limit), None);
        let elapsed = start.elapsed();
        eprintln!("100ms test: elapsed={:?}, depth={}, nodes={}", elapsed, depth, nodes);
        assert!(mv.is_some(), "expected a move");
        assert!(elapsed < Duration::from_millis(500), "search took {:?}, helper threads did not stop in time", elapsed);
    }

    // Regression: back-to-back searches must not panic on Arc::get_mut
    #[test]
    fn test_back_to_back_searches() {
        let mut game = Game::new_infinite(None, None);
        for i in 0..3 {
            let time_limit = Duration::from_millis(200);
            let start = Instant::now();
            let (mv, _eval, depth, nodes) = game.find_best_move(None, Some(time_limit), None);
            let elapsed = start.elapsed();
            eprintln!("search {}: elapsed={:?}, depth={}, nodes={}", i, elapsed, depth, nodes);
            assert!(mv.is_some(), "expected a move for search {}", i);
            assert!(elapsed < Duration::from_millis(1000), "search {} took {:?}", i, elapsed);
        }
    }

    // Regression: external stop flag must be respected promptly
    #[test]
    fn test_external_stop_100ms() {
        use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
        let mut game = Game::new_infinite(None, None);
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = Arc::clone(&stop);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            stop2.store(true, Ordering::Relaxed);
        });
        let start = Instant::now();
        let (mv, _eval, depth, nodes) = game.find_best_move(None, None, Some(stop));
        let elapsed = start.elapsed();
        eprintln!("ext-stop 100ms test: elapsed={:?}, depth={}, nodes={}", elapsed, depth, nodes);
        assert!(mv.is_some(), "expected a move");
        assert!(elapsed < Duration::from_millis(500), "search took {:?}, did not stop promptly", elapsed);
    }
}
