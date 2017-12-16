# system [![Build Status](https://api.travis-ci.org/GuillaumeGomez/sysinfo.png?branch=master)](https://travis-ci.org/GuillaumeGomez/sysinfo)

A system handler to interact with processes.

Support the following platforms:

 * Linux
 * Raspberry
 * Mac OSX

## C interface

It's possible to use this crate directly from C. Take a look at the `Makefile` and at the `examples/src/simple.c` files.

To build the C example, just run:

```bash
> make
> ./simple
# If needed:
> LD_LIBRARY_PATH=target/release/ ./simple
```
