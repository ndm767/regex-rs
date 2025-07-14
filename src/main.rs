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

    return dot_cmd;
}

fn main() {
    let toks = lex(read!("{}\n"));
    println!("toks: {:?}", toks);

    let nfa = parse(toks);
    println!("nfa: {:?}", nfa);

    let mut dfa = Dfa::from_nfa(nfa.clone());

    let mut dfa_non_min_child = show_dot(dfa.to_dot(String::from("Unminimized DFA")));

    dfa.minimize();

    let mut nfa_child = show_dot(nfa.to_dot());
    let mut dfa_child = show_dot(dfa.to_dot(String::from("Minimized DFA")));

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

    nfa_child.kill().expect("Failed to kill nfa child");
    dfa_child.kill().expect("Failed to kill dfa child");
    dfa_non_min_child
        .kill()
        .expect("Failed to kill dfa non-minimized child");
}
