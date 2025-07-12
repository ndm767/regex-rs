use std::collections::{BTreeSet, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};

static STATE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum State {
    Start,
    Accepting,
    S(u64),
}

impl State {
    fn new() -> Self {
        Self::S(STATE_COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    pub fn to_dot_node(&self) -> String {
        match self {
            Self::Start => "start".to_string(),
            Self::Accepting => "accepting".to_string(),
            Self::S(n) => format!("s{n}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Transition {
    Literal(char),
    Wildcard,
    Epsilon, // Empty String
}

impl Transition {
    pub fn to_dot_label(&self) -> String {
        match self {
            Self::Literal(c) => format!("'{c}'"),
            Self::Wildcard => ".".to_string(),
            Self::Epsilon => "ε".to_string(),
        }
    }
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
    pub transitions: HashMap<State, HashMap<Transition, Vec<State>>>,
    empty: bool,
}

impl Nfa {
    pub fn empty() -> Self {
        Self {
            transitions: HashMap::new(),
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
                    HashMap::from([(Transition::Epsilon, vec![State::Accepting])]),
                );
                transitions.insert(
                    final_state,
                    HashMap::from([(Transition::Epsilon, vec![State::Start])]),
                );
            }
            Some(TransitionModifier::Plus) => {
                plus_modifier = true;
            }
            Some(TransitionModifier::Question) => {
                transitions.insert(
                    State::Start,
                    HashMap::from([(Transition::Epsilon, vec![State::Accepting])]),
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
                .insert(edge, vec![final_state]);
        } else {
            transitions.insert(State::Start, HashMap::from([(edge, vec![final_state])]));
        }

        let mut ret = Nfa {
            transitions: transitions,
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

    pub fn add_modifier(&mut self, modifier: Option<TransitionModifier>) {
        match modifier {
            None => {}
            Some(TransitionModifier::Star) => {
                let final_state = State::new();
                self.set_accepting_state(final_state);

                if !self.transitions.contains_key(&State::Start) {
                    self.transitions.insert(State::Start, HashMap::new());
                }

                let map = self.transitions.get_mut(&State::Start).unwrap();
                if !map.contains_key(&Transition::Epsilon) {
                    map.insert(Transition::Epsilon, Vec::new());
                }
                map.get_mut(&Transition::Epsilon)
                    .unwrap()
                    .push(State::Accepting);

                if !self.transitions.contains_key(&final_state) {
                    self.transitions.insert(final_state, HashMap::new());
                }

                let map = self.transitions.get_mut(&final_state).unwrap();
                if !map.contains_key(&Transition::Epsilon) {
                    map.insert(Transition::Epsilon, Vec::new());
                }

                map.get_mut(&Transition::Epsilon)
                    .unwrap()
                    .push(State::Start);
            }

            Some(TransitionModifier::Plus) => {
                let mut new_nfa = self.clone();
                new_nfa.add_modifier(Some(TransitionModifier::Star));
                self.concat(&mut new_nfa);
            }

            Some(TransitionModifier::Question) => {
                if !self.transitions.contains_key(&State::Start) {
                    self.transitions.insert(State::Start, HashMap::new());
                }

                let map = self.transitions.get_mut(&State::Start).unwrap();
                if !map.contains_key(&Transition::Epsilon) {
                    map.insert(Transition::Epsilon, Vec::new());
                }
                map.get_mut(&Transition::Epsilon)
                    .unwrap()
                    .push(State::Accepting);
            }

            Some(TransitionModifier::Range(lower, upper)) => {
                let template = self.clone();
                for i in 1..upper {
                    let mut new_nfa = template.clone();
                    if i >= lower {
                        new_nfa.add_modifier(Some(TransitionModifier::Question));
                    }
                    self.concat(&mut new_nfa);
                }
            }
        }
    }

    // change all transition entries with State::Start to new_start
    fn swap_state(&mut self, old_state: State, new_state: State) {
        let old_trans = self.transitions.remove(&old_state).unwrap();
        if self.transitions.contains_key(&new_state) {
            let row = self.transitions.get_mut(&new_state).unwrap();
            for (k, v) in old_trans.iter() {
                if row.contains_key(k) {
                    let mut new_vals = row.get(k).unwrap().clone();
                    new_vals.append(&mut v.clone());
                    row.insert(*k, new_vals);
                } else {
                    row.insert(*k, v.clone());
                }
            }
        } else {
            self.transitions.insert(new_state, old_trans);
        }

        for map in self.transitions.values_mut() {
            for transition in map.values_mut() {
                for state in transition.iter_mut() {
                    if *state == old_state {
                        *state = new_state;
                    }
                }
            }
        }
    }

    fn set_accepting_state(&mut self, new_state: State) {
        for map in self.transitions.values_mut() {
            for transition in map.values_mut() {
                for state in transition.iter_mut() {
                    if *state == State::Accepting {
                        *state = new_state;
                    }
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
                        let mut new_vals = row.get(transition.0).unwrap().clone();
                        new_vals.append(&mut transition.1.clone());
                        row.insert(*transition.0, new_vals);
                    } else {
                        row.insert(*transition.0, transition.1.clone());
                    }
                }
            } else {
                self.transitions.insert(*entry.0, entry.1.clone());
            }
        }
    }

    pub fn epsilon_closure(&self, states: Vec<State>) -> BTreeSet<State> {
        let mut stack = Vec::new();
        let mut ret = BTreeSet::new();

        for state in states.iter() {
            ret.insert(*state);
            stack.push(*state);
        }

        while stack.len() != 0 {
            let t = stack.pop().unwrap();
            if let Some(trans) = self.transitions.get(&t) {
                if let Some(epsilon_trans) = trans.get(&Transition::Epsilon) {
                    for eps in epsilon_trans {
                        if !ret.contains(eps) {
                            ret.insert(*eps);
                            stack.push(*eps);
                        }
                    }
                }
            }
        }

        ret
    }

    pub fn to_dot(&self) -> String {
        let mut out = String::new();
        for (start, map) in &self.transitions {
            for (transition, states) in map {
                for end in states {
                    out.push_str(
                        format!(
                            "{} -> {} [label = \"{}\"];\n",
                            start.to_dot_node(),
                            end.to_dot_node(),
                            transition.to_dot_label()
                        )
                        .as_str(),
                    );
                }
            }
        }

        format!("digraph nfa {{\n{out}}}")
    }
}
