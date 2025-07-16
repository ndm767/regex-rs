#![allow(non_snake_case)]

use std::collections::{BTreeSet, HashMap};

use crate::nfa::Nfa;
use crate::transition_table::{NfaState, StateContainer, Transition, TransitionTable};

#[derive(Debug, PartialEq)]
pub enum SimError {
    #[allow(dead_code)] // only accessed via Debug
    NoMatch(char),
    EndOfString,
    NoTransitions,
    Premature,
}

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Clone, Default)]
pub struct DfaState {
    pub internal: BTreeSet<NfaState>,
    accepting: bool,
}

impl From<BTreeSet<NfaState>> for DfaState {
    fn from(value: BTreeSet<NfaState>) -> Self {
        Self {
            internal: value.clone(),
            accepting: value.contains(&NfaState::Accepting),
        }
    }
}

impl StateContainer<DfaState> for DfaState {
    fn new_container() -> Self {
        Self {
            internal: BTreeSet::new(),
            accepting: false,
        }
    }

    fn insert_state(&mut self, v: DfaState) {
        self.internal = v.internal;
        self.accepting = v.accepting;
    }

    fn contains_state(&self, v: &DfaState) -> bool {
        *self == *v
    }

    fn replace_state(&mut self, old: &DfaState, new: DfaState) {
        if *self == *old {
            self.insert_state(new);
        }
    }
}

impl DfaState {
    fn merge(&mut self, other: DfaState) {
        self.internal = self.internal.union(&other.internal).cloned().collect();
        self.accepting |= other.accepting;
    }

    fn to_dot_node_ref(&self) -> String {
        self.internal.iter().fold(String::new(), |mut acc, s| {
            acc.push_str(s.dot_node().as_str());
            acc
        })
    }

    fn to_dot_node_label(&self) -> String {
        let r = self.to_dot_node_ref();
        let mut label = String::from('{');

        for state in &self.internal {
            if label.len() != 1 {
                label.push_str(", ");
            }
            label.push_str(state.dot_node().as_str());
        }

        label.push('}');

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
        let start_state = DfaState::from(nfa.epsilon_closure(vec![NfaState::Start]));
        let mut transitions: HashMap<DfaState, HashMap<Transition, DfaState>> = HashMap::new();
        let mut states = BTreeSet::from([start_state.clone()]);

        let mut seen = BTreeSet::new();
        let mut unmarked = BTreeSet::from([start_state.clone()]);

        while let Some(state) = unmarked.pop_first() {
            seen.insert(state.clone());

            if !states.contains(&state) {
                states.insert(state.clone());
            }

            // find all transitions out of the state set
            let mut possible: HashMap<Transition, Vec<NfaState>> = HashMap::new();

            for internal in &state.internal {
                if !nfa.transitions.contains_key(internal) {
                    continue;
                }

                for (transition, ends) in nfa.transitions.get(internal).unwrap() {
                    if *transition == Transition::Epsilon {
                        continue;
                    }

                    possible
                        .entry(*transition)
                        .or_default()
                        .extend(ends.clone());
                }
            }

            // If there is a wildcard transition, add its end states to every other transition
            // this allows for expressions such as a.?b
            if possible.contains_key(&Transition::Wildcard) {
                let wildcards = possible.get(&Transition::Wildcard).unwrap().clone();
                for (trans, ends) in possible.iter_mut() {
                    if *trans != Transition::Wildcard {
                        ends.extend(wildcards.clone());
                    }
                }
            }

            // loop through each transition
            for (trans, ends) in possible {
                let closure = DfaState::from(nfa.epsilon_closure(ends));
                if !transitions.contains_key(&state) {
                    transitions.insert(state.clone(), HashMap::new());
                }

                transitions
                    .get_mut(&state)
                    .unwrap()
                    .entry(trans)
                    .or_default()
                    .merge(closure);

                // since we may have merged states, the new state may not necessarily be the closure
                // add it to the unmarked set if we haven't seen it before
                let insertion = transitions.get(&state).unwrap().get(&trans).unwrap();
                if !seen.contains(insertion) && !unmarked.contains(insertion) {
                    unmarked.insert(insertion.clone());
                }
            }
        }

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

        // start with partitions of accepting and non-accepting states
        let accepting = self
            .states
            .iter()
            .filter(|s| s.accepting)
            .cloned()
            .collect::<BTreeSet<_>>();

        let nonaccepting = self
            .states
            .iter()
            .filter(|s| !s.accepting)
            .cloned()
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
                        .extend(ends.iter().cloned());
                }
            }

            for (_, end_states) in end_states {
                let P_clone = P.clone();
                let old_P_iter = P_clone.iter();
                for R in old_P_iter {
                    let mut intersection = R.intersection(&end_states).cloned().peekable();
                    if intersection.peek().is_some() && !R.is_subset(&end_states) {
                        let R1 = intersection.collect();
                        let R2: BTreeSet<DfaState> = R.difference(&R1).cloned().collect();

                        P.remove(R);
                        P.insert(R1.clone());
                        P.insert(R2.clone());

                        if W.contains(R) {
                            W.remove(R);
                            W.insert(R1);
                            W.insert(R2);
                        } else if R1.len() <= R2.len() {
                            W.insert(R1);
                        } else {
                            W.insert(R2);
                        }
                    }
                }
            }
        }

        // map the old DFA states to their new, minimized equivalents
        let mut changes: HashMap<DfaState, DfaState> = HashMap::new();
        for p in P {
            if p.len() > 1 {
                let mut new_state = DfaState {
                    internal: BTreeSet::new(),
                    accepting: false,
                };
                for state in &p {
                    new_state.internal.extend(state.internal.clone());
                    new_state.accepting |= state.accepting;
                }

                for state in p {
                    changes.insert(state, new_state.clone());
                }
            }
        }

        if changes.contains_key(&self.start_state) {
            self.start_state = changes.get(&self.start_state).unwrap().clone();
        }

        for (old, new) in changes {
            self.transitions.rename(old, new);
        }
    }

    pub fn to_dot(&self, label: &str) -> String {
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
                        transition.dot_label()
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
        let mut accepted = false;

        while !accepted {
            if let Some(map) = self.transitions.get(curr_state) {
                if char_iter.peek().is_some() {
                    let c = *char_iter.peek().unwrap();
                    let possible_edges = [
                        Transition::Literal(c),
                        Transition::Wildcard,
                        Transition::Epsilon,
                    ];

                    let transition = possible_edges.iter().find(|&edge| map.get(edge).is_some());

                    if let Some(transition) = transition {
                        if *transition != Transition::Epsilon {
                            let _ = char_iter.next();
                        }

                        curr_state = map.get(transition).unwrap();
                    } else if curr_state.accepting {
                        accepted = true;
                    } else {
                        return Err(SimError::NoMatch(c));
                    }
                } else if let Some(new_state) = map.get(&Transition::Epsilon) {
                    curr_state = new_state;
                } else if curr_state.accepting {
                    accepted = true;
                } else {
                    return Err(SimError::EndOfString);
                }
            } else if curr_state.accepting {
                accepted = true;
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
