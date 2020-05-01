# Kanter Core
This is a library for node based image editing created for use in my program [Kanter](https://github.com/lukors/kanter).

It's not meant to be used yet, but you can use it and it should be easy to see how it works by looking at the tests in `tests/integration_tests.rs`.

## Features
- Multithreaded, each node is executed in its own thread
- Nested graphs, a single node can contain an entire graph, so you can reuse graphs, see the `nested_graph` test for an example
- Basic nodes like `add`, `subtract`, `multiply` and `divide`
- No concept of colors (except for image export), only operates on grayscale buffers

## Progress
Currently I'm working on a GUI for this library over in [Kanter](https://github.com/lukors/kanter), which will require some changes in this library.

Here are some planned tasks:

- [ ] Combine grayscale and rgba variants of input/output nodes
- [ ] Combine basic nodes like `Add` into a `Math` node
