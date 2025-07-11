use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

static STATE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum State {
    Start,
    Accepting,
    S(u64),
}

impl State {
    fn new() -> Self {
        Self::S(STATE_COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Debug, Clone)]
pub struct Nfa {
    transitions: HashMap<State, HashMap<char, State>>,
    empty: bool,
}

impl Nfa {
    pub fn empty() -> Self {
        Self {
            transitions: HashMap::new(),
            empty: true,
        }
    }

    pub fn new(edge: char) -> Self {
        Nfa {
            transitions: HashMap::from([(State::Start, HashMap::from([(edge, State::Accepting)]))]),
            empty: false,
        }
    }

    pub fn simulate(&self, input: String) -> Result<(), char> {
        let mut curr_state = State::Start;
        for char in input.chars() {
            if let Some(map) = self.transitions.get(&curr_state) {
                if let Some(new_state) = map.get(&char) {
                    curr_state = *new_state;
                } else {
                    return Err(char);
                }
            } else {
                return Err(char);
            }
        }

        if curr_state == State::Accepting {
            return Ok(());
        }

        Err('x')
    }

    pub fn concat(&mut self, other: &mut Self) {
        if self.empty {
            println!("Merging emtpy");
            *self = other.clone();
            return;
        }

        let new_state = State::new();

        // set old accepting state to other's start state
        for start_state in self.transitions.values_mut() {
            for transition in start_state.values_mut() {
                if *transition == State::Accepting {
                    *transition = new_state;
                }
            }
        }

        let mut other = other.to_owned();

        // set other's start to new state
        let old_start = other.transitions.remove(&State::Start).unwrap();
        other.transitions.insert(new_state, old_start);

        for start_state in self.transitions.iter_mut() {
            for transition in start_state.1.iter_mut() {
                if *transition.1 == State::Start {
                    *transition.1 = new_state;
                }
            }
        }

        for transition in other.transitions {
            self.transitions.insert(transition.0, transition.1);
        }
    }
}
