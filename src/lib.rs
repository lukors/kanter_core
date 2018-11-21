// TODO:
// - Add support for all ResizePolicy variants
// - Add a resize node, though nodes are able to output a different size than their input.
// - Implement same features as Channel Shuffle 1 & 2.
// - Implement CLI.
// - Make randomly generated test to try finding corner cases.
// - Make benchmark tests.
// - Optimize away the double-allocation when resizing an image before it's processed.
// - Make each node save the resized versions of their inputs,
//   and use them if they are still relevant.

mod shared;
pub mod dag;
pub mod node;
