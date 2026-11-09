# Rust, offline, deterministic rendering

Videos are rendered offline: a fixed-timestep simulation drives frame-by-frame drawing piped to ffmpeg, with audio synthesized sample-accurately from the same simulation and muxed afterwards. Nothing is captured in real time, so a slow frame can never desync audio or drop frames, and the same Seed and generator version always produce the same Video. The generator is written in Rust (replacing an earlier Python/pygame prototype) because splitting Scenarios routinely reach thousands of balls and plain per-ball loops need to stay fast without contorting Rules into vectorised array code; the cost is slower iteration, which we offset by keeping Scenario tuning out of compiled code where possible.

## Considered Options

- **Python + numpy + skia/cairo**: fastest iteration and the heavy lifting is native anyway, but per-ball Rule logic at 2000+ balls forces vectorisation, and skia-python packaging is fiddly on NixOS.
- **Real-time pygame with screen capture**: simplest, but frame drops desync audio and pygame's drawing is aliased at 1080p.
