#![allow(non_snake_case)]

use std::collections::{BTreeSet, HashMap};

use crate::nfa::Nfa;
use crate::transition_table::{State, Transition, TransitionTable};

#[derive(Debug)]
pub enum SimError {
    #[allow(dead_code)] // only accessed via Debug
    NoMatch(char),
    EndOfString,
    NoTransitions,
    Premature,
}

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Clone)]
pub struct DfaState {
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
    states: BTreeSet<DfaState>,
    start_state: DfaState,
}

impl Dfa {
    pub fn from_nfa(nfa: Nfa) -> Self {
        let start_state = DfaState::from(nfa.epsilon_closure(vec![State::Start]));
        let mut transitions = HashMap::new();
        let mut states = BTreeSet::from([start_state.clone()]);

        let mut seen = BTreeSet::new();
        let mut unmarked = BTreeSet::from([start_state.clone()]);

        while let Some(state) = unmarked.pop_first() {
            seen.insert(state.clone());

            if !states.contains(&state) {
                states.insert(state.clone());
            }

            for internal in &state.internal {
                if !nfa.transitions.contains_key(internal) {
                    continue;
                }
                for (transition, map) in nfa.transitions.get(internal).unwrap() {
                    let closure = DfaState::from(nfa.epsilon_closure(map.clone()));

                    if !transitions.contains_key(&state) {
                        transitions.insert(state.clone(), HashMap::new());
                    }

                    let row = transitions.get_mut(&state).unwrap();
                    if row.contains_key(transition) {
                        let curr_row: &mut DfaState = row.get_mut(transition).unwrap();

                        unmarked.remove(&curr_row);
                        unmarked.remove(&closure);

                        curr_row.internal = curr_row
                            .internal
                            .union(&closure.internal)
                            .cloned()
                            .collect();

                        curr_row.accepting &= closure.accepting;
                    } else {
                        row.insert(*transition, closure);
                    }

                    let insertion = row.get(transition).unwrap();
                    if !seen.contains(insertion) && !unmarked.contains(insertion) {
                        unmarked.insert(insertion.clone());
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
            transitions,
            states,
            start_state,
        }
    }

    pub fn minimize(&mut self) {
        // hopcroft's algorithm as described in (Hopcroft 1971) and (Xu 2009)

        // create inverse transition table
        // $ \delta^{-1}(s,a) = \{t|\delta(t, a) = s\} $
        let mut inv_delta: HashMap<DfaState, HashMap<Transition, BTreeSet<DfaState>>> =
            HashMap::new();

        for (start, map) in &self.transitions {
            for (trans, end) in map {
                inv_delta.add_transition(end.clone(), *trans, start.clone());
            }
        }

        let accepting = self
            .states
            .clone()
            .iter()
            .filter(|s| s.accepting)
            .map(|s| s.clone())
            .collect::<BTreeSet<_>>();

        let nonaccepting = self
            .states
            .clone()
            .iter()
            .filter(|s| !s.accepting)
            .map(|s| s.clone())
            .collect::<BTreeSet<_>>();

        let mut W = BTreeSet::from([accepting, nonaccepting]);
        let mut P = W.clone();

        while let Some(S) = W.pop_first() {
            // $$ I_a \leftarrow \delta^{-1}(S, a)$$
            let mut end_states = HashMap::new();
            for state in S {
                if !inv_delta.contains_key(&state) {
                    continue;
                }
                for (in_trans, ends) in inv_delta.get(&state).unwrap() {
                    if !end_states.contains_key(in_trans) {
                        end_states.insert(*in_trans, BTreeSet::new());
                    }
                    end_states
                        .get_mut(in_trans)
                        .unwrap()
                        .extend(ends.clone().iter().map(|s| s.clone()));
                }
            }

            for (_, end_states) in end_states {
                let P_clone = P.clone();
                let old_P_iter = P_clone.iter();
                for R in old_P_iter {
                    let mut intersection =
                        R.intersection(&end_states).map(|s| s.clone()).peekable();
                    if intersection.peek().is_some() && !R.is_subset(&end_states) {
                        let R1 = intersection.collect();
                        let R2: BTreeSet<DfaState> = R.difference(&R1).map(|s| s.clone()).collect();

                        P.remove(R);
                        P.insert(R1.clone());
                        P.insert(R2.clone());

                        if W.contains(R) {
                            W.remove(R);
                            W.insert(R1);
                            W.insert(R2);
                        } else {
                            if R1.len() <= R2.len() {
                                W.insert(R1);
                            } else {
                                W.insert(R2);
                            }
                        }
                    }
                }
            }
        }

        let mut changes: HashMap<DfaState, DfaState> = HashMap::new();
        for p in P {
            if p.len() > 1 {
                let mut new_state = DfaState {
                    internal: BTreeSet::new(),
                    accepting: false,
                };
                for state in &p {
                    new_state.internal.extend(state.internal.clone());
                    if new_state.accepting != state.accepting {
                        panic!("Accepting mismatch!");
                    }
                    new_state.accepting |= state.accepting;
                }

                for state in p {
                    changes.insert(state, new_state.clone());
                }
            }
        }

        for (old, new) in &changes {
            if self.transitions.contains_key(old) {
                let row = self.transitions.remove(old).unwrap();
                self.transitions.insert(new.clone(), row);
            }
        }

        for map in self.transitions.values_mut() {
            let map_clone = map.clone();
            let keys = map_clone.keys().collect::<Vec<_>>();
            for key in keys {
                if changes.contains_key(map.get(key).unwrap()) {
                    map.insert(*key, changes.get(map.get(key).unwrap()).unwrap().clone());
                }
            }
        }
    }

    pub fn to_dot(&self, label: String) -> String {
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

        format!("digraph dfa {{\ngraph [label=\"{label}\"];\n{node_str}\n{edges}}}")
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
