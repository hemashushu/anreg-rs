// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::transition::Transition;

/*
// image
// |-- state set --\
// |            |-- state node --\
// |            |                |- link head --> transition
// |            |                |- link node --> transition
// |            |                |- ...
// |            |                \- link tail --> transition
// |            |-- state node
// |            |-- ...
// |            |-- state node
// |
// |-- state set
 */

pub const MAIN_STATESET_INDEX: usize = 0;

// image
// |-- state set --\
// |            |-- state node --\
// |            |                |- transition node
// |            |                |- transition node
// |            |                |- ...
// |            |                \- transition node
// |            |-- state node
// |            |-- ...
// |            |-- state node
// |
// |-- state set

// the compile target
pub struct Image {
    pub statesets: Vec<StateSet>,
    captures: Vec<Capture>,
    number_of_counters: usize,
}

pub struct StateSet {
    states: Vec<StateNode>,
    pub start_node_index: usize,
    pub end_node_index: usize,

    // it is true when the expression starts with `^`, or encounters is_after and is_before
    pub fixed_start: bool,

    // it is true when the expression ends with `$`, or encounters is_after
    pub fixed_end: bool,

    /*
     * object references
     */
    // links: Vec<LinkNode>,
    // transitions: Vec<TransitionNode>,
}

// Every state node has one or more transitions.
struct StateNode {
    // link_head_index: Option<usize>,
    // link_tail_index: Option<usize>,
    transitions: Vec<TransitionNode>,
}

// struct LinkNode {
//     previous_index: Option<usize>, // the index of previous link node
//     next_index: Option<usize>,     // the index of next link node
//     transition_index: usize,       // the index of transition node
// }

struct TransitionNode {
    transition: Transition,    // the type of transition
    target_state_index: usize, // the index of next state
}

struct Capture {
    name: Option<String>,
}

impl Image {
    pub fn new() -> Self {
        Image {
            statesets: vec![],
            captures: vec![],
            number_of_counters: 0,
        }
    }

    pub fn new_stateset(&mut self) -> usize {
        let stateset = StateSet {
            states: vec![],
            start_node_index: 0,
            end_node_index: 0,
            fixed_start: false,
            fixed_end: false,
            // links: vec![],
            // transitions: vec![],
        };

        let idx = self.statesets.len();
        self.statesets.push(stateset);
        idx
    }

    pub fn new_counter(&mut self) -> usize {
        let counter_index = self.number_of_counters;
        self.number_of_counters += 1;
        counter_index
    }

    // pub fn get_stateset_ref_mut(&mut self, idx: usize) -> &mut StateSet {
    //     &mut self.statesets[idx]
    // }

    pub fn new_match(&mut self, name: Option<String>) -> usize {
        let idx = self.captures.len();
        self.captures.push(Capture { name });
        idx
    }

    pub fn get_capture_index_by_name(&self, name: &str) -> Option<usize> {
        self.captures.iter().position(|e| match &e.name {
            Some(n) => n == name,
            None => false,
        })
    }

    pub fn get_capture_names(&self) -> Vec<Option<&String>> {
        self.captures
            .iter()
            .map(|item| {
                if let Some(name) = &item.name {
                    Some(name)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn get_number_of_captures(&self) -> usize {
        self.captures.len()
    }

    pub fn get_number_of_counters(&self) -> usize {
        self.number_of_counters
    }

    // for debug
    /*
    pub fn get_image_text_verbose(&self) -> String {
        let mut lines = vec![];
        if self.statesets.len() == 1 {
            lines.push(self.statesets[0].get_stateset_text_verbose());
        } else {
            for (stateset_index, stateset) in self.statesets.iter().enumerate() {
                lines.push(format!("stateset: ${}", stateset_index));
                lines.push(stateset.get_stateset_text_verbose());
            }
        }

        // matches
        for (match_index, m) in self.captures.iter().enumerate() {
            let match_line = if let Some(n) = &m.name {
                format!("# group {{idx:{}}}, {}", match_index, n)
            } else {
                format!("# group {{idx:{}}}", match_index)
            };
            lines.push(match_line);
        }

        lines.join("\n")
    }
    */

    // for debug
    pub fn get_image_text(&self) -> String {
        let mut lines = vec![];
        if self.statesets.len() == 1 {
            lines.push(self.statesets[0].get_stateset_text());
        } else {
            for (stateset_index, stateset) in self.statesets.iter().enumerate() {
                lines.push(format!("stateset: ${}", stateset_index));
                lines.push(stateset.get_stateset_text());
            }
        }

        // matches
        for (match_index, m) in self.captures.iter().enumerate() {
            let match_line = if let Some(n) = &m.name {
                format!("# {{{}}}, {}", match_index, n)
            } else {
                format!("# {{{}}}", match_index)
            };
            lines.push(match_line);
        }

        lines.join("\n")
    }
}

impl StateSet {
    pub fn new_state(&mut self) -> usize {
        let state = StateNode {
            // link_head_index: None,
            // link_tail_index: None,
            transitions: vec![],
        };
        let idx = self.states.len();
        self.states.push(state);

        // return the index of the new state node
        idx
    }

    //     // private (helper) function
    //     fn add_transition_node(&mut self, transition_node: TransitionNode) -> usize {
    //         let idx = self.transitions.len();
    //         self.transitions.push(transition_node);
    //         idx
    //     }
    //
    //     // private (helper) function
    //     fn add_link_node(&mut self, link_node: LinkNode) -> usize {
    //         let idx = self.links.len();
    //         self.links.push(link_node);
    //         idx
    //     }

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

        self.states[source_state_index]
            .transitions
            .push(transition_node);

        //         let transition_index = self.add_transition_node(transition_node);
        //
        //         let is_transition_empty = self.states[source_state_index].is_transition_empty();
        //         if is_transition_empty {
        //             // add the first link list node
        //             let link_node = LinkNode {
        //                 previous_index: None,
        //                 next_index: None,
        //                 transition_index,
        //             };
        //             let link_node_index = self.add_link_node(link_node);
        //
        //             let source_state = &mut self.states[source_state_index];
        //             source_state.link_head_index = Some(link_node_index);
        //             source_state.link_tail_index = Some(link_node_index);
        //         } else {
        //             // append new transition node to the tail of link list
        //
        //             // create new link list node
        //             let last_link_node_index = self.states[source_state_index].link_tail_index.unwrap();
        //             let link_node = LinkNode {
        //                 previous_index: Some(last_link_node_index),
        //                 next_index: None,
        //                 transition_index,
        //             };
        //             let link_node_index = self.add_link_node(link_node);
        //
        //             // change the last node
        //             self.links[last_link_node_index].next_index = Some(link_node_index);
        //
        //             // change the tail node pointer of state
        //             self.states[source_state_index].link_tail_index = Some(link_node_index);
        //         }
    }

    //     pub fn insert_transition(
    //         &mut self,
    //         source_state_index: usize,
    //         target_state_index: usize,
    //         transition: Transition,
    //     ) {
    //         let transition_node = TransitionNode {
    //             transition,
    //             target_state_index,
    //         };
    //         let transition_index = self.add_transition_node(transition_node);
    //
    //         let is_transition_empty = self.states[source_state_index].is_transition_empty();
    //         if is_transition_empty {
    //             // add the first link list node
    //             let link_node = LinkNode {
    //                 previous_index: None,
    //                 next_index: None,
    //                 transition_index,
    //             };
    //             let link_node_index = self.add_link_node(link_node);
    //
    //             let source_state = &mut self.states[source_state_index];
    //             source_state.link_head_index = Some(link_node_index);
    //             source_state.link_tail_index = Some(link_node_index);
    //         } else {
    //             // instart new transition node to the head of link list
    //
    //             // create new link list node
    //             let first_link_node_index = self.states[source_state_index].link_head_index.unwrap();
    //             let link_node = LinkNode {
    //                 previous_index: None,
    //                 next_index: Some(first_link_node_index),
    //                 transition_index,
    //             };
    //             let link_node_index = self.add_link_node(link_node);
    //
    //             // change the first node
    //             self.links[first_link_node_index].previous_index = Some(link_node_index);
    //
    //             // change the head node pointer of state
    //             self.states[source_state_index].link_head_index = Some(link_node_index);
    //         }
    //     }

    /*
        // for debug
        pub fn get_stateset_text_verbose(&self) -> String {
            let mut lines = vec![];

            // states and transitions
            for (state_index, state_node) in self.states.iter().enumerate() {
                let prefix = if state_index == self.start_node_index {
                    '>'
                } else if state_index == self.end_node_index {
                    '<'
                } else {
                    '-'
                };

                // states

                let state_line = format!(
                    // "{} state <idx:{}>, head:{:?}, tail:{:?}",
                    // prefix, state_index, state_node.link_head_index, state_node.link_tail_index
                    "{} state <idx:{}>",
                    prefix, state_index
                );
                lines.push(state_line);

    //             // transitions (as well as the link list)
    //             let mut next_link_node_index = state_node.link_head_index;
    //             while let Some(link_node_index) = next_link_node_index {
    //                 let link_node = &self.links[link_node_index];
    //                 let link_line = format!(
    //                     "  * link ({}), prev:{:?}, next:{:?}",
    //                     link_node_index, link_node.previous_index, link_node.next_index
    //                 );
    //
    //                 let transition_node_index = link_node.transition_index;
    //                 let transition_node = &self.transitions[transition_node_index];
    //                 let transition_line = format!(
    //                     "    trans '{}', target state <idx:{}>, {}",
    //                     transition_node_index,
    //                     transition_node.target_state_index,
    //                     transition_node.transition
    //                 );
    //
    //                 lines.push(link_line);
    //                 lines.push(transition_line);
    //
    //                 // update next
    //                 next_link_node_index = link_node.next_index;
    //             }

                for transition_node in &state_node.transitions {
                    let transition_line = format!(
                        "  * target state <idx:{}>, {}",
                        transition_node.target_state_index,
                        transition_node.transition
                    );

                    lines.push(transition_line);
                }
            }

            lines.join("\n")
        }
     */

    // for debug
    pub fn get_stateset_text(&self) -> String {
        let mut lines = vec![];

        // states and transitions
        for (state_index, state_node) in self.states.iter().enumerate() {
            let prefix = if state_index == self.start_node_index {
                '>'
            } else if state_index == self.end_node_index {
                '<'
            } else {
                '-'
            };

            let state_line = format!("{} {}", prefix, state_index);
            lines.push(state_line);

            //             let mut next_link_node_index = state_node.link_head_index;
            //             while let Some(link_node_index) = next_link_node_index {
            //                 let link_node = &self.links[link_node_index];
            //                 let transition_node_index = link_node.transition_index;
            //                 let transition_node = &self.transitions[transition_node_index];
            //                 let transition_line = format!(
            //                     "  -> {}, {}",
            //                     transition_node.target_state_index, transition_node.transition
            //                 );
            //
            //                 lines.push(transition_line);
            //
            //                 // update next
            //                 next_link_node_index = link_node.next_index;
            //             }

            for transition_node in &state_node.transitions {
                let transition_line = format!(
                    "  -> {}, {}",
                    transition_node.target_state_index, transition_node.transition
                );
                lines.push(transition_line);
            }
        }

        lines.join("\n")
    }
}

/*
impl StateNode {
    // pub fn is_transition_empty(&self) -> bool {
    //     self.link_head_index.is_none()
    // }

    // pub fn get_first_transition_index(&self) -> Option<usize> {
    //     self.link_head_index
    // }
}
 */

#[cfg(test)]
mod tests {
    use pretty_assertions::{assert_eq, assert_str_eq};

    use crate::{
        image::Image,
        transition::{CharTransition, Transition},
    };

    #[test]
    fn test_image_new_stateset() {
        let mut image = Image::new();

        // create a stateset
        {
            let stateset_index = image.new_stateset();
            let stateset = &mut image.statesets[stateset_index];

            // create a state
            stateset.new_state();

            assert_str_eq!(
                stateset.get_stateset_text(),
                "\
> 0" //                 "\
                     // > state <idx:0>, head:None, tail:None"
            );

            // create other states
            let idx1 = stateset.new_state();
            stateset.new_state();
            let idx2 = stateset.new_state();

            stateset.start_node_index = idx1;
            stateset.end_node_index = idx2;

            assert_str_eq!(
                stateset.get_stateset_text(),
                "\
- 0
> 1
- 2
< 3" //                 "\
                     // - state <idx:0>, head:None, tail:None
                     // > state <idx:1>, head:None, tail:None
                     // - state <idx:2>, head:None, tail:None
                     // < state <idx:3>, head:None, tail:None"
            );
        }

        // create another stateset
        {
            let stateset_index = image.new_stateset();
            let stateset = &mut image.statesets[stateset_index];

            // create a state
            stateset.new_state();

            assert_str_eq!(
                image.get_image_text(),
                "\
stateset: $0
- 0
> 1
- 2
< 3
stateset: $1
> 0" //                 "\
                     // stateset: $0
                     // - state <idx:0>, head:None, tail:None
                     // > state <idx:1>, head:None, tail:None
                     // - state <idx:2>, head:None, tail:None
                     // < state <idx:3>, head:None, tail:None
                     // stateset: $1
                     // > state <idx:0>, head:None, tail:None"
            );
        }
    }

    #[test]
    fn test_image_new_match() {
        let mut image = Image::new();
        let stateset_index = image.new_stateset();
        let stateset = &mut image.statesets[stateset_index];

        stateset.new_state();
        stateset.new_state();

        image.new_match(None);
        image.new_match(Some("foo".to_owned()));
        image.new_match(None);

        assert_str_eq!(
            image.get_image_text(),
            "\
> 0
- 1
# {0}
# {1}, foo
# {2}" //             "\
                   // > state <idx:0>, head:None, tail:None
                   // - state <idx:1>, head:None, tail:None
                   // # group {idx:0}
                   // # group {idx:1}, foo
                   // # group {idx:2}"
        );

        assert_eq!(image.get_capture_index_by_name("foo"), Some(1));
        assert!(image.get_capture_index_by_name("bar").is_none());
    }

    #[test]
    fn test_image_new_counter() {
        let mut image = Image::new();
        assert_eq!(image.number_of_counters, 0);

        let index1 = image.new_counter();

        assert_eq!(index1, 0);
        assert_eq!(image.number_of_counters, 1);

        let index2 = image.new_counter();
        let index3 = image.new_counter();

        assert_eq!(index2, 1);
        assert_eq!(index3, 2);
    }

    #[test]
    fn test_stateset_append_transition() {
        let mut image = Image::new();
        let stateset_index = image.new_stateset();
        let stateset = &mut image.statesets[stateset_index];

        let state_idx0 = stateset.new_state();
        let state_idx1 = stateset.new_state();
        let state_idx2 = stateset.new_state();
        let state_idx3 = stateset.new_state();
        // let state_idx4 = stateset.new_state();

        stateset.append_transition(
            state_idx0,
            state_idx1,
            Transition::Char(CharTransition::new('a')),
        );

        assert_str_eq!(
            stateset.get_stateset_text(),
            "\
> 0
  -> 1, Char 'a'
- 1
- 2
- 3" //             "\
                 // > state <idx:0>, head:Some(0), tail:Some(0)
                 //   * link (0), prev:None, next:None
                 //     trans '0', target state <idx:1>, Char 'a'
                 // - state <idx:1>, head:None, tail:None
                 // - state <idx:2>, head:None, tail:None
                 // - state <idx:3>, head:None, tail:None
                 // - state <idx:4>, head:None, tail:None"
        );

        stateset.append_transition(
            state_idx0,
            state_idx2,
            Transition::Char(CharTransition::new('b')),
        );

        stateset.append_transition(
            state_idx0,
            state_idx3,
            Transition::Char(CharTransition::new('c')),
        );

        assert_str_eq!(
            stateset.get_stateset_text(),
            "\
> 0
  -> 1, Char 'a'
  -> 2, Char 'b'
  -> 3, Char 'c'
- 1
- 2
- 3" //             "\
                 // > state <idx:0>, head:Some(0), tail:Some(2)
                 //   * link (0), prev:None, next:Some(1)
                 //     trans '0', target state <idx:1>, Char 'a'
                 //   * link (1), prev:Some(0), next:Some(2)
                 //     trans '1', target state <idx:2>, Char 'b'
                 //   * link (2), prev:Some(1), next:None
                 //     trans '2', target state <idx:3>, Char 'c'
                 // - state <idx:1>, head:None, tail:None
                 // - state <idx:2>, head:None, tail:None
                 // - state <idx:3>, head:None, tail:None
                 // - state <idx:4>, head:None, tail:None"
        );

        /*
                stateset.insert_transition(
                    state_idx0,
                    state_idx4,
                    Transition::Char(CharTransition::new('d')),
                );

                assert_str_eq!(
                    stateset.get_stateset_text_verbose(),
                    "\
        > state <idx:0>, head:Some(3), tail:Some(2)
          * link (3), prev:None, next:Some(0)
            trans '3', target state <idx:4>, Char 'd'
          * link (0), prev:Some(3), next:Some(1)
            trans '0', target state <idx:1>, Char 'a'
          * link (1), prev:Some(0), next:Some(2)
            trans '1', target state <idx:2>, Char 'b'
          * link (2), prev:Some(1), next:None
            trans '2', target state <idx:3>, Char 'c'
        - state <idx:1>, head:None, tail:None
        - state <idx:2>, head:None, tail:None
        - state <idx:3>, head:None, tail:None
        - state <idx:4>, head:None, tail:None"
                );
                 */
    }

    /*
        #[test]
        fn test_stateset_transition_insert() {
            let mut image = Image::new();
            let stateset_index = image.new_stateset();
            let stateset = &mut image.statesets[stateset_index];

            let state_idx0 = stateset.new_state();
            let state_idx1 = stateset.new_state();
            let state_idx2 = stateset.new_state();
            let state_idx3 = stateset.new_state();
            let state_idx4 = stateset.new_state();

            stateset.insert_transition(
                state_idx0,
                state_idx1,
                Transition::Char(CharTransition::new('a')),
            );

            assert_str_eq!(
                stateset.get_stateset_text_verbose(),
                "\
    > state <idx:0>, head:Some(0), tail:Some(0)
      * link (0), prev:None, next:None
        trans '0', target state <idx:1>, Char 'a'
    - state <idx:1>, head:None, tail:None
    - state <idx:2>, head:None, tail:None
    - state <idx:3>, head:None, tail:None
    - state <idx:4>, head:None, tail:None"
            );

            stateset.insert_transition(
                state_idx0,
                state_idx2,
                Transition::Char(CharTransition::new('b')),
            );

            stateset.insert_transition(
                state_idx0,
                state_idx3,
                Transition::Char(CharTransition::new('c')),
            );

            assert_str_eq!(
                stateset.get_stateset_text_verbose(),
                "\
    > state <idx:0>, head:Some(2), tail:Some(0)
      * link (2), prev:None, next:Some(1)
        trans '2', target state <idx:3>, Char 'c'
      * link (1), prev:Some(2), next:Some(0)
        trans '1', target state <idx:2>, Char 'b'
      * link (0), prev:Some(1), next:None
        trans '0', target state <idx:1>, Char 'a'
    - state <idx:1>, head:None, tail:None
    - state <idx:2>, head:None, tail:None
    - state <idx:3>, head:None, tail:None
    - state <idx:4>, head:None, tail:None"
            );

            stateset.append_transition(
                state_idx0,
                state_idx4,
                Transition::Char(CharTransition::new('d')),
            );

            assert_str_eq!(
                stateset.get_stateset_text_verbose(),
                "\
    > state <idx:0>, head:Some(2), tail:Some(3)
      * link (2), prev:None, next:Some(1)
        trans '2', target state <idx:3>, Char 'c'
      * link (1), prev:Some(2), next:Some(0)
        trans '1', target state <idx:2>, Char 'b'
      * link (0), prev:Some(1), next:Some(3)
        trans '0', target state <idx:1>, Char 'a'
      * link (3), prev:Some(0), next:None
        trans '3', target state <idx:4>, Char 'd'
    - state <idx:1>, head:None, tail:None
    - state <idx:2>, head:None, tail:None
    - state <idx:3>, head:None, tail:None
    - state <idx:4>, head:None, tail:None"
            );
        }
         */
}
