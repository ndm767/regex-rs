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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Transition {
    Literal(char),
    Wildcard,
    Epsilon, // Empty String
}

#[derive(Debug, Clone, Copy)]
pub enum TransitionModifier {
    Star,
    Plus,
    Question,
    Range(u64, u64),
}

#[derive(Debug, Clone)]
pub struct Nfa {
    transitions: HashMap<State, HashMap<Transition, State>>,
    merge_queue: Vec<(State, State)>,
    empty: bool,
}

impl Nfa {
    pub fn empty() -> Self {
        Self {
            transitions: HashMap::new(),
            merge_queue: Vec::new(),
            empty: true,
        }
    }

    pub fn new(edge: Transition, modifier: Option<TransitionModifier>) -> Self {
        let mut transitions = HashMap::new();

        let mut final_state = State::Accepting;
        let mut plus_modifier = false;
        let mut range: Option<(u64, u64)> = None;

        match modifier {
            Some(TransitionModifier::Star) => {
                // handle star
                final_state = State::new();
                transitions.insert(
                    State::Start,
                    HashMap::from([(Transition::Epsilon, State::Accepting)]),
                );
                transitions.insert(
                    final_state,
                    HashMap::from([(Transition::Epsilon, State::Start)]),
                );
            }
            Some(TransitionModifier::Plus) => {
                plus_modifier = true;
            }
            Some(TransitionModifier::Question) => {
                transitions.insert(
                    State::Start,
                    HashMap::from([(Transition::Epsilon, State::Accepting)]),
                );
            }
            Some(TransitionModifier::Range(mi, ma)) => {
                range = Some((mi, ma));
            }
            _ => {}
        }

        if transitions.contains_key(&State::Start) {
            transitions
                .get_mut(&State::Start)
                .unwrap()
                .insert(edge, final_state);
        } else {
            transitions.insert(State::Start, HashMap::from([(edge, final_state)]));
        }

        let mut ret = Nfa {
            transitions: transitions,
            merge_queue: Vec::new(),
            empty: false,
        };

        if plus_modifier {
            // r+ = rr*
            ret.concat(&mut Nfa::new(edge, Some(TransitionModifier::Star)));
        } else if range.is_some() {
            // r{min,max} = r.(min).rr?.(max-min).r?
            let range = range.unwrap();
            for i in 1..range.1 {
                let modif = if i < range.0 {
                    None
                } else {
                    Some(TransitionModifier::Question)
                };
                ret.concat(&mut Nfa::new(edge, modif));
            }
        }

        ret
    }

    pub fn simulate(&self, input: String) -> Result<(), char> {
        let mut curr_state = State::Start;
        let mut char_iter = input.chars().peekable();

        while curr_state != State::Accepting {
            if let Some(map) = self.transitions.get(&curr_state) {
                if char_iter.peek().is_some() {
                    let char = char_iter.peek().unwrap();
                    if let Some(new_state) = map.get(&Transition::Literal(*char)) {
                        let _ = char_iter.next();
                        curr_state = *new_state;
                    } else if let Some(new_state) = map.get(&Transition::Wildcard) {
                        let _ = char_iter.next();
                        curr_state = *new_state;
                    } else if let Some(new_state) = map.get(&Transition::Epsilon) {
                        curr_state = *new_state;
                    } else {
                        return Err(*char);
                    }
                } else if let Some(new_state) = map.get(&Transition::Epsilon) {
                    curr_state = *new_state;
                } else {
                    return Err('x');
                }
            } else {
                return Err('y');
            }
        }

        if curr_state == State::Accepting && char_iter.peek().is_none() {
            return Ok(());
        }

        Err('z')
    }

    // change all transition entries with State::Start to new_start
    fn swap_state(&mut self, old_state: State, new_state: State) {
        let old_trans = self.transitions.remove(&old_state).unwrap();
        if self.transitions.contains_key(&new_state) {
            let row = self.transitions.get_mut(&new_state).unwrap();
            for (k, v) in old_trans.iter() {
                if row.contains_key(k) {
                    self.merge_queue.push((*row.get(k).unwrap(), *v));
                } else {
                    row.insert(*k, *v);
                }
            }
        } else {
            self.transitions.insert(new_state, old_trans);
        }

        for map in self.transitions.values_mut() {
            for transition in map.values_mut() {
                if *transition == old_state {
                    *transition = new_state;
                }
            }
        }
    }

    fn set_accepting_state(&mut self, new_state: State) {
        for map in self.transitions.values_mut() {
            for transition in map.values_mut() {
                if *transition == State::Accepting {
                    *transition = new_state;
                }
            }
        }
    }

    pub fn concat(&mut self, other: &mut Self) {
        if self.empty {
            *self = other.clone();
            return;
        }

        let new_state = State::new();

        // set old accepting state to other's start state
        self.set_accepting_state(new_state);

        let mut other = other.to_owned();

        // set other's start to new state
        other.swap_state(State::Start, new_state);

        // copy other's transition table over to self
        for transition in other.transitions {
            self.transitions.insert(transition.0, transition.1);
        }
    }

    pub fn union(&mut self, other: &mut Self) {
        for entry in other.transitions.iter() {
            if self.transitions.contains_key(entry.0) {
                // merge tables
                let row = self.transitions.get_mut(entry.0).unwrap();
                for transition in entry.1.iter() {
                    if row.contains_key(transition.0) {
                        self.merge_queue
                            .push((*row.get(transition.0).unwrap(), *transition.1));
                    } else {
                        row.insert(*transition.0, transition.1.clone());
                    }
                }
            } else {
                self.transitions.insert(*entry.0, entry.1.clone());
            }
        }

        while self.merge_queue.len() > 0 {
            let (a, b) = self.merge_queue.pop().unwrap();

            let new_state = State::new();

            self.swap_state(a, new_state);
            self.swap_state(b, new_state);
        }
    }
}
