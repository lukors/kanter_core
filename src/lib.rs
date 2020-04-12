// NAME IDEAS
// Skyffel
// Materialistic, Materialism
// Kanter
// Materialbilder

// TODO:
// - Automatic grayscale to rgba conversion when exporting from a gray output
// - Update errors to include their relevant data
// - Create a system to save and load graphs
// - Create Normal map node

// - Restore multiply node
// - Add division node

// # CLI
// - Implement CLI.

// # GUI
// - Implement GUI

// # Tests
// - Make randomly generated test to try finding corner cases to help ensure there are no bugs
//   introduced when optimizing and building out the software.

// # Optimization
// - Make benchmark tests.
// - Do not calculate any nodes that do not lead to an output
// - Optimize away the double-allocation when resizing an image before it's processed.
// - Make each node save the resized versions of their inputs,
//   and use them if they are still relevant so they don't have to be resized every time that node
//   is re-processed. It will make it faster when one input to a node changes, but not the other.

pub mod dag;
pub mod error;
pub mod node;
pub mod node_data;
pub mod node_graph;
mod process;
mod shared;
