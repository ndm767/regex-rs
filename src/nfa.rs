use std::collections::{BTreeSet, HashMap};

use crate::{
    parse::ParseElement,
    transition_table::{NfaState, Transition, TransitionTable},
};

#[derive(Debug, Clone)]
pub struct Nfa {
    pub transitions: HashMap<NfaState, HashMap<Transition, Vec<NfaState>>>,
    pub empty: bool,
}

impl Nfa {
    pub fn empty() -> Self {
        Self {
            transitions: HashMap::new(),
            empty: true,
        }
    }

    pub fn new(edge: Transition, modifier: Option<ParseElement>) -> Self {
        let mut ret = Self {
            transitions: HashMap::from([(
                NfaState::Start,
                HashMap::from([(edge, vec![NfaState::Accepting])]),
            )]),
            empty: false,
        };

        ret.add_modifier(modifier);

        ret
    }

    pub fn add_modifier(&mut self, modifier: Option<ParseElement>) {
        match modifier {
            Some(ParseElement::Star) => {
                let final_state = NfaState::new();
                let start_state = NfaState::new();

                // we create new start and final states to avoid issues with unions
                self.transitions.rename(NfaState::Accepting, final_state);
                self.transitions.rename(NfaState::Start, start_state);

                // add epsilon transition from start to finish for 0 instances
                self.transitions.add_transition(
                    start_state,
                    Transition::Epsilon,
                    NfaState::Accepting,
                );

                // add epsilon transition from finish to start for repeated instances
                self.transitions
                    .add_transition(final_state, Transition::Epsilon, start_state);

                // add epsilon transition from true start to the new start
                self.transitions
                    .add_transition(NfaState::Start, Transition::Epsilon, start_state);
            }

            Some(ParseElement::Plus) => {
                // treat x+ as x concatenated with x*
                let mut new_nfa = self.clone();
                new_nfa.add_modifier(Some(ParseElement::Star));
                self.concat(&mut new_nfa);
            }

            Some(ParseElement::Question) => {
                // add epsilon transition from start to finish
                self.transitions.add_transition(
                    NfaState::Start,
                    Transition::Epsilon,
                    NfaState::Accepting,
                );
            }

            Some(ParseElement::Range(lower, upper)) => {
                // repeated concatenation up to lower, then concatenate with ? metacharacter through upper
                let template = self.clone();
                for i in 1..upper {
                    let mut new_nfa = template.clone();
                    if i >= lower {
                        new_nfa.add_modifier(Some(ParseElement::Question));
                    }
                    self.concat(&mut new_nfa);
                }
            }

            Some(ParseElement::OpenRange(start)) => {
                // concatenate start times, with the last getting a *
                let template = self.clone();
                for i in 0..start {
                    let mut new_nfa = template.clone();
                    if i == start - 1 {
                        new_nfa.add_modifier(Some(ParseElement::Star));
                    }
                    self.concat(&mut new_nfa);
                }
            }

            _ => {}
        }
    }

    pub fn concat(&mut self, other: &mut Self) {
        if self.empty {
            *self = other.clone();
            return;
        }

        let new_state = NfaState::new();

        // set old accepting state to other's start state
        self.transitions.rename(NfaState::Accepting, new_state);

        let mut other = other.to_owned();

        // set other's start to new state
        other.transitions.rename(NfaState::Start, new_state);

        // copy other's transition table over to self
        for (start, map) in &other.transitions {
            self.transitions.insert(*start, map.clone());
        }
    }

    pub fn union(&mut self, other: &mut Self) {
        // since states are unique, the union is just the two transition tables merging
        for (start, map) in other.transitions.iter() {
            for (transition, states) in map.iter() {
                for state in states {
                    self.transitions.add_transition(*start, *transition, *state);
                }
            }
        }
    }

    // find all states reachable from the set states through epsilon-transitions alone
    pub fn epsilon_closure(&self, states: Vec<NfaState>) -> BTreeSet<NfaState> {
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
