// TODO:
// - Add resize node

// - Restore add node
// - Restore subtract node

// - Create an invert graph to nest

// - Restore multiply node


// - Add a resize node, though nodes are able to output a different size than their input, but only
//   with a default filter (Bilinear?) and ResizePolicy (LargestAxes) then.
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
