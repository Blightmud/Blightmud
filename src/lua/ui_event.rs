#[derive(Clone)]
pub enum UiEvent {
    StepLeft,
    StepRight,
    StepToStart,
    StepToEnd,
    StepWordLeft,
    StepWordRight,
    Remove,
    DeleteRight,
    DeleteToEnd,
    DeleteFromStart,
    PreviousCommand,
    NextCommand,
    ScrollUp,
    ScrollDown,
    ScrollBottom,
    Complete,
    Unknown(String),
}

impl From<&str> for UiEvent {
    fn from(s: &str) -> Self {
        match s {
            "step_left" => UiEvent::StepLeft,
            "step_right" => UiEvent::StepRight,
            "step_to_start" => UiEvent::StepToStart,
            "step_to_end" => UiEvent::StepToEnd,
            "step_word_left" => UiEvent::StepWordLeft,
            "step_word_right" => UiEvent::StepWordRight,
            "delete" => UiEvent::Remove,
            "delete_right" => UiEvent::DeleteRight,
            "delete_to_end" => UiEvent::DeleteToEnd,
            "delete_from_start" => UiEvent::DeleteFromStart,
            "previous_command" => UiEvent::PreviousCommand,
            "next_command" => UiEvent::NextCommand,
            "scroll_up" => UiEvent::ScrollUp,
            "scroll_down" => UiEvent::ScrollDown,
            "scroll_bottom" => UiEvent::ScrollBottom,
            "complete" => UiEvent::Complete,
            _ => UiEvent::Unknown(s.to_string()),
        }
    }
}
