mod dfa;
mod nfa;
mod parse;
mod transition_table;

use std::io::Write;
use std::process::{Child, Command, Stdio};

use colored::Colorize;
use text_io::read;

use dfa::Dfa;
use parse::{lex, parse};

fn show_dot(dot_file: String) -> Child {
    let mut dot_cmd = Command::new("dot")
        .args(["-T", "x11"])
        .stdin(Stdio::piped())
        .spawn()
        .expect("Failed to spawn dot process");

    let mut stdin = dot_cmd.stdin.take().expect("Failed to open stdin");
    stdin
        .write_all(dot_file.as_bytes())
        .expect("Failed to write to stdin");

    dot_cmd
}

fn write_dot(filename: &str, dot_file: String) {
    #[allow(clippy::zombie_processes)]
    let mut dot_cmd = Command::new("dot")
        .args(["-T", "png", "-o", filename])
        .stdin(Stdio::piped())
        .spawn()
        .expect("Failed to spawn dot process");

    let mut stdin = dot_cmd.stdin.take().expect("Failed to open stdin");
    stdin
        .write_all(dot_file.as_bytes())
        .expect("Failed to write to stdin");
}

fn main() {
    let args = std::env::args();
    let should_write = args
        .collect::<Vec<_>>()
        .contains(&String::from("--output-png"));

    // parse regex
    let toks = lex(read!("{}\n"));

    let nfa = parse(toks);

    let mut dfa = Dfa::from_nfa(nfa.clone());
    let mut dfa_non_min_child = show_dot(dfa.to_dot("Unminimized DFA"));
    if should_write {
        write_dot("./dfa_nonmin.png", dfa.to_dot("Unminimized DFA"));
    }

    dfa.minimize();

    let mut nfa_child = show_dot(nfa.to_dot());
    if should_write {
        write_dot("./nfa.png", nfa.to_dot());
    }

    let mut dfa_child = show_dot(dfa.to_dot("DFA minimized with Hopcroft's algorithm"));
    if should_write {
        write_dot(
            "./dfa_min.png",
            dfa.to_dot("DFA minimized with Hopcroft's algorithm"),
        );
    }

    // TUI
    print!("{}", "> ".green().bold());
    let mut input: String = read!("{}\n");

    while input != "exit" {
        let sim = dfa.simulate(input);
        match sim {
            Ok(_) => println!("{}{:?}", "Output: ".green(), sim),
            Err(_) => println!("{}{:?}", "Output: ".red(), sim),
        }
        print!("{}", "> ".green().bold());
        input = read!("{}\n");
    }

    // subprocess cleanup
    nfa_child.kill().expect("Failed to kill nfa child");
    dfa_child.kill().expect("Failed to kill dfa child");
    dfa_non_min_child
        .kill()
        .expect("Failed to kill dfa non-minimized child");

    nfa_child.wait().expect("nfa_child command wasn't running");
    dfa_child.wait().expect("dfa_child command wasn't running");
    dfa_non_min_child
        .wait()
        .expect("dfa_non_min_child command wasn't running");
}
