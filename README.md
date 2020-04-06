# Connor Fitzgerald Turing Machine Homework

Hello! We established that the binary should work, I don't have access to the cs lab machines
right now (my creds expired a bit ago), but there are the same runtime dependencies as before,
so I expect no issues.

## Running

To run, just call the command `./connor-fitzgerald-tm` and it will ask for all the info it needs. For ease of testing
I also made it accept the path to the code as the first argument, so you may do that if you so
choose.

## Source Code

I have commented up the source code fairly well to try to help show what I am doing/explain features
of rust if I think they might be confusing. There are two files you should look at:
 - `src/main.rs` The source code for the whole program
 - `Cargo.toml` Dependencies for the program
 
This program does have dependencies, but none related to the actual act of running a TM, all related to
argument parsing, deserialization, utilities etc.

Of course, if you have questions, feel free to email me if something I explained was unclear.

## Building yourself

I really hope you don't get here, but just in case you want to use the makefile to build it yourself, here
are the instructions:

First grab rustup (instructions are here: https://rustup.rs).  
Once rustup is installed you should have `cargo` in your PATH. You _should_ just be able to run `make` and get
both a binary and a tar file.

