// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::transition::Transition;

// state set --\
//             |-- state node --\
//             |                |- link head --> transition
//             |                |- link node --> transition
//             |                |- ...
//             |                \- link tail --> transition
//             |-- state node
//             |-- ...
//             |-- state node

pub struct StateSet {
    pub start_node_index: usize,
    pub end_node_index: usize,
    states: Vec<StateNode>,
    links: Vec<LinkNode>,
    transitions: Vec<TransitionNode>,
}

// Every state node has one or more transitions.
struct StateNode {
    link_head_index: Option<usize>,
    link_tail_index: Option<usize>,
}

struct LinkNode {
    previous_index: Option<usize>, // the index of previous link node
    next_index: Option<usize>,     // the index of next link node
    transition_index: usize,       // the index of transition node
}

struct TransitionNode {
    transition: Transition,    // the type of transition
    target_state_index: usize, // the index of next state
}

impl StateSet {
    pub fn new() -> Self {
        StateSet {
            start_node_index: 0,
            end_node_index: 0,
            states: vec![],
            links: vec![],
            transitions: vec![],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    // return the index of the new state node
    pub fn new_state(&mut self) -> usize {
        let state = StateNode {
            link_head_index: None,
            link_tail_index: None,
        };
        let idx = self.states.len();
        self.states.push(state);
        idx
    }

    // private (helper) function
    fn add_transition_node(&mut self, transition_node: TransitionNode) -> usize {
        let idx = self.transitions.len();
        self.transitions.push(transition_node);
        idx
    }

    // private (helper) function
    fn add_link_node(&mut self, link_node: LinkNode) -> usize {
        let idx = self.links.len();
        self.links.push(link_node);
        idx
    }

    pub fn append_transition(
        &mut self,
        source_state_index: usize,
        target_state_index: usize,
        transition: Transition,
    ) {
        let transition_node = TransitionNode {
            transition,
            target_state_index,
        };
        let transition_index = self.add_transition_node(transition_node);

        let is_transition_empty = self.states[source_state_index].is_transition_empty();
        if is_transition_empty {
            // add the first link list node
            let link_node = LinkNode {
                previous_index: None,
                next_index: None,
                transition_index,
            };
            let link_node_index = self.add_link_node(link_node);

            let source_state = &mut self.states[source_state_index];
            source_state.link_head_index = Some(link_node_index);
            source_state.link_tail_index = Some(link_node_index);
        } else {
            // append new transition node to the tail of link list

            // create new link list node
            let last_link_node_index = self.states[source_state_index].link_tail_index.unwrap();
            let link_node = LinkNode {
                previous_index: Some(last_link_node_index),
                next_index: None,
                transition_index,
            };
            let link_node_index = self.add_link_node(link_node);

            // change the last node
            self.links[last_link_node_index].next_index = Some(link_node_index);

            // change the tail node pointer of state
            self.states[source_state_index].link_tail_index = Some(link_node_index);
        }
    }

    pub fn insert_transition(
        &mut self,
        source_state_index: usize,
        target_state_index: usize,
        transition: Transition,
    ) {
        let transition_node = TransitionNode {
            transition,
            target_state_index,
        };
        let transition_index = self.add_transition_node(transition_node);

        let is_transition_empty = self.states[source_state_index].is_transition_empty();
        if is_transition_empty {
            // add the first link list node
            let link_node = LinkNode {
                previous_index: None,
                next_index: None,
                transition_index,
            };
            let link_node_index = self.add_link_node(link_node);

            let source_state = &mut self.states[source_state_index];
            source_state.link_head_index = Some(link_node_index);
            source_state.link_tail_index = Some(link_node_index);
        } else {
            // instart new transition node to the head of link list

            // create new link list node
            let first_link_node_index = self.states[source_state_index].link_head_index.unwrap();
            let link_node = LinkNode {
                previous_index: None,
                next_index: Some(first_link_node_index),
                transition_index,
            };
            let link_node_index = self.add_link_node(link_node);

            // change the first node
            self.links[first_link_node_index].previous_index = Some(link_node_index);

            // change the head node pointer of state
            self.states[source_state_index].link_head_index = Some(link_node_index);
        }
    }

    // for debug
    pub fn get_transition_index_list(&self, source_state_index: usize) -> Vec<usize> {
        let mut indices = vec![];
        let mut next_index = self.states[source_state_index].link_head_index;

        while let Some(idx) = next_index {
            indices.push(idx);
            next_index = self.links[idx].next_index;
        }

        indices
    }

    pub fn get_states_and_transitions_text(&self) -> String {
        let mut lines = vec![];
        for (state_index, state_node) in self.states.iter().enumerate() {
            let prefix = if state_index == self.start_node_index {
                '>'
            } else if state_index == self.end_node_index {
                '<'
            } else {
                '-'
            };

            let state_line = format!(
                "{} idx:{}, head:{:?}, tail:{:?}",
                prefix, state_index, state_node.link_head_index, state_node.link_tail_index
            );
            lines.push(state_line);

            let mut next_link_node_index = state_node.link_head_index;
            while let Some(link_node_index) = next_link_node_index {
                let link_node = &self.links[link_node_index];
                let link_line = format!(
                    "  * link idx:{}, prev:{:?}, next:{:?}",
                    link_node_index, link_node.previous_index, link_node.next_index
                );

                let transition_node_index = link_node.transition_index;
                let transition_node = &self.transitions[transition_node_index];
                let transition_line = format!(
                    "    trans idx:{}, target state:{}, [{}]",
                    transition_node_index,
                    transition_node.target_state_index,
                    transition_node.transition
                );

                lines.push(link_line);
                lines.push(transition_line);

                // update next
                next_link_node_index = link_node.next_index;
            }
        }

        lines.join("\n")
    }
}

impl StateNode {
    pub fn is_transition_empty(&self) -> bool {
        self.link_head_index.is_none()
    }

    pub fn get_first_transition_index(&self) -> Option<usize> {
        self.link_head_index
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_str_eq;

    use crate::transition::{CharTransition, Transition};

    use super::StateSet;

    #[test]
    fn test_state_set() {
        let mut state_set = StateSet::new();
        assert!(state_set.is_empty());

        // create a state
        state_set.new_state();

        assert!(!state_set.is_empty());
        assert_str_eq!(
            state_set.get_states_and_transitions_text(),
            "> idx:0, head:None, tail:None"
        );

        // create other states
        let idx1 = state_set.new_state();
        state_set.new_state();
        let idx2 = state_set.new_state();

        state_set.start_node_index = idx1;
        state_set.end_node_index = idx2;

        assert_str_eq!(
            state_set.get_states_and_transitions_text(),
            "\
- idx:0, head:None, tail:None
> idx:1, head:None, tail:None
- idx:2, head:None, tail:None
< idx:3, head:None, tail:None"
        );
    }

    #[test]
    fn test_state_transition_append() {
        let mut state_set = StateSet::new();
        let state_idx0 = state_set.new_state();
        let state_idx1 = state_set.new_state();
        let state_idx2 = state_set.new_state();
        let state_idx3 = state_set.new_state();
        let state_idx4 = state_set.new_state();

        state_set.append_transition(
            state_idx0,
            state_idx1,
            Transition::Char(CharTransition::new('a', false)),
        );

        assert_str_eq!(
            state_set.get_states_and_transitions_text(),
            "\
> idx:0, head:Some(0), tail:Some(0)
  * link idx:0, prev:None, next:None
    trans idx:0, target state:1, [Char a]
- idx:1, head:None, tail:None
- idx:2, head:None, tail:None
- idx:3, head:None, tail:None
- idx:4, head:None, tail:None"
        );

        state_set.append_transition(
            state_idx0,
            state_idx2,
            Transition::Char(CharTransition::new('b', false)),
        );

        state_set.append_transition(
            state_idx0,
            state_idx3,
            Transition::Char(CharTransition::new('c', false)),
        );

        assert_str_eq!(
            state_set.get_states_and_transitions_text(),
            "\
> idx:0, head:Some(0), tail:Some(2)
  * link idx:0, prev:None, next:Some(1)
    trans idx:0, target state:1, [Char a]
  * link idx:1, prev:Some(0), next:Some(2)
    trans idx:1, target state:2, [Char b]
  * link idx:2, prev:Some(1), next:None
    trans idx:2, target state:3, [Char c]
- idx:1, head:None, tail:None
- idx:2, head:None, tail:None
- idx:3, head:None, tail:None
- idx:4, head:None, tail:None"
        );

        state_set.insert_transition(
            state_idx0,
            state_idx4,
            Transition::Char(CharTransition::new('d', false)),
        );

        assert_str_eq!(
            state_set.get_states_and_transitions_text(),
            "\
> idx:0, head:Some(3), tail:Some(2)
  * link idx:3, prev:None, next:Some(0)
    trans idx:3, target state:4, [Char d]
  * link idx:0, prev:Some(3), next:Some(1)
    trans idx:0, target state:1, [Char a]
  * link idx:1, prev:Some(0), next:Some(2)
    trans idx:1, target state:2, [Char b]
  * link idx:2, prev:Some(1), next:None
    trans idx:2, target state:3, [Char c]
- idx:1, head:None, tail:None
- idx:2, head:None, tail:None
- idx:3, head:None, tail:None
- idx:4, head:None, tail:None"
        );
    }

    #[test]
    fn test_state_transition_insert() {
        let mut state_set = StateSet::new();
        let state_idx0 = state_set.new_state();
        let state_idx1 = state_set.new_state();
        let state_idx2 = state_set.new_state();
        let state_idx3 = state_set.new_state();
        let state_idx4 = state_set.new_state();

        state_set.insert_transition(
            state_idx0,
            state_idx1,
            Transition::Char(CharTransition::new('a', false)),
        );

        assert_str_eq!(
            state_set.get_states_and_transitions_text(),
            "\
> idx:0, head:Some(0), tail:Some(0)
  * link idx:0, prev:None, next:None
    trans idx:0, target state:1, [Char a]
- idx:1, head:None, tail:None
- idx:2, head:None, tail:None
- idx:3, head:None, tail:None
- idx:4, head:None, tail:None"
        );

        state_set.insert_transition(
            state_idx0,
            state_idx2,
            Transition::Char(CharTransition::new('b', false)),
        );

        state_set.insert_transition(
            state_idx0,
            state_idx3,
            Transition::Char(CharTransition::new('c', false)),
        );

        assert_str_eq!(
            state_set.get_states_and_transitions_text(),
            "\
> idx:0, head:Some(2), tail:Some(0)
  * link idx:2, prev:None, next:Some(1)
    trans idx:2, target state:3, [Char c]
  * link idx:1, prev:Some(2), next:Some(0)
    trans idx:1, target state:2, [Char b]
  * link idx:0, prev:Some(1), next:None
    trans idx:0, target state:1, [Char a]
- idx:1, head:None, tail:None
- idx:2, head:None, tail:None
- idx:3, head:None, tail:None
- idx:4, head:None, tail:None"
        );

        state_set.append_transition(
            state_idx0,
            state_idx4,
            Transition::Char(CharTransition::new('d', false)),
        );

        assert_str_eq!(
            state_set.get_states_and_transitions_text(),
            "\
> idx:0, head:Some(2), tail:Some(3)
  * link idx:2, prev:None, next:Some(1)
    trans idx:2, target state:3, [Char c]
  * link idx:1, prev:Some(2), next:Some(0)
    trans idx:1, target state:2, [Char b]
  * link idx:0, prev:Some(1), next:Some(3)
    trans idx:0, target state:1, [Char a]
  * link idx:3, prev:Some(0), next:None
    trans idx:3, target state:4, [Char d]
- idx:1, head:None, tail:None
- idx:2, head:None, tail:None
- idx:3, head:None, tail:None
- idx:4, head:None, tail:None"
        );
    }
}
