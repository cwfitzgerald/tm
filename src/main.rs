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

macro_rules! print_flush {
    ($($t:tt)*) => {
        let stdout = std::io::stdout();
        let mut locked = stdout.lock();
        write!(locked, $($t)*).expect("Unable to write to stdout");
        locked.flush().expect("Unable to flush stdout");
    };
}

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

fn signal_handler() {
    INTERRUPTED.store(true, Ordering::Release);
}

fn read_input<R, E>() -> R
where
    R: FromStr<Err = E>,
    E: Debug,
{
    let mut string = String::new();
    std::io::stdin()
        .lock()
        .read_line(&mut string)
        .expect("Unable to read line");
    string.pop(); // Remove newline
    R::from_str(&string).expect("Must be able to convert from string")
}

fn discard_input() {
    let mut _garbo = String::new();
    std::io::stdin()
        .lock()
        .read_line(&mut _garbo)
        .expect("Unable to read line");
}

fn strip_comments(input: &str) -> String {
    let mut stripped = String::new();
    for line in input.lines() {
        let line = line.trim();
        // Found comment
        if let Some(idx) = line.find("//") {
            if idx != 0 {
                stripped.push_str(line[0..idx].trim());
            } else {
                // Ignore line if it is empty
                continue;
            }
        } else {
            stripped.push_str(line);
        }
        stripped.push('\n');
    }
    stripped
}

#[derive(Debug, Deserialize)]
struct Line {
    state_number: String,
    tape_symbol: char,
    new_state: String,
    new_tape_symbol: char,
    direction: char,
}

type Tape = HashMap<i64, char>;

pub fn get_from_tape(input: &Tape, location: i64) -> char {
    if let Some(&c) = input.get(&location) {
        c
    } else {
        'B'
    }
}

pub fn print_id(input: &Tape, state: &str, location: i64) {
    let (mut minimum, mut maximum) = match input.keys().minmax() {
        MinMaxResult::NoElements => return,
        MinMaxResult::OneElement(&x) => (x, x),
        MinMaxResult::MinMax(&min, &max) => (min, max),
    };
    minimum = minimum.min(location);
    maximum = maximum.max(location);

    for i in minimum..=maximum {
        let value = get_from_tape(input, i);
        if i == location {
            print!("|q{}=>{}|", state, value);
        } else {
            print!("{}", value);
        }
    }
    println!()
}

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

fn main() {
    let mut arguments = pico_args::Arguments::from_env();
    let path: PathBuf = if let Ok(Some(p)) = arguments.free_from_str() {
        p
    } else {
        print_flush!("Please enter code file path: ");
        read_input()
    };

    let file_contents = read_to_string(path).expect("Path not found");
    let no_comments = strip_comments(&file_contents);

    let mut csv_reader = csv::ReaderBuilder::new()
        .delimiter(b' ')
        .trim(Trim::All)
        .has_headers(false)
        .from_reader(Cursor::new(no_comments));

    let mut transitions = TransitionStore::new();

    for result in csv_reader.deserialize() {
        let line: Line = result.expect("Could not deserialize line");
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

    loop {
        print_flush!("Read code. Ctrl-C to halt execution. Input word: (type quit to exit)\n > ");
        let input: String = read_input();
        if input.to_lowercase() == "quit" {
            print!("Cherrio!");
            break;
        }
        run_tm(&input, &transitions);
    }
}

fn run_tm(input: &str, transitions: &TransitionStore) {
    let mut tape = Tape::new();

    for (idx, c) in input.chars().enumerate() {
        tape.insert(idx as i64, c);
    }

    let mut tape_idx = 0_i64;
    let mut current_state = String::from("0");

    let mut accepted = true;

    let signal =
        unsafe { register(SIGINT, signal_handler) }.expect("Unable to set signal on SIGINT");
    while current_state != "f" {
        if INTERRUPTED.swap(false, Ordering::AcqRel) {
            unregister(signal);
            println!("Interrupted!");
            accepted = false;
            break;
        }
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
            current_state = new_state.clone();
            tape.insert(tape_idx, *new_tape_symbol);
            match direction {
                'R' => tape_idx += 1,
                'L' => tape_idx -= 1,
                _ => unreachable!(),
            }
        } else {
            println!(
                "Rejected! No out transitions for input ({}, {})",
                source.state_number, source.tape_symbol
            );
            accepted = false;
            break;
        }
    }
    if accepted {
        print_id(&tape, &current_state, tape_idx);
        println!("Accepted!");
    }
}
