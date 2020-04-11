// TODO:
// - Restore add node
// - Restore subtract node

// - Create a nested invert graph test
// - Create a system to save and load graphs

// - Restore multiply node

// - Implement same features as Channel Shuffle 1 & 2.
// - Implement CLI.
// - Make randomly generated test to try finding corner cases.
// - Make benchmark tests.
// - Optimize away the double-allocation when resizing an image before it's processed.
// - Make each node save the resized versions of their inputs,
//   and use them if they are still relevant.

pub mod dag;
pub mod error;
pub mod node;
pub mod node_data;
pub mod node_graph;
mod process;
mod shared;
