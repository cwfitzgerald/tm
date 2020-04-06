use csv::Trim;
use itertools::{Itertools, MinMaxResult};
use serde::Deserialize;
use signal_hook::{register, unregister, SIGINT};
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::read_to_string;
use std::io::{BufRead, Cursor, Write};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};

// I'll get a bit handwave-y here and say that this is basically the print
// macro with an addition of a flush. Don't worry about the syntax, rust
// declarative macros can be a bit syntax heavy.
macro_rules! print_flush {
    ($($t:tt)*) => {
        let stdout = std::io::stdout();
        let mut locked = stdout.lock();
        write!(locked, $($t)*).expect("Unable to write to stdout");
        locked.flush().expect("Unable to flush stdout");
    };
}

// Atomic used to signal to stop execution
static INTERRUPTED: AtomicBool = AtomicBool::new(false);

// Signal handler function that sets the boolean.
fn signal_handler() {
    INTERRUPTED.store(true, Ordering::Release);
}

// Generic function that reads from stdin and converts to the type wanted.
// Rust has full type deduction within functions, so you can write:
// let value: PathBuf = read_input();
// and rust can just figure out that R should be PathBuf.
fn read_input<R, E>() -> R
where
    // Unlike c++ templates, rust generics require you to say what traits (think interfaces) each generic implements
    R: FromStr<Err = E>,
    E: Debug,
{
    let mut string = String::new();
    // read from stdin
    std::io::stdin()
        .lock()
        .read_line(&mut string)
        .expect("Unable to read line");
    string.pop(); // Remove newline
                  // convert to return value, panicking if conversion fails
    R::from_str(&string).expect("Must be able to convert from string")
}

// Takes a non-owning &str (pointer + size) and removes all comments
fn strip_comments(input: &str) -> String {
    // Output value
    let mut stripped = String::new();

    // Iterate over lines
    for line in input.lines() {
        // Trim whitespace at beginning and end of line
        let line = line.trim();

        if let Some(idx) = line.find("//") {
            // Found comment
            if idx != 0 {
                // Comment is in middle of the line, capture the non-comment portion, and trim off whitespace at beginning and end of that section
                stripped.push_str(line[0..idx].trim());
            } else {
                // Ignore line if it is empty
                continue;
            }
        } else {
            // No comment, just push it on as is
            stripped.push_str(line);
        }
        // Push on a newline
        stripped.push('\n');
    }
    stripped
}

// Rust has automatic deserialization using the library serde, we are using a csv frontend for serde
// by deriving the Deserialize trait, we autogenerate the deserializer.
#[derive(Debug, Deserialize)]
struct Line {
    state_number: String,
    tape_symbol: char,
    new_state: String,
    new_tape_symbol: char,
    direction: char,
}

// Convenience alias for the tape.
// The tape is just a map from index to the character in the index.
type Tape = HashMap<i64, char>;

// Convenience function to get the value from the tape
pub fn get_from_tape(input: &Tape, location: i64) -> char {
    if let Some(&c) = input.get(&location) {
        // We find a value at the location
        c
    } else {
        // Nothing is there, it must be blank
        'B'
    }
}

// Print out the Instantaneous Description of the turing machine
pub fn print_id(input: &Tape, state: &str, location: i64) {
    // The HashMap is potentially sparse, so find out the left and right most index.
    // This is O(n) over the size of the tape, but this is the damn turing machine simulator
    // do we really give a shit ;)
    let (minimum, maximum) = match input.keys().minmax() {
        MinMaxResult::NoElements => return,
        MinMaxResult::OneElement(&x) => (x, x),
        MinMaxResult::MinMax(&min, &max) => (min, max),
    };
    // The location is possibly looking beyond the bounds of the tape, so extend the min and max
    let minimum = minimum.min(location);
    let maximum = maximum.max(location);

    // Iterate over the range we care about
    for i in minimum..=maximum {
        let value = get_from_tape(input, i);
        if i == location {
            // We're at the cell that we currently live in, print fancy state viewer
            print!("|q{}=>{}|", state, value);
        } else {
            print!("{}", value);
        }
    }
    // Print a newline, we're done
    println!()
}

// HashMap of the transitions. put in the TransitionSource and get out the TransitionResult
type TransitionStore = HashMap<TransitionSource, TransitionResult>;

#[derive(Debug, PartialEq, Eq, Hash)]
struct TransitionSource {
    state_number: String,
    tape_symbol: char,
}

#[derive(Debug)]
struct TransitionResult {
    new_state: String,
    new_tape_symbol: char,
    direction: char,
}

// Main function
fn main() {
    // Allow the file path to be passed as an argument for ease of testing
    // pico-args is a crazy lightweight argument parsing library that I've recently
    // fallen in love with
    let mut arguments = pico_args::Arguments::from_env();
    let path: PathBuf = if let Ok(Some(p)) = arguments.free_from_str() {
        // We have an argument, return it
        p
    } else {
        // There is not an argument, ask for the filepath
        print_flush!("Please enter code file path: ");
        // Note the return value of this generic is deduced by the explicit type declaration in the let statement
        read_input()
    };

    // Read contents and strip comments
    let file_contents = read_to_string(path).expect("Path not found");
    let no_comments = strip_comments(&file_contents);

    // Create a CSV reader that uses spaces as a deliminator
    let mut csv_reader = csv::ReaderBuilder::new()
        .delimiter(b' ')
        .trim(Trim::All)
        .has_headers(false)
        .from_reader(Cursor::new(no_comments));

    let mut transitions = TransitionStore::new();

    for result in csv_reader.deserialize() {
        let line: Line = result.expect("Could not deserialize line");
        // Deserialize line by line, it automatically deserializes into the Line class
        // then insert it into the transition map
        transitions.insert(
            TransitionSource {
                state_number: line.state_number,
                tape_symbol: line.tape_symbol,
            },
            TransitionResult {
                new_state: line.new_state,
                new_tape_symbol: line.new_tape_symbol,
                direction: line.direction,
            },
        );
    }

    println!("Read all code.");

    // loop is a nice rust construct where you can loop forever
    loop {
        print_flush!("Ctrl-C to halt execution. Input word: (type quit to exit)\n > ");
        let input: String = read_input();
        if input.to_lowercase() == "quit" {
            println!("Cherrio!");
            break;
        }
        run_tm(&input, &transitions);
    }
}

fn run_tm(input: &str, transitions: &TransitionStore) {
    // Make a new tape, and insert everything from the input on it
    let mut tape = Tape::new();

    // The .enumerate iterator adapter turns an iterator that yields T into an iterator that yields
    // a tuple of the index and T. We then immediately destructure the tuple into its component parts
    for (idx, c) in input.chars().enumerate() {
        tape.insert(idx as i64, c);
    }

    // State we need to run the turing machine
    let mut tape_idx = 0_i64;
    let mut current_state = String::from("0");

    // We assume acceptance until we can prove otherwise
    let mut accepted = true;

    // Register a ctrl-c handler
    let signal =
        unsafe { register(SIGINT, signal_handler) }.expect("Unable to set signal on SIGINT");
    // We want to stop when we hit the "f" state
    while current_state != "f" {
        // We use swap so we simultaneously set the value to false, and get if it has been triggered
        if INTERRUPTED.swap(false, Ordering::AcqRel) {
            // Remove signal handler. This doesn't restore the old one, unfortunately, but that's fine
            // as you can always put "quit" as your input to bail out
            unregister(signal);
            println!("Interrupted!");
            accepted = false;
            break;
        }
        // Print the instantanious description
        print_id(&tape, &current_state, tape_idx);
        let source = TransitionSource {
            state_number: current_state.clone(),
            tape_symbol: get_from_tape(&tape, tape_idx),
        };
        if let Some(TransitionResult {
            new_state,
            new_tape_symbol,
            direction,
        }) = transitions.get(&source)
        {
            // There is a transition for the given input state and tape symbol
            current_state = new_state.clone();
            tape.insert(tape_idx, *new_tape_symbol);
            match direction {
                'R' => tape_idx += 1,
                'L' => tape_idx -= 1,
                _ => unreachable!(),
            }
        } else {
            // No transition out of this state and symbol! We must reject
            println!(
                "Rejected! No out transitions for input ({}, {})",
                source.state_number, source.tape_symbol
            );
            accepted = false;
            break;
        }
    }
    if accepted {
        // When we accept, we haven't printed the final ID with state of f, so let's do it here.
        print_id(&tape, &current_state, tape_idx);
        println!("Accepted!");
    }

    // This function returns, and we go around the loop again.
}
