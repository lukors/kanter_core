# Kanter Core
This is a library for node based image editing created for use in [Kanter](https://github.com/lukors/kanter).

It's not meant to be used yet, but you can use it and it should be easy to see how it works by looking at the tests in `tests/integration_tests.rs`.

## Features
- Multithreaded, each node is executed in its own thread
- Nested graphs, a single node can contain an entire graph, so you can reuse graphs
- Basic nodes to add/divide etc.
- Every image channel is 32 bit float

## Progress
Currently I'm working on a GUI for this library called [Kanter](https://github.com/lukors/kanter).

Here are some planned tasks:

### General
- [x] Combine basic nodes like `Add` into a `Mix` node
- [ ] Implement rgba slots
- [ ] Combine input/output grayscale and rgba nodes
- [ ] Automatic conversion to and from grayscale and rgba slots
- [ ] Randomly generated test
- [ ] Noise node: https://github.com/jackmott/rust-simd-noise
- [ ] Consider variable support
- [ ] Command line interface

### Optimization
- [ ] Make benchmark tests
- [ ] Disregard nodes that do not lead to an output
- [ ] Do not resize inputs before processing, instead sample the un-resized image using the resizing filter
