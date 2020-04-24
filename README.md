# kanter_core
This is a library for node based image editing created for use in my program [Kanter](https://github.com/lukors/kanter).

## Features
- Multithreaded, each node is executed in its own thread
- Nested graphs, a single node can contain an entire graph, so you can reuse graphs, see the `nested_graph` test for an example
- Basic nodes like `add`, `subtract`, `multiply` and `divide`
- No concept of colors (except for image export), only operates on grayscale buffers

## Current state
It is in working condition and it should be easy to see how it works by looking at the tests in `tests/integration_tests.rs`.

However it is not meant to be used yet, I'm not even updating the version number. It's more of a personal project currently so there will still be big changes.

## Plans
Currently I'm working on a GUI for this library over in [Kanter](https://github.com/lukors/kanter), which is likely to require some changes in this library.

Once that is working I'm going to think about what the next steps should be.
