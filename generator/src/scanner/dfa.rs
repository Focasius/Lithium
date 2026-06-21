use crate::core::bitset::{BitSet, empty_set};
use crate::core::interval::Interval;
use crate::scanner::nfa::{EdgeType, NFA};
use rayon::prelude::*;
use std::collections::{BTreeSet, HashMap, VecDeque};

pub struct CompressedDFA {
    pub default_trans: Vec<usize>,
    pub special_trans: Vec<Vec<(usize, usize)>>,
    pub accept: Vec<Option<usize>>,
    pub start_state: usize,
    pub intervals: Vec<(u32, u32)>,
}

pub struct DFA {
    pub transitions: Vec<Vec<Option<usize>>>,
    pub accept: Vec<Option<usize>>,
    pub start_state: usize,
    pub intervals: Vec<(u32, u32)>,
}

impl DFA {
    pub fn minimize(&mut self) {
        let num_states = self.transitions.len();
        if num_states == 0 {
            return;
        }
        let num_classes = self.intervals.len();

        let mut part_of = vec![0; num_states];
        let mut parts = Vec::new();
        let mut part_id_of_token = std::collections::HashMap::new();
        for (i, &tok) in self.accept.iter().enumerate() {
            let key = tok.map(|t| t as i64).unwrap_or(-1);
            let pid = *part_id_of_token.entry(key).or_insert_with(|| {
                parts.push(Vec::new());
                parts.len() - 1
            });
            part_of[i] = pid;
            parts[pid].push(i);
        }

        let mut worklist: VecDeque<usize> = (0..parts.len()).collect();

        while let Some(pid) = worklist.pop_front() {
            for class in 0..num_classes {
                let mut split: std::collections::HashMap<usize, Vec<usize>> =
                    std::collections::HashMap::new();
                for &state in &parts[pid] {
                    let target = self.transitions[state][class];
                    let target_part = target.map(|t| part_of[t]).unwrap_or(usize::MAX);
                    split.entry(target_part).or_default().push(state);
                }
                if split.len() <= 1 {
                    continue;
                }

                let mut new_blocks: Vec<Vec<usize>> = split.into_values().collect();
                parts[pid] = new_blocks.remove(0);
                for new_block in new_blocks {
                    let new_pid = parts.len();
                    parts.push(new_block);
                    worklist.push_back(new_pid);
                    for &s in &parts[new_pid] {
                        part_of[s] = new_pid;
                    }
                }

                worklist.push_back(pid);
                break;
            }
        }

        let new_states = parts.len();
        let mut new_transitions = vec![vec![None; num_classes]; new_states];
        let mut new_accept = vec![None; new_states];
        for (pid, block) in parts.iter().enumerate() {
            let rep = block[0];
            new_accept[pid] = self.accept[rep];
            for class in 0..num_classes {
                if let Some(next) = self.transitions[rep][class] {
                    new_transitions[pid][class] = Some(part_of[next]);
                }
            }
        }
        self.transitions = new_transitions;
        self.accept = new_accept;
        self.start_state = part_of[self.start_state];
    }

    pub fn minimize_parallel(&mut self) {
        let num_states = self.transitions.len();
        if num_states == 0 {
            return;
        }
        let num_classes = self.intervals.len();

        let mut part_of = vec![0; num_states];
        let mut parts = Vec::new();
        let mut part_id_of_token = std::collections::HashMap::new();
        for (i, &tok) in self.accept.iter().enumerate() {
            let key = tok.map(|t| t as i64).unwrap_or(-1);
            let pid = *part_id_of_token.entry(key).or_insert_with(|| {
                parts.push(Vec::new());
                parts.len() - 1
            });
            part_of[i] = pid;
            parts[pid].push(i);
        }

        let mut worklist: VecDeque<usize> = (0..parts.len()).collect();

        while let Some(pid) = worklist.pop_front() {
            let block = parts[pid].clone();
            if block.is_empty() {
                continue;
            }

            let mut split_occurred = false;
            for class in 0..num_classes {
                use std::collections::HashMap;
                let split_map = block
                    .par_iter()
                    .fold(
                        HashMap::new,
                        |mut map: HashMap<usize, Vec<usize>>, &state| {
                            let target = self.transitions[state][class];
                            let target_part = target.map(|t| part_of[t]).unwrap_or(usize::MAX);
                            map.entry(target_part).or_default().push(state);
                            map
                        },
                    )
                    .reduce(HashMap::new, |mut map1, map2| {
                        for (k, v) in map2 {
                            map1.entry(k).or_default().extend(v);
                        }
                        map1
                    });

                if split_map.len() <= 1 {
                    continue;
                }

                let mut subblocks: Vec<Vec<usize>> = split_map.into_values().collect();
                let kept = subblocks.remove(0);
                parts[pid] = kept;
                for new_block in subblocks {
                    let new_pid = parts.len();
                    parts.push(new_block);
                    worklist.push_back(new_pid);
                    for &s in &parts[new_pid] {
                        part_of[s] = new_pid;
                    }
                }
                split_occurred = true;
                break;
            }

            if split_occurred {
                worklist.push_back(pid);
            }
        }

        let new_states = parts.len();
        let mut new_transitions = vec![vec![None; num_classes]; new_states];
        let mut new_accept = vec![None; new_states];
        for (pid, block) in parts.iter().enumerate() {
            let rep = block[0];
            new_accept[pid] = self.accept[rep];
            for class in 0..num_classes {
                if let Some(next) = self.transitions[rep][class] {
                    new_transitions[pid][class] = Some(part_of[next]);
                }
            }
        }
        self.transitions = new_transitions;
        self.accept = new_accept;
        self.start_state = part_of[self.start_state];
    }

    pub fn compress(&self) -> CompressedDFA {
        let num_states = self.transitions.len();
        let mut default_trans = vec![0; num_states];
        let mut special_trans = vec![Vec::new(); num_states];

        for s in 0..num_states {
            let row = &self.transitions[s];
            if row.is_empty() {
                default_trans[s] = 0;
                continue;
            }

            let mut best = row[0];
            let mut best_count = 1;
            for i in 1..row.len() {
                let mut count = 1;
                for j in 0..i {
                    if row[j] == row[i] {
                        count += 1;
                    }
                }
                if count > best_count {
                    best_count = count;
                    best = row[i];
                }
            }
            let default_opt = best;
            let default = default_opt.unwrap_or(0);
            default_trans[s] = default;

            for (class, &next_opt) in row.iter().enumerate() {
                if next_opt != default_opt
                    && let Some(next) = next_opt
                {
                    special_trans[s].push((class, next));
                }
            }
        }

        CompressedDFA {
            default_trans,
            special_trans,
            accept: self.accept.clone(),
            start_state: self.start_state,
            intervals: self.intervals.clone(),
        }
    }

    pub fn merge_columns(&mut self) {
        let num_classes = self.intervals.len();
        if num_classes <= 1 {
            return;
        }

        let columns: Vec<Vec<Option<usize>>> = (0..num_classes)
            .map(|col| self.transitions.iter().map(|row| row[col]).collect())
            .collect();

        let mut merged: Vec<Vec<usize>> = Vec::new();
        let mut used = vec![false; num_classes];
        for i in 0..num_classes {
            if used[i] {
                continue;
            }
            let mut group = vec![i];
            used[i] = true;
            for j in i + 1..num_classes {
                if !used[j] && columns[i] == columns[j] {
                    used[j] = true;
                    group.push(j);
                }
            }
            merged.push(group);
        }

        if merged.len() == num_classes {
            return;
        }

        let mut new_intervals = Vec::new();
        for group in &merged {
            let start = self.intervals[group[0]].0;
            let end = self.intervals[group[group.len() - 1]].1;
            new_intervals.push((start, end));
        }

        let mut new_transitions = Vec::new();
        for row in &self.transitions {
            let mut new_row = Vec::with_capacity(merged.len());
            for group in &merged {
                new_row.push(row[group[0]]);
            }
            new_transitions.push(new_row);
        }

        self.transitions = new_transitions;
        self.intervals = new_intervals;
    }
}

fn collect_boundaries(nfa: &NFA) -> Vec<u32> {
    let mut points = BTreeSet::new();
    points.insert(0);
    points.insert(0x110000);
    for state in &nfa.states {
        for edge in &state.edges {
            match edge.edge_type {
                EdgeType::Character => {
                    if let Some(ch) = edge.character {
                        points.insert(ch);
                        points.insert(ch + 1);
                    }
                }
                EdgeType::Range => {
                    if let (Some(start), Some(end)) = (edge.range_start, edge.range_end) {
                        points.insert(start);
                        points.insert(end + 1);
                    }
                }
                _ => {}
            }
        }
    }
    points.into_iter().collect()
}

fn build_intervals(boundaries: &[u32]) -> Vec<(u32, u32)> {
    let intervals = Interval::build_intervals(boundaries);
    intervals
        .into_iter()
        .map(|int| (int.start, int.end))
        .collect()
}

fn epsilon_closure(nfa: &NFA, start_set: &BitSet) -> BitSet {
    let mut closure = start_set.clone();
    let mut stack: Vec<usize> = start_set.iter().collect();
    while let Some(s) = stack.pop() {
        for edge in &nfa.states[s].edges {
            if let EdgeType::Epsilon = edge.edge_type {
                let t = edge.target;
                if !closure.contains(t) {
                    closure.add(t);
                    stack.push(t);
                }
            }
        }
    }
    closure
}

fn move_on_codepoint(nfa: &NFA, states: &BitSet, cp: u32) -> BitSet {
    let mut next = empty_set(nfa.states.len());
    for s in states.iter() {
        for edge in &nfa.states[s].edges {
            let matches = match edge.edge_type {
                EdgeType::Character => {
                    if let Some(ch) = edge.character {
                        cp == ch
                    } else {
                        false
                    }
                }
                EdgeType::Range => {
                    if let (Some(start), Some(end)) = (edge.range_start, edge.range_end) {
                        cp >= start && cp <= end
                    } else {
                        false
                    }
                }
                _ => false,
            };
            if matches {
                next.add(edge.target);
            }
        }
    }
    epsilon_closure(nfa, &next)
}

pub fn build_dfa(nfa: &NFA) -> DFA {
    let boundaries = collect_boundaries(nfa);
    let intervals = build_intervals(&boundaries);
    let num_classes = intervals.len();

    let mut start_set = empty_set(nfa.states.len());
    start_set.add(nfa.start);
    let start_set = epsilon_closure(nfa, &start_set);

    let mut dfa_states: Vec<BitSet> = vec![start_set];
    let mut dfa_trans: Vec<Vec<Option<usize>>> = vec![vec![None; num_classes]];
    let mut state_to_id: HashMap<BitSet, usize> = HashMap::new();
    state_to_id.insert(dfa_states[0].clone(), 0);

    let mut queue = VecDeque::new();
    queue.push_back(0);

    while let Some(id) = queue.pop_front() {
        let set = dfa_states[id].clone();

        let next_sets: Vec<Option<BitSet>> = intervals
            .par_iter()
            .map(|&(start_cp, _)| {
                let next_set = move_on_codepoint(nfa, &set, start_cp);
                if next_set.is_empty() {
                    None
                } else {
                    Some(next_set)
                }
            })
            .collect();

        for (class_idx, next_set_opt) in next_sets.into_iter().enumerate() {
            if let Some(next_set) = next_set_opt {
                let next_id = if let Some(&i) = state_to_id.get(&next_set) {
                    i
                } else {
                    let new_id = dfa_states.len();
                    dfa_states.push(next_set.clone());
                    dfa_trans.push(vec![None; num_classes]);
                    state_to_id.insert(next_set, new_id);
                    queue.push_back(new_id);
                    new_id
                };
                dfa_trans[id][class_idx] = Some(next_id);
            }
        }
    }

    let accept: Vec<Option<usize>> = dfa_states
        .iter()
        .map(|set| {
            let mut best_token = None;
            for &(state_id, token_id) in &nfa.accepts {
                if set.contains(state_id)
                    && (best_token.is_none() || token_id < best_token.unwrap())
                {
                    best_token = Some(token_id);
                }
            }
            best_token
        })
        .collect();

    let mut dfa = DFA {
        transitions: dfa_trans,
        accept,
        start_state: 0,
        intervals,
    };

    dfa.merge_columns();

    dfa
}

impl std::fmt::Display for DFA {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "DFA start state: {}", self.start_state)?;
        writeln!(f, "Number of states: {}", self.transitions.len())?;
        writeln!(f, "Number of character classes: {}", self.intervals.len())?;
        writeln!(
            f,
            "Accept states: {:?}",
            self.accept
                .iter()
                .enumerate()
                .filter_map(|(i, &a)| a.map(|tok| (i, tok)))
                .collect::<Vec<_>>()
        )?;
        writeln!(f, "Transitions:")?;
        for (i, row) in self.transitions.iter().enumerate() {
            for (j, &next) in row.iter().enumerate() {
                if let Some(next_state) = next {
                    let (start, end) = self.intervals[j];
                    writeln!(
                        f,
                        "  state {} -- [U+{:04X}, U+{:04X}) --> state {}",
                        i, start, end, next_state
                    )?;
                }
            }
        }
        Ok(())
    }
}
