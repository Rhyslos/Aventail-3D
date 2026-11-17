//! Checks take turns, and a check that never finishes is the last one
//! started until it ends. Adapted from Solis's `check_turns.rs`.
//!
//! The Lust compiler can't be stopped from outside: a check that runs
//! past [`CHECK_LIMIT`] is given up on, and its thread keeps a processor
//! for as long as it runs (for ever, if the compiler never finishes). So
//! only one is ever left behind: while it runs, the next check isn't
//! started, and the person is told why.

use crate::problem::Problem;
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::Duration;
use words::{tr, trf};

/// The longest checking a script may take before it's given up on.
pub const CHECK_LIMIT: Duration = Duration::from_secs(30);

/// Whose turn it is to check.
pub(crate) struct Turns {
    turn: Mutex<Turn>,
    /// Told whenever a turn ends.
    ended: Condvar,
}

enum Turn {
    /// Nothing is being checked.
    Free,
    /// A check is running, and is waited for.
    Taken,
    /// A check was given up on, and its thread may still be running.
    GivenUp(JoinHandle<()>),
}

impl Turns {
    /// Turns with nothing being checked.
    pub(crate) const fn new() -> Self {
        Self {
            turn: Mutex::new(Turn::Free),
            ended: Condvar::new(),
        }
    }

    /// Runs `check` on a thread of its own when its turn comes, and waits
    /// for it at most `limit`. Past that it's given up on, and no check
    /// starts until its thread ends: those get one mistake saying so.
    pub(crate) fn check<T: Send + 'static>(
        &self,
        file: &str,
        limit: Duration,
        check: impl FnOnce() -> Result<T, Vec<Problem>> + Send + 'static,
    ) -> Result<T, Vec<Problem>> {
        let mut turn = self.turn();
        loop {
            match &*turn {
                Turn::Free => break,
                Turn::Taken => {
                    turn = self
                        .ended
                        .wait(turn)
                        .unwrap_or_else(PoisonError::into_inner);
                }
                Turn::GivenUp(worker) if worker.is_finished() => break,
                Turn::GivenUp(_) => {
                    return Err(vec![Problem::whole(
                        file,
                        tr("the script wasn't checked: an earlier check never finished, and is still running. Scripts can be checked again when it ends, or when the app is restarted.").to_string(),
                    )]);
                }
            }
        }
        if let Turn::GivenUp(ended) = std::mem::replace(&mut *turn, Turn::Taken) {
            let _ = ended.join();
        }
        drop(turn);
        let (next, checked) = Self::run(file, limit, check);
        *self.turn() = next;
        self.ended.notify_all();
        checked
    }

    /// One check, from its start to its answer or to giving up on it.
    fn run<T: Send + 'static>(
        file: &str,
        limit: Duration,
        check: impl FnOnce() -> Result<T, Vec<Problem>> + Send + 'static,
    ) -> (Turn, Result<T, Vec<Problem>>) {
        let (sender, answer) = std::sync::mpsc::channel();
        let worker = std::thread::Builder::new()
            .name("script check".into())
            .spawn(move || {
                let _ = sender.send(check());
            });
        let worker = match worker {
            Ok(worker) => worker,
            Err(why) => {
                let message = trf!("the script couldn't be checked: there was no room to start ({}).", why);
                return (Turn::Free, Err(vec![Problem::whole(file, message)]));
            }
        };
        match answer.recv_timeout(limit) {
            Ok(checked) => {
                let _ = worker.join();
                (Turn::Free, checked)
            }
            Err(RecvTimeoutError::Timeout) => {
                let message = trf!(
                    "checking the script took more than {} seconds, so it was given up on. Something in it is more than the checker can handle: undo the last change, or look for something unusual (please report it).",
                    limit.as_secs()
                );
                (
                    Turn::GivenUp(worker),
                    Err(vec![Problem::whole(file, message)]),
                )
            }
            Err(RecvTimeoutError::Disconnected) => {
                let _ = worker.join();
                (Turn::Free, Err(vec![crate::compile::compiler_stopped(file)]))
            }
        }
    }

    fn turn(&self) -> MutexGuard<'_, Turn> {
        self.turn.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_check_that_never_ends_is_given_up_on_and_blocks_the_next_until_it_ends() {
        let turns = Turns::new();
        let (release, wait) = std::sync::mpsc::channel::<()>();
        let stuck = turns.check("a.lua", Duration::from_millis(50), move || {
            let _ = wait.recv();
            Ok(())
        });
        assert!(stuck.unwrap_err()[0].message.contains("given up on"));
        let blocked = turns.check("b.lua", Duration::from_secs(5), || Ok(1));
        assert!(blocked.unwrap_err()[0].message.contains("never finished"));
        release.send(()).unwrap();
        // Once it ends, checking goes on.
        let start = std::time::Instant::now();
        loop {
            match turns.check("b.lua", Duration::from_secs(5), || Ok(2)) {
                Ok(n) => {
                    assert_eq!(n, 2);
                    break;
                }
                Err(_) if start.elapsed() < Duration::from_secs(5) => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(problems) => panic!("{problems:?}"),
            }
        }
    }

    #[test]
    fn a_check_that_panics_is_a_mistake_not_a_crash() {
        let turns = Turns::new();
        let got: Result<(), _> = turns.check("a.lsc", Duration::from_secs(5), || {
            panic!("the compiler broke")
        });
        assert!(got.unwrap_err()[0].message.contains("stopped unexpectedly"));
    }
}
