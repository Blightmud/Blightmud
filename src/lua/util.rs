use crate::event::Event;
use std::{borrow::Cow, env, sync::mpsc::Sender};

pub fn output_stack_trace(writer: &Sender<Event>, error: &str) {
    writer
        .send(Event::Error("[Lua] Script error:".to_string()))
        .unwrap();
    for line in error.split('\n') {
        writer
            .send(Event::Error(format!("\t{}", line).to_string()))
            .unwrap();
    }
}

/// "~/blightmud" => "/home/yourname/blightmud"
pub fn expand_tilde(path: &str) -> Cow<str> {
    if path.starts_with("~") {
        Cow::from(env::var("HOME").expect("$HOME must be set") + &path[1..])
    } else {
        Cow::from(path)
    }
}

#[cfg(test)]
mod util_tests {
    use super::*;

    #[test]
    fn homedir_expansion() {
        env::set_var("HOME", "/home/what");
        assert_eq!("/home/what/blightmud", expand_tilde("~/blightmud"));

        env::set_var("HOME", "/Users/cindi");
        assert_eq!("/Users/cindi/blightmud/data", expand_tilde("~/blightmud/data"));

        assert_eq!("/leave/it/alone", expand_tilde("/leave/it/alone"));
        assert_eq!("/leave/~/alone", expand_tilde("/leave/~/alone"));
    }
}
