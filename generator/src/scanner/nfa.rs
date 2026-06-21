use crate::core::regex::ast::{RegexType, RepeatType};
use std::fmt::{Display, Formatter, Result};

#[derive(Debug, Clone)]
pub enum EdgeType {
    Epsilon,
    Character,
    Range,
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub edge_type: EdgeType,
    pub character: Option<u32>,
    pub range_start: Option<u32>,
    pub range_end: Option<u32>,
    pub target: usize,
}

#[derive(Debug, Clone)]
pub struct State {
    pub edges: Vec<Edge>,
}

#[derive(Debug, Clone)]
pub struct NFA {
    pub states: Vec<State>,
    pub start: usize,
    pub accepts: Vec<(usize, usize)>,
}

impl NFA {
    pub fn new() -> Self {
        NFA {
            states: Vec::new(),
            start: 0,
            accepts: Vec::new(),
        }
    }
    pub fn add_state(&mut self, state: State) -> usize {
        let index = self.states.len();
        self.states.push(state);
        index
    }
    pub fn add_edge(&mut self, from: usize, edge: Edge) {
        self.states[from].edges.push(edge);
    }
}

impl Default for NFA {
    fn default() -> Self {
        Self::new()
    }
}

pub fn epsilon_edge(target: usize) -> Edge {
    Edge {
        edge_type: EdgeType::Epsilon,
        character: None,
        range_start: None,
        range_end: None,
        target,
    }
}

pub fn char_edge(ch: u32, target: usize) -> Edge {
    Edge {
        edge_type: EdgeType::Character,
        character: Some(ch),
        range_start: None,
        range_end: None,
        target,
    }
}

pub fn range_edge(start: u32, end: u32, target: usize) -> Edge {
    Edge {
        edge_type: EdgeType::Range,
        character: None,
        range_start: Some(start),
        range_end: Some(end),
        target,
    }
}

fn build_regex_fragment(regex: &RegexType, nfa: &mut NFA) -> (usize, usize) {
    match regex {
        RegexType::Char(ch) => {
            let s = nfa.add_state(State { edges: Vec::new() });
            let a = nfa.add_state(State { edges: Vec::new() });
            nfa.add_edge(s, char_edge(*ch, a));
            (s, a)
        }
        RegexType::Concat(left, right) => {
            let (s1, a1) = build_regex_fragment(left, nfa);
            let (s2, a2) = build_regex_fragment(right, nfa);
            nfa.add_edge(a1, epsilon_edge(s2));
            (s1, a2)
        }
        RegexType::Alt(left, right) => {
            let (s1, a1) = build_regex_fragment(left, nfa);
            let (s2, a2) = build_regex_fragment(right, nfa);
            let start = nfa.add_state(State { edges: Vec::new() });
            let accept = nfa.add_state(State { edges: Vec::new() });
            nfa.add_edge(start, epsilon_edge(s1));
            nfa.add_edge(start, epsilon_edge(s2));
            nfa.add_edge(a1, epsilon_edge(accept));
            nfa.add_edge(a2, epsilon_edge(accept));
            (start, accept)
        }
        RegexType::Star(sub) => {
            let (s_in, a_in) = build_regex_fragment(sub, nfa);
            let start = nfa.add_state(State { edges: Vec::new() });
            let accept = nfa.add_state(State { edges: Vec::new() });
            nfa.add_edge(start, epsilon_edge(s_in));
            nfa.add_edge(start, epsilon_edge(accept));
            nfa.add_edge(a_in, epsilon_edge(s_in));
            nfa.add_edge(a_in, epsilon_edge(accept));
            (start, accept)
        }
        RegexType::Plus(sub) => {
            let (s_in, a_in) = build_regex_fragment(sub, nfa);
            let start = nfa.add_state(State { edges: Vec::new() });
            let accept = nfa.add_state(State { edges: Vec::new() });
            nfa.add_edge(start, epsilon_edge(s_in));
            nfa.add_edge(a_in, epsilon_edge(s_in));
            nfa.add_edge(a_in, epsilon_edge(accept));
            (start, accept)
        }
        RegexType::Opt(sub) => {
            let (s_in, a_in) = build_regex_fragment(sub, nfa);
            let start = nfa.add_state(State { edges: Vec::new() });
            let accept = nfa.add_state(State { edges: Vec::new() });
            nfa.add_edge(start, epsilon_edge(s_in));
            nfa.add_edge(start, epsilon_edge(accept));
            nfa.add_edge(a_in, epsilon_edge(accept));
            (start, accept)
        }
        RegexType::CharClass(class) => {
            let s = nfa.add_state(State { edges: Vec::new() });
            let a = nfa.add_state(State { edges: Vec::new() });
            if class.negated {
                let mut points = std::collections::BTreeSet::new();
                points.insert(0);
                points.insert(0x110000);
                for item in &class.items {
                    points.insert(item.start);
                    points.insert(item.end + 1);
                }
                let boundaries: Vec<u32> = points.into_iter().collect();
                for win in boundaries.windows(2) {
                    let start = win[0];
                    let end = win[1];
                    if start < end {
                        nfa.add_edge(s, range_edge(start, end - 1, a));
                    }
                }
            } else {
                for item in &class.items {
                    nfa.add_edge(s, range_edge(item.start, item.end, a));
                }
            }
            (s, a)
        }
        RegexType::Repeat(sub, kind) => match kind {
            RepeatType::Exactly(count) => {
                if *count == 0 {
                    let start = nfa.add_state(State { edges: Vec::new() });
                    let accept = nfa.add_state(State { edges: Vec::new() });
                    nfa.add_edge(start, epsilon_edge(accept));
                    (start, accept)
                } else {
                    let (start, mut accept) = build_regex_fragment(sub, nfa);
                    for _ in 1..*count {
                        let (next_start, next_accept) = build_regex_fragment(sub, nfa);
                        nfa.add_edge(accept, epsilon_edge(next_start));
                        accept = next_accept;
                    }
                    (start, accept)
                }
            }
            RepeatType::AtLeast(n) => {
                let (start, mut accept) = build_regex_fragment(sub, nfa);
                for _ in 1..*n {
                    let (next_start, next_accept) = build_regex_fragment(sub, nfa);
                    nfa.add_edge(accept, epsilon_edge(next_start));
                    accept = next_accept;
                }
                nfa.add_edge(accept, epsilon_edge(start));
                let final_accept = nfa.add_state(State { edges: Vec::new() });
                nfa.add_edge(accept, epsilon_edge(final_accept));
                (start, final_accept)
            }
            RepeatType::Between(n, m) => {
                if *n == 0 && *m == 0 {
                    let start = nfa.add_state(State { edges: Vec::new() });
                    let accept = nfa.add_state(State { edges: Vec::new() });
                    nfa.add_edge(start, epsilon_edge(accept));
                    return (start, accept);
                }
                let (start, mut accept) = build_regex_fragment(sub, nfa);
                for _ in 1..*n {
                    let (next_start, next_accept) = build_regex_fragment(sub, nfa);
                    nfa.add_edge(accept, epsilon_edge(next_start));
                    accept = next_accept;
                }
                let mut current_accept = accept;
                for _ in 0..(m - n) {
                    let (opt_start, opt_accept) = build_regex_fragment(sub, nfa);
                    let new_tail = nfa.add_state(State { edges: Vec::new() });
                    nfa.add_edge(current_accept, epsilon_edge(new_tail));
                    nfa.add_edge(current_accept, epsilon_edge(opt_start));
                    nfa.add_edge(opt_accept, epsilon_edge(new_tail));
                    current_accept = new_tail;
                }
                (start, current_accept)
            }
        },
    }
}

pub fn build_nfa(patterns: &[(usize, RegexType)]) -> NFA {
    let mut nfa = NFA::new();
    let global_start = nfa.add_state(State { edges: Vec::new() });
    nfa.start = global_start;

    for &(token_id, ref ast) in patterns {
        let (sub_start, sub_accept) = build_regex_fragment(ast, &mut nfa);
        nfa.add_edge(global_start, epsilon_edge(sub_start));
        nfa.accepts.push((sub_accept, token_id));
    }

    nfa
}

impl Display for NFA {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "Start state: {}", self.start)?;
        writeln!(f, "Accepts states: {:?}", self.accepts)?;
        for (i, state) in self.states.iter().enumerate() {
            writeln!(f, "State {}:", i)?;
            for edge in &state.edges {
                match edge.edge_type {
                    EdgeType::Epsilon => writeln!(f, "  ε -> {}", edge.target)?,
                    EdgeType::Character => {
                        let ch = edge.character.unwrap();
                        if let Some(c) = std::char::from_u32(ch) {
                            writeln!(f, "  '{}' -> {}", c, edge.target)?;
                        } else {
                            writeln!(f, "  U+{:04X} -> {}", ch, edge.target)?;
                        }
                    }
                    EdgeType::Range => {
                        let start = edge.range_start.unwrap();
                        let end = edge.range_end.unwrap();
                        writeln!(f, "  [U+{:04X}, U+{:04X}] -> {}", start, end, edge.target)?;
                    }
                }
            }
        }
        Ok(())
    }
}
