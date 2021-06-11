# Real-time Ray Tracing with `nannou` & `rust-gpu`

An attempt at a real-time implementation of "Ray Tracing in One Weekend" by
Peter Shirley. This was a personal experiment to learn more about rust-gpu,
ray tracing, and what limitations there are around ray tracing in real-time.

![screenshot](./nannou-ray-tracer-screenshot.png)

## Build Requirements

- Use `rustup` to install nightly Rust and include the `rust-dev` and
  `rustc-src` components. These are necessary for the rust-gpu spir-v builder to
  function.
- `nannou` is used to provide the event loop, wgpu graphics pipeline and a small
  GUI for tweaking performance. Take a look at the platform-specific
  requirements for nannou projects [here](https://guide.nannou.cc/getting_started/platform-specific_setup.html).

## rust-gpu

`rust-gpu` is still very rough around the edges but is already approaching a
dream-come-true.

Being able to share code between the CPU and GPU was especially helpful for
debugging. I had a few bugs (poor RNG, tracing an infinite loop, etc) that would
normally be a nightmare to debug in a shader, however in this case I could just
call the same function on the CPU, add some `dbg!`s and work out what the
issue was in no time.

I especially look forward to support for ADTs (enums with data) as this would
make looping over `Material`s and `Hittable` objects much easier (the current
workaround uses IDs and indices in a rather hacky manner). Support for trait
objects would be equally nice, though it looks like this would require changes
to the Vulkan specification itself.
