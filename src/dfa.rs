use std::collections::{BTreeSet, HashMap};

use crate::nfa::{Nfa, State, Transition};

#[derive(Debug)]
pub enum SimError {
    #[allow(dead_code)] // only accessed via Debug
    NoMatch(char),
    EndOfString,
    NoTransitions,
    Premature,
}

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Clone)]
struct DfaState {
    pub internal: BTreeSet<State>,
    accepting: bool,
}

impl From<BTreeSet<State>> for DfaState {
    fn from(value: BTreeSet<State>) -> Self {
        Self {
            internal: value.clone(),
            accepting: value.len() == 1 && value.contains(&State::Accepting),
        }
    }
}

impl DfaState {
    fn to_dot_node_ref(&self) -> String {
        self.internal.iter().fold(String::new(), |mut acc, s| {
            acc.push_str(s.to_dot_node().as_str());
            acc
        })
    }

    fn to_dot_node_label(&self) -> String {
        let r = self.to_dot_node_ref();
        let mut label = String::from("{");
        for state in &self.internal {
            if label.len() != 1 {
                label.push_str(", ");
            }
            label.push_str(state.to_dot_node().as_str());
        }
        label.push_str("}");

        let shape = if self.accepting {
            "doublecircle"
        } else {
            "circle"
        };

        format!("{r} [label = \"{label}\"; shape = {shape}];\n")
    }
}

#[derive(Debug)]
pub struct Dfa {
    transitions: HashMap<DfaState, HashMap<Transition, DfaState>>,
    start_state: DfaState,
}

impl Dfa {
    pub fn from_nfa(nfa: Nfa) -> Self {
        let start_state = DfaState::from(nfa.epsilon_closure(vec![State::Start]));
        let mut transitions = HashMap::new();

        let mut seen = BTreeSet::new();
        let mut unmarked = BTreeSet::from([start_state.clone()]);

        while let Some(state) = unmarked.pop_first() {
            seen.insert(state.clone());
            for internal in &state.internal {
                if !nfa.transitions.contains_key(internal) {
                    continue;
                }
                for (transition, map) in nfa.transitions.get(internal).unwrap() {
                    let closure = DfaState::from(nfa.epsilon_closure(map.clone()));
                    if !seen.contains(&closure) && !unmarked.contains(&closure) {
                        unmarked.insert(closure.clone());
                    }

                    if !transitions.contains_key(&state) {
                        transitions.insert(state.clone(), HashMap::new());
                    }

                    let row = transitions.get_mut(&state).unwrap();
                    if row.contains_key(transition) {
                        let curr_row: &mut DfaState = row.get_mut(transition).unwrap();
                        curr_row.internal = curr_row
                            .internal
                            .union(&closure.internal)
                            .cloned()
                            .collect();

                        curr_row.accepting &= closure.accepting;
                    } else {
                        row.insert(*transition, closure);
                    }
                }
            }
        }

        /* TODO: is it possible for a valid regex to have disjoint accepting states? ab?|abc is not well-formed
        let mut accepting_transitions = transitions.clone();
        for map in accepting_transitions.values_mut() {
            for state in map.values_mut() {
                // check if state is accepting
                if !transitions.contains_key(state)
                    || !transitions
                        .get(state)
                        .unwrap()
                        .contains_key(&Transition::Epsilon)
                {
                    state.accepting = true;
                }
            }
        }
        */

        Self {
            transitions: transitions,
            start_state: start_state,
        }
    }

    pub fn to_dot(&self) -> String {
        let mut edges = String::new();
        let mut nodes = HashMap::new();

        for (start, map) in &self.transitions {
            for (transition, end) in map {
                let start_ref = start.to_dot_node_ref();
                let end_ref = end.to_dot_node_ref();
                if !nodes.contains_key(&start_ref) {
                    nodes.insert(start_ref.clone(), start.to_dot_node_label());
                }
                if !nodes.contains_key(&end_ref) {
                    nodes.insert(end_ref.clone(), end.to_dot_node_label());
                }
                edges.push_str(
                    format!(
                        "{} -> {} [label = \"{}\"];\n",
                        start_ref,
                        end_ref,
                        transition.to_dot_label()
                    )
                    .as_str(),
                );
            }
        }

        let node_str = nodes.values().fold(String::new(), |mut acc, n| {
            acc.push_str(n.as_str());
            acc
        });

        format!("digraph dfa {{{node_str}\n{edges}}}")
    }

    pub fn simulate(&self, input: String) -> Result<(), SimError> {
        let mut curr_state = &self.start_state;

        let mut char_iter = input.chars().peekable();

        while !curr_state.accepting {
            if let Some(map) = self.transitions.get(&curr_state) {
                if char_iter.peek().is_some() {
                    let c = *char_iter.peek().unwrap();
                    let possible_edges = [
                        Transition::Literal(c),
                        Transition::Wildcard,
                        Transition::Epsilon,
                    ];

                    let transition = possible_edges.iter().find(|&edge| map.get(edge).is_some());

                    if transition.is_some() {
                        let transition = transition.unwrap();
                        if *transition != Transition::Epsilon {
                            let _ = char_iter.next();
                        }

                        curr_state = map.get(transition).unwrap();
                    } else {
                        return Err(SimError::NoMatch(c));
                    }
                } else if let Some(new_state) = map.get(&Transition::Epsilon) {
                    curr_state = new_state;
                } else {
                    return Err(SimError::EndOfString);
                }
            } else {
                return Err(SimError::NoTransitions);
            }
        }

        if char_iter.peek().is_none() {
            return Ok(());
        }

        Err(SimError::Premature)
    }
}
