use std::collections::{hash_map::Entry, BTreeSet, HashMap};

use crate::transition_table::{State, Transition, TransitionTable};

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
    pub empty: bool,
}

impl Nfa {
    pub fn empty() -> Self {
        Self {
            transitions: HashMap::new(),
            empty: true,
        }
    }

    pub fn new(edge: Transition, modifier: Option<TransitionModifier>) -> Self {
        let mut ret = Self {
            transitions: HashMap::from([(
                State::Start,
                HashMap::from([(edge, vec![State::Accepting])]),
            )]),
            empty: false,
        };

        ret.add_modifier(modifier);

        ret
    }

    pub fn add_modifier(&mut self, modifier: Option<TransitionModifier>) {
        match modifier {
            None => {}
            Some(TransitionModifier::Star) => {
                let final_state = State::new();
                self.set_accepting_state(final_state);

                self.transitions.add_transition(
                    State::Start,
                    Transition::Epsilon,
                    State::Accepting,
                );

                self.transitions
                    .add_transition(final_state, Transition::Epsilon, State::Start);
            }

            Some(TransitionModifier::Plus) => {
                let mut new_nfa = self.clone();
                new_nfa.add_modifier(Some(TransitionModifier::Star));
                self.concat(&mut new_nfa);
            }

            Some(TransitionModifier::Question) => {
                self.transitions.add_transition(
                    State::Start,
                    Transition::Epsilon,
                    State::Accepting,
                );
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

        if let Entry::Vacant(e) = self.transitions.entry(new_state) {
            e.insert(old_trans);
        } else {
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
        for (start, map) in other.transitions.iter() {
            for (transition, states) in map.iter() {
                for state in states {
                    self.transitions.add_transition(*start, *transition, *state);
                }
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

        while let Some(t) = stack.pop() {
            if let Some(trans) = self.transitions.get(&t)
                && let Some(epsilon_trans) = trans.get(&Transition::Epsilon)
            {
                for eps in epsilon_trans {
                    if !ret.contains(eps) {
                        ret.insert(*eps);
                        stack.push(*eps);
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
                            start.dot_node(),
                            end.dot_node(),
                            transition.dot_label()
                        )
                        .as_str(),
                    );
                }
            }
        }

        format!("digraph nfa {{\ngraph [label=\"NFA\"];\n{out}}}")
    }
}
