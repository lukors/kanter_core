# Kanter Core
This is a library for node based image editing created for use in [Kanter](https://github.com/lukors/kanter).

It's not meant to be used yet, but you can use it and it should be easy to see how it works by looking at the tests in `tests/integration_tests.rs`.

## Features
- Multithreaded, each node is executed in its own thread
- Nested graphs, a single node can contain an entire graph, so you can reuse graphs
- Basic nodes like to mix images

## Progress
Currently I'm working on a GUI for this library called [Kanter](https://github.com/lukors/kanter).

Here are some planned tasks:

### General
- [ ] Combine basic nodes like `Add` into a `Mix` node
- [ ] Combine grayscale and rgba variants of input/output nodes
- [ ] Make randomly generated test
- [ ] Make noise node: https://github.com/jackmott/rust-simd-noise
- [ ] Consider variable support
- [ ] Create a command line interface

### Optimization
- [ ] Make benchmark tests
- [ ] Disregard nodes that do not lead to an output
- [ ] Do not resize processing, instead sample the un-resized image
