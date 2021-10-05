# Kanter Core
This is a library for node based image editing created for use in [Kanter](https://github.com/lukors/kanter).

It's not meant to be used yet, but you can use it and it should be easy to see how it works by looking at the tests in `tests/integration_tests.rs`.

## Features
- Multithreaded, each node is executed in its own thread
- Nested graphs, a single node can contain an entire graph, so you can reuse graphs
- Basic nodes to add/divide etc.
- Every image channel is 32 bit float

## Progress
The current goal is to implement all features required for a really smooth GUI experience.

## License
Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
