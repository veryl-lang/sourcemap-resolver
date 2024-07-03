# sourcemap-resolver

[![Actions Status](https://github.com/veryl-lang/sourcemap-resolver/workflows/Regression/badge.svg)](https://github.com/veryl-lang/sourcemap-resolver/actions)
[![Crates.io](https://img.shields.io/crates/v/sourcemap-resolver.svg)](https://crates.io/crates/sourcemap-resolver)

sourcemap-resolver is a CLI utility and library to resolve [Source Map Revision 3](https://sourcemaps.info/spec.html) which is adopted as [Veryl](https://github.com/veryl-lang/veryl)'s sourcemap.
Through the CLI command, a file location in log files can be resolved to the original location.

For example, if the line 28 in `test.sv` is generated from line 26 in `test.veryl`, the following annotation will be added by the CLI.

```
ERROR: [VRFC 10-2865] module 'test3' ignored due to previous errors [/path.../test.sv:28]
                                                                     ^-- /path.../test.veryl:26:18
```

## Installation

The prebuilt binary will be provided within [Veryl's release](https://github.com/veryl-lang/veryl/releases).
The following command can be used too.

```
cargo install sourcemap-resolver
```

## Usage

To annotate the existing logs, the following command can be used.

```
$ sourcemap-resolver test.log
```

Pipe can be used too to annotate on the fly.

```
$ [command] | sourcemap-resolver
```

## Supported tools

The following tools are supported:

* [Verilator](https://www.veripool.org/verilator/)
* [Synopsys VCS](https://www.synopsys.com/verification/simulation/vcs.html)
* [Synopsys DesignCompiler](https://www.synopsys.com/implementation-and-signoff/rtl-synthesis-test/design-compiler-nxt.html)
* [Synopsys Formality](https://www.synopsys.com/implementation-and-signoff/signoff/formality-equivalence-checking.html)
* [AMD Vivado Simulator](https://www.xilinx.com/products/design-tools/vivado/verification.html)

If you want to add another tool support, please open an [issue](https://github.com/veryl-lang/sourcemap-resolver/issues) and submit a log example.

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
