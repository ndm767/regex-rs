use std::collections::{BTreeSet, HashMap};
use std::hash::Hash;
use std::sync::atomic::{AtomicU64, Ordering};

pub trait TransitionTable<T> {
    fn add_transition(&mut self, start: T, transition: Transition, end: T);
    fn rename(&mut self, old: T, new: T);
}

impl<S, T> TransitionTable<S> for HashMap<S, HashMap<Transition, T>>
where
    S: PartialEq + Eq + Hash + Clone,
    T: StateContainer<S>,
{
    fn add_transition(&mut self, start: S, transition: Transition, end: S) {
        self.entry(start)
            .or_default()
            .entry(transition)
            .or_insert(T::new_container())
            .insert_state(end);
    }
    fn rename(&mut self, old: S, new: S) {
        if self.contains_key(&old) {
            let row = self.remove(&old).unwrap();
            self.insert(new.clone(), row);
        }

        for (_, map) in self.iter_mut() {
            for end in map.values_mut() {
                if end.contains_state(&old) {
                    end.replace_state(&old, new.clone());
                }
            }
        }
    }
}

trait StateContainer<T> {
    fn new_container() -> Self;
    fn insert_state(&mut self, v: T);
    fn contains_state(&self, v: &T) -> bool;
    fn replace_state(&mut self, old: &T, new: T);
}

impl<T: PartialEq + Clone> StateContainer<T> for Vec<T> {
    fn new_container() -> Self {
        Self::new()
    }

    fn insert_state(&mut self, v: T) {
        self.push(v);
    }

    fn contains_state(&self, v: &T) -> bool {
        self.contains(v)
    }

    fn replace_state(&mut self, old: &T, new: T) {
        self.iter_mut().for_each(|v| {
            if *v == *old {
                *v = new.clone()
            }
        });
    }
}

impl<T> StateContainer<T> for BTreeSet<T>
where
    T: Ord,
{
    fn new_container() -> Self {
        Self::new()
    }

    fn insert_state(&mut self, v: T) {
        self.insert(v);
    }

    fn contains_state(&self, v: &T) -> bool {
        self.contains(v)
    }

    fn replace_state(&mut self, old: &T, new: T) {
        if self.remove(old) {
            self.insert(new);
        }
    }
}

static STATE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NfaState {
    Start,
    Accepting,
    S(u64),
}

impl NfaState {
    pub fn new() -> Self {
        Self::S(STATE_COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    pub fn dot_node(&self) -> String {
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
    pub fn dot_label(&self) -> String {
        match self {
            Self::Literal(c) => format!("'{c}'"),
            Self::Wildcard => ".".to_string(),
            Self::Epsilon => "ε".to_string(),
        }
    }
}
