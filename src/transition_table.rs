use std::collections::{BTreeSet, HashMap};
use std::hash::Hash;
use std::sync::atomic::{AtomicU64, Ordering};

pub trait TransitionTable<T> {
    fn add_transition(&mut self, start: T, transition: Transition, end: T);
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
}
trait StateContainer<T> {
    fn new_container() -> Self;
    fn insert_state(&mut self, v: T);
}

impl<T> StateContainer<T> for Vec<T> {
    fn new_container() -> Self {
        Self::new()
    }

    fn insert_state(&mut self, v: T) {
        self.push(v);
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
}

static STATE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum State {
    Start,
    Accepting,
    S(u64),
}

impl State {
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
